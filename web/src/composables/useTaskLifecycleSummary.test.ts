import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useTaskLifecycleSummary } from './useTaskLifecycleSummary'
import { useTaskStore } from '../stores/taskStore'

describe('useTaskLifecycleSummary', () => {
  beforeEach(() => {
    const pinia = createPinia()
    setActivePinia(pinia)
  })

  it('derives active-task posture and progress from the shared task store', () => {
    const taskStore = useTaskStore()
    taskStore.$patch({
      activeById: {
        task_running: {
          id: 'task_running',
          command: 'run_double_ma_optimization',
          label: 'Optimization',
          status: 'running',
          startedAt: 1_700_000_000_000,
          message: 'Evaluate 42/100',
        },
      },
      activeOrder: ['task_running'],
      centerStatus: 'running',
      overallProgress: {
        pct: 42,
        label: 'Optimize 42/100',
        source: 'double_ma_progress',
        atUnixMs: 1_700_000_000_000,
      },
      history: [
        {
          id: 'task_done',
          command: 'run_double_ma_validation',
          label: 'Validation',
          status: 'success',
          startedAt: 1_700_000_000_100,
          endedAt: 1_700_000_000_900,
          message: 'Validation complete.',
        },
      ],
      toasts: [
        {
          id: 'toast_done',
          atUnixMs: 1_700_000_001_000,
          source: 'run_double_ma_validation',
          message: 'Validation complete.',
          level: 'success',
        },
      ],
    })

    const summary = useTaskLifecycleSummary()

    expect(summary.activeTaskCount.value).toBe(1)
    expect(summary.activeTasks.value.map((task) => task.id)).toEqual(['task_running'])
    expect(summary.centerStatusLabel.value).toBe('Running')
    expect(summary.overallProgressLabel.value).toContain('Optimize 42/100')
    expect(summary.latestTask.value?.id).toBe('task_done')
    expect(summary.latestTaskOutcome.value).toBe('Validation · Completed')
    expect(summary.taskHeadline.value).toBe('Evaluate 42/100')
  })

  it('falls back from active work to latest toast and latest task outcome', () => {
    const taskStore = useTaskStore()
    taskStore.$patch({
      activeById: {},
      activeOrder: [],
      centerStatus: 'failed',
      overallProgress: null,
      history: [
        {
          id: 'task_failed',
          command: 'run_double_ma_validation',
          label: 'Validation',
          status: 'failed',
          startedAt: 1_700_000_000_000,
          endedAt: 1_700_000_001_000,
          error: 'Validation failed.',
        },
      ],
      toasts: [
        {
          id: 'toast_failed',
          atUnixMs: 1_700_000_001_000,
          source: 'run_double_ma_validation',
          message: 'Validation failed.',
          level: 'error',
        },
      ],
    })

    const summary = useTaskLifecycleSummary()

    expect(summary.activeTaskCount.value).toBe(0)
    expect(summary.centerStatusLabel.value).toBe('Failed')
    expect(summary.overallProgressLabel.value).toBe('No active progress')
    expect(summary.latestTaskOutcome.value).toBe('Validation · Failed')
    expect(summary.taskHeadline.value).toBe('Validation failed.')
  })

  it('reports an idle headline when no active or historical task state exists', () => {
    const summary = useTaskLifecycleSummary()

    expect(summary.activeTaskCount.value).toBe(0)
    expect(summary.centerStatusLabel.value).toBe('Idle')
    expect(summary.latestTask.value).toBeNull()
    expect(summary.latestTaskOutcome.value).toBe('No completed or cancelled tasks yet.')
    expect(summary.taskHeadline.value).toBe('No active commands.')
  })
})
