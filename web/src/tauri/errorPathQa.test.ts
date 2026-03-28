import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import { installTauriInstrumentation } from './instrumentation'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useTaskStore } from '../stores/taskStore'

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

function setupRuntime(invokeImpl: (command: string) => Promise<unknown>): void {
  ;(globalThis as Record<string, unknown>).window = {
    localStorage: new MemoryStorage(),
    __TAURI__: {
      core: {
        invoke: vi.fn(async (command: string) => invokeImpl(command)),
      },
      event: {
        listen: vi.fn(async () => () => {}),
      },
    },
    __VTA_INVOKE_PATCHED__: false,
    __VTA_INVOKE_PATCH_BLOCKED__: false,
    __VTA_EVENT_UNSUBSCRIBERS__: [],
  }
}

function setupRuntimeReadonlyInvoke(invokeImpl: (command: string) => Promise<unknown>): void {
  const core: { invoke?: (command: string) => Promise<unknown> } = {}
  Object.defineProperty(core, 'invoke', {
    configurable: false,
    enumerable: true,
    writable: false,
    value: vi.fn(async (command: string) => invokeImpl(command)),
  })
  ;(globalThis as Record<string, unknown>).window = {
    localStorage: new MemoryStorage(),
    __TAURI__: {
      core,
      event: {
        listen: vi.fn(async () => () => {}),
      },
    },
    __VTA_INVOKE_PATCHED__: false,
    __VTA_INVOKE_PATCH_BLOCKED__: false,
    __VTA_EVENT_UNSUBSCRIBERS__: [],
  }
}

async function runInstrumented(command: string): Promise<void> {
  installTauriInstrumentation()
  const invoke = window.__TAURI__?.core?.invoke
  if (!invoke) throw new Error('Expected patched Tauri invoke function.')
  await invoke(command, undefined)
}

describe('error-path QA through instrumented invoke', () => {
  let pinia: Pinia
  let warnSpy: ReturnType<typeof vi.spyOn>
  let errorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
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
    delete (globalThis as Record<string, unknown>).window
  })

  it('classifies invalid-input failures as failed with surfaced error', async () => {
    setupRuntime(async () => {
      throw new Error('HalfTrend amplitude must be in [1, 65535].')
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
    setupRuntime(async () => {
      throw new Error('API key + secret are required unless dry-run is enabled.')
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
    setupRuntime(async () => {
      throw new Error('Validation cancelled by user.')
    })

    await expect(runInstrumented('run_double_ma_validation')).rejects.toThrow('Validation cancelled by user.')

    const store = useTaskStore(pinia)
    expect(store.history[0]?.status).toBe('cancelled')
    expect(store.centerStatus).toBe('cancelled')
    expect(store.toasts[0]?.level).toBe('warn')
  })

  it('surfaces backend failures as failed with error toast', async () => {
    setupRuntime(async () => {
      throw new Error('Backend failure: internal optimizer panic.')
    })

    await expect(runInstrumented('run_meta_selection')).rejects.toThrow(
      'Backend failure: internal optimizer panic.',
    )

    const store = useTaskStore(pinia)
    expect(store.history[0]?.status).toBe('failed')
    expect(store.history[0]?.error).toContain('optimizer panic')
    expect(store.toasts[0]?.level).toBe('error')
  })

  it('does not crash when tauri invoke is readonly', async () => {
    setupRuntimeReadonlyInvoke(async () => 'ok')
    expect(() => installTauriInstrumentation()).not.toThrow()
    expect(window.__VTA_INVOKE_PATCHED__).toBe(false)
    expect(window.__VTA_INVOKE_PATCH_BLOCKED__).toBe(true)
    const invoke = window.__TAURI__?.core?.invoke
    expect(invoke).toBeTypeOf('function')
    await expect(invoke?.('run_meta_selection')).resolves.toBe('ok')
  })
})
