import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import {
  beginTrackedCommand,
  completeTrackedCommand,
  failTrackedCommand,
  markTaskCancelling,
  taskPostureSnapshot,
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

describe('task-store lifecycle smoke', () => {
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

  it('records a success lifecycle and updates posture counts', () => {
    const taskId = beginTrackedCommand('run_double_ma_optimization')
    completeTrackedCommand(taskId)

    const store = useTaskStore(pinia)
    expect(store.history[0]).toMatchObject({
      command: 'run_double_ma_optimization',
      status: 'success',
    })
    expect(store.centerStatus).toBe('success')

    const posture = taskPostureSnapshot()
    expect(posture.counts.completed).toBe(1)
    expect(posture.counts.active).toBe(0)
  })

  it('classifies cancellation-like failures distinctly from hard failures', () => {
    const cancelledId = beginTrackedCommand('run_double_ma_validation')
    failTrackedCommand(cancelledId, new Error('Validation cancelled by user.'))

    const failedId = beginTrackedCommand('run_meta_selection')
    failTrackedCommand(failedId, new Error('backend panic'))

    const store = useTaskStore(pinia)
    expect(store.history[0]).toMatchObject({
      command: 'run_meta_selection',
      status: 'failed',
    })
    expect(store.history[1]).toMatchObject({
      command: 'run_double_ma_validation',
      status: 'cancelled',
    })

    const posture = taskPostureSnapshot()
    expect(posture.counts.failed).toBe(1)
    expect(posture.counts.cancelled).toBe(1)
  })

  it('keeps posture counts coherent across queued, cancelling, and terminal tasks', async () => {
    const queuedId = beginTrackedCommand('run_double_ma_optimization', { queued: true })
    const cancellingId = beginTrackedCommand('run_meta_selection')
    markTaskCancelling(cancellingId)
    const completedId = beginTrackedCommand('run_ml_signal_validation')
    completeTrackedCommand(completedId)

    const immediate = taskPostureSnapshot()
    expect(immediate.queued.map((task) => task.id)).toContain(queuedId)
    expect(immediate.cancelling.map((task) => task.id)).toContain(cancellingId)
    expect(immediate.counts.completed).toBe(1)
    expect(immediate.counts.active).toBe(2)

    await Promise.resolve()

    const settled = taskPostureSnapshot()
    expect(settled.running.map((task) => task.id)).toContain(queuedId)
    expect(settled.counts.running).toBeGreaterThanOrEqual(1)
    expect(settled.counts.cancelling).toBe(1)
    expect(settled.counts.completed).toBe(1)
  })
})
