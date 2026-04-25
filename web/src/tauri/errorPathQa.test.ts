import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useTaskStore } from '../stores/taskStore'
import { installMockIpc, resetTauriApiMocks } from '../test/tauriMocks'
import { invokeCommand } from './invoke'
import {
  installTauriInstrumentation,
  resetTauriInstrumentationForTests,
} from './instrumentation'

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

async function runInstrumented(command: Parameters<typeof invokeCommand>[0]): Promise<void> {
  installTauriInstrumentation()
  await invokeCommand(command, undefined)
}

describe('error-path QA through instrumented invoke', () => {
  let pinia: Pinia
  let warnSpy: ReturnType<typeof vi.spyOn>
  let errorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    resetTauriInstrumentationForTests()
    resetTauriApiMocks()
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      value: new MemoryStorage(),
    })
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    const store = useTaskStore(pinia)
    store.$reset()
  })

  afterEach(() => {
    warnSpy.mockRestore()
    errorSpy.mockRestore()
    resetTauriInstrumentationForTests()
    resetTauriApiMocks()
  })

  it('classifies invalid-input failures as failed with surfaced error', async () => {
    installMockIpc({
      run_composite_cpu_optimization: async () => {
        throw new Error('HalfTrend amplitude must be in [1, 65535].')
      },
    })

    await expect(runInstrumented('run_composite_cpu_optimization')).rejects.toThrow(
      'HalfTrend amplitude must be in [1, 65535].',
    )

    const store = useTaskStore(pinia)
    expect(store.history[0]?.status).toBe('failed')
    expect(store.history[0]?.error).toContain('HalfTrend amplitude')
    expect(store.centerStatus).toBe('failed')
  })

  it('surfaces missing-API-key failures as failed', async () => {
    installMockIpc({
      start_live_execution_runner: async () => {
        throw new Error('API key + secret are required unless dry-run is enabled.')
      },
    })

    await expect(runInstrumented('start_live_execution_runner')).rejects.toThrow(
      'API key + secret are required unless dry-run is enabled.',
    )

    const store = useTaskStore(pinia)
    expect(store.history[0]?.status).toBe('failed')
    expect(store.history[0]?.error).toContain('API key + secret')
    expect(store.events[0]?.message).toContain('API key + secret')
  })

  it('classifies cancelled runs distinctly from failures', async () => {
    installMockIpc({
      run_double_ma_validation: async () => {
        throw new Error('Validation cancelled by user.')
      },
    })

    await expect(runInstrumented('run_double_ma_validation')).rejects.toThrow(
      'Validation cancelled by user.',
    )

    const store = useTaskStore(pinia)
    expect(store.history[0]?.status).toBe('cancelled')
    expect(store.centerStatus).toBe('cancelled')
    expect(store.toasts[0]?.level).toBe('warn')
  })

  it('surfaces backend failures as failed with error toast', async () => {
    installMockIpc({
      run_meta_selection: async () => {
        throw new Error('Backend failure: internal optimizer panic.')
      },
    })

    await expect(runInstrumented('run_meta_selection')).rejects.toThrow(
      'Backend failure: internal optimizer panic.',
    )

    const store = useTaskStore(pinia)
    expect(store.history[0]?.status).toBe('failed')
    expect(store.history[0]?.error).toContain('optimizer panic')
    expect(store.toasts[0]?.level).toBe('error')
  })

  it('is idempotent when installed more than once', async () => {
    installMockIpc({
      run_meta_selection: async () => ({ ok: true }),
    })

    installTauriInstrumentation()
    installTauriInstrumentation()

    await expect(invokeCommand('run_meta_selection')).resolves.toEqual({ ok: true })

    const store = useTaskStore(pinia)
    expect(store.history).toHaveLength(1)
    expect(store.history[0]?.command).toBe('run_meta_selection')
  })
})
