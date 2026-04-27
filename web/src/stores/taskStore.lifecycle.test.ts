import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia, type Pinia } from 'pinia'
import {
  activeTasksCount,
  beginTrackedCommand,
  cancelCommandFor,
  completeTrackedCommand,
  dismissToast,
  failTrackedCommand,
  hydrateTaskCenterForTests,
  markTasksCancellingForCommand,
  markTaskCancelling,
  persistTaskCenterForTests,
  resetTaskStoreForTests,
  shouldGuardDesktopCloseForTask,
  shouldTrackCommandLifecycle,
  taskPostureSnapshot,
  useTaskStore,
} from './taskStore'
import { bindRuntimePinia } from './runtimePinia'
import { TASK_TIMELINE_STORAGE_KEY } from '../utils/storage'

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

function makeCompletedTask(id: string, status: 'success' | 'failed' | 'cancelled'): {
  id: string
  command: string
  label: string
  status: 'success' | 'failed' | 'cancelled'
  startedAt: number
  endedAt: number
  message: string
  error?: string
} {
  return {
    id,
    command: 'run_double_ma_optimization',
    label: 'Double Ma Optimization',
    status,
    startedAt: 1_700_000_000_000,
    endedAt: 1_700_000_001_000,
    message: `${status} task`,
    ...(status === 'failed' ? { error: 'backend panic' } : {}),
  }
}

function readPersistedTaskCenter(): Record<string, unknown> | null {
  const raw = globalThis.window?.localStorage?.getItem(TASK_TIMELINE_STORAGE_KEY) ?? null
  return raw ? (JSON.parse(raw) as Record<string, unknown>) : null
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
    resetTaskStoreForTests()
    const store = useTaskStore(pinia)
    store.$reset()
  })

  afterEach(() => {
    resetTaskStoreForTests()
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

  it('hydrates persisted task-center payloads including bounded toasts', () => {
    const now = Date.now()
    globalThis.window.localStorage.setItem(
      TASK_TIMELINE_STORAGE_KEY,
      JSON.stringify({
        version: 2,
        history: Array.from({ length: 82 }, (_, index) =>
          makeCompletedTask(`history_${index + 1}`, index === 0 ? 'failed' : 'success'),
        ),
        events: Array.from({ length: 162 }, (_, index) => ({
          id: `event_${index + 1}`,
          atUnixMs: 1_700_000_000_000 + index,
          source: 'run_double_ma_optimization',
          message: `Optimization event ${index + 1}.`,
          level: 'info',
        })),
        timeline: Array.from({ length: 302 }, (_, index) => ({
          id: `timeline_${index + 1}`,
          atUnixMs: 1_700_000_000_000 + index,
          source: 'run_double_ma_optimization',
          phase: index === 0 ? 'failed' : 'success',
          message: `Optimization timeline ${index + 1}.`,
          level: index === 0 ? 'error' : 'success',
        })),
        toasts: Array.from({ length: 10 }, (_, index) => ({
          id: `toast_${index + 1}`,
          atUnixMs: now + index,
          source: 'run_double_ma_optimization',
          message: `Toast ${index + 1}`,
          level: 'success',
        })),
      }),
    )

    resetTaskStoreForTests()
    const store = useTaskStore(pinia)
    hydrateTaskCenterForTests()

    expect(activeTasksCount()).toBe(0)
    expect(store.history).toHaveLength(80)
    expect(store.events).toHaveLength(160)
    expect(store.timeline).toHaveLength(300)
    expect(store.toasts).toHaveLength(8)
    expect(store.centerStatus).toBe('failed')
    expect(store.history[0]?.id).toBe('history_1')
    expect(store.events[0]?.id).toBe('event_1')
    expect(store.timeline[0]?.id).toBe('timeline_1')
    expect(store.toasts.map((toast) => toast.id)).toEqual([
      'toast_1',
      'toast_2',
      'toast_3',
      'toast_4',
      'toast_5',
      'toast_6',
      'toast_7',
      'toast_8',
    ])
  })

  it('remains backward compatible with persisted v1 payloads that did not include toasts', () => {
    globalThis.window.localStorage.setItem(
      TASK_TIMELINE_STORAGE_KEY,
      JSON.stringify({
        version: 1,
        history: [makeCompletedTask('legacy_history', 'cancelled')],
        events: [],
        timeline: [],
      }),
    )

    resetTaskStoreForTests()
    const store = useTaskStore(pinia)
    hydrateTaskCenterForTests()

    expect(activeTasksCount()).toBe(0)
    expect(store.history[0]?.id).toBe('legacy_history')
    expect(store.toasts).toEqual([])
  })

  it('persists toast additions and dismissals in the bounded task-center payload', async () => {
    vi.useFakeTimers()
    try {
      for (let index = 0; index < 9; index += 1) {
        const taskId = beginTrackedCommand('run_double_ma_optimization')
        completeTrackedCommand(taskId)
      }

      await vi.advanceTimersByTimeAsync(1000)

      const store = useTaskStore(pinia)
      persistTaskCenterForTests()
      const latestToastId = store.toasts[0]?.id ?? ''
      const persistedBeforeDismiss = readPersistedTaskCenter()
      expect(store.toasts).toHaveLength(8)
      expect(persistedBeforeDismiss?.version).toBe(2)
      expect((persistedBeforeDismiss?.toasts as unknown[] | undefined)?.length).toBe(8)

      dismissToast(latestToastId)
      await vi.advanceTimersByTimeAsync(1000)

      const persistedAfterDismiss = readPersistedTaskCenter()
      expect(store.toasts).toHaveLength(7)
      expect(store.toasts.some((toast) => toast.id === latestToastId)).toBe(false)
      expect((persistedAfterDismiss?.toasts as Array<{ id: string }> | undefined)?.some((toast) => toast.id === latestToastId)).toBe(false)
    } finally {
      vi.useRealTimers()
    }
  })

  it('auto-dismisses success toasts after a short timeout', async () => {
    vi.useFakeTimers()
    try {
      const taskId = beginTrackedCommand('run_double_ma_optimization')
      completeTrackedCommand(taskId)

      const store = useTaskStore(pinia)
      expect(store.toasts).toHaveLength(1)

      await vi.advanceTimersByTimeAsync(4600)

      expect(store.toasts).toHaveLength(0)
    } finally {
      vi.useRealTimers()
    }
  })

  it('does not surface a success toast for candidate-detail micro-operations', () => {
    const taskId = beginTrackedCommand('compute_double_ma_drilldown')
    completeTrackedCommand(taskId)

    const store = useTaskStore(pinia)
    expect(store.history[0]).toMatchObject({
      command: 'compute_double_ma_drilldown',
      status: 'success',
    })
    expect(store.toasts).toHaveLength(0)
  })

  it('keeps analysis commands tracked but non-cancelable', () => {
    const taskId = beginTrackedCommand('compute_double_ma_drilldown')

    expect(shouldTrackCommandLifecycle('compute_double_ma_drilldown')).toBe(true)
    expect(cancelCommandFor('compute_double_ma_drilldown')).toBe(null)
    expect(
      shouldGuardDesktopCloseForTask({
        command: 'compute_double_ma_drilldown',
        status: 'running',
      }),
    ).toBe(true)

    const touchedTaskIds = markTasksCancellingForCommand('cancel_current_run')
    const store = useTaskStore(pinia)
    expect(touchedTaskIds).not.toContain(taskId)
    expect(store.activeById[taskId]?.status).toBe('running')
  })
})
