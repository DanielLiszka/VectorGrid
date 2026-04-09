import { describe, expect, it } from 'vitest'
import {
  describeLatestTaskOutcome,
  describeTaskEvent,
  describeTaskToast,
  describeTimelineEntry,
  describeRuntimePhase,
  describeTaskActivity,
  describeTrackedTask,
  formatTaskPhaseLabel,
  formatTaskSourceLabel,
  formatOverallProgressLabel,
  taskLevelLabel,
  taskLevelTone,
  runtimePhaseLabel,
  taskLifecycleLabel,
  taskLifecycleTone,
} from './statusPresentation'

describe('statusPresentation', () => {
  it('normalizes task lifecycle labels across shell surfaces', () => {
    expect(taskLifecycleTone('running')).toBe('running')
    expect(taskLifecycleTone('warning')).toBe('idle')
    expect(taskLifecycleLabel('running')).toBe('Running')
    expect(taskLifecycleLabel('success')).toBe('Completed')
    expect(taskLifecycleLabel('cancelled')).toBe('Cancelled')
    expect(taskLifecycleLabel('unknown')).toBe('Idle')
  })

  it('builds tracked task detail from the best available fields', () => {
    expect(
      describeTrackedTask({
        id: 'task_1',
        command: 'run_double_ma_optimization',
        label: 'Optimization',
        status: 'running',
        startedAt: 1_700_000_000_000,
        phase: 'Evaluate',
        progressPct: 42.35,
      }),
    ).toBe('Evaluate (42.4%)')
  })

  it('summarizes active work before toast or history fallback', () => {
    expect(
      describeTaskActivity({
        activeTasks: [
          {
            id: 'task_1',
            command: 'run_double_ma_validation',
            label: 'Validation',
            status: 'running',
            startedAt: 1_700_000_000_000,
            message: 'Running validation 4/16',
          },
        ],
        latestToast: {
          id: 'toast_1',
          atUnixMs: 1_700_000_000_500,
          source: 'run_double_ma_validation',
          message: 'Validation failed.',
          level: 'error',
        },
      }),
    ).toBe('Running validation 4/16')

    expect(
      describeTaskActivity({
        activeTasks: [],
        latestToast: {
          id: 'toast_1',
          atUnixMs: 1_700_000_000_500,
          source: 'run_double_ma_validation',
          message: 'Validation failed.',
          level: 'error',
        },
      }),
    ).toBe('Validation failed.')

    expect(
      describeTaskActivity({
        activeTasks: [],
        latestToast: null,
        latestTask: {
          id: 'task_2',
          command: 'run_double_ma_validation',
          label: 'Validation',
          status: 'failed',
          startedAt: 1_700_000_000_000,
        },
      }),
    ).toBe('Validation failed.')
  })

  it('formats progress and runtime summaries consistently', () => {
    expect(
      formatOverallProgressLabel({
        pct: 42,
        label: 'Optimize 42/100',
        source: 'double_ma_progress',
        atUnixMs: 1_700_000_000_000,
      }),
    ).toContain('Optimize 42/100')

    expect(runtimePhaseLabel('booting')).toBe('Booting')
    expect(
      describeRuntimePhase({
        phase: 'booting',
        stage: 'Hydrating stores',
        elapsedMs: 52,
      }),
    ).toBe('Hydrating stores (52ms)')
    expect(
      describeRuntimePhase({
        phase: 'error',
        error: 'Renderer boot failed',
      }),
    ).toBe('Renderer boot failed')
  })

  it('normalizes task severity, source, and phase labels for diagnostics surfaces', () => {
    expect(taskLevelTone('warning')).toBe('warn')
    expect(taskLevelTone('done')).toBe('success')
    expect(taskLevelLabel('warn')).toBe('Warning')
    expect(taskLevelLabel('success')).toBe('Success')
    expect(formatTaskPhaseLabel('phase_changed')).toBe('Phase Changed')
    expect(formatTaskSourceLabel('run_double_ma_optimization')).toBe('Double Ma Optimization')
  })

  it('describes timeline entries and alerts consistently', () => {
    expect(
      describeLatestTaskOutcome({
        id: 'task_2',
        command: 'run_double_ma_validation',
        label: 'Validation',
        status: 'failed',
        startedAt: 1_700_000_000_000,
      }),
    ).toBe('Validation · Failed')

    expect(
      describeTimelineEntry({
        id: 'timeline_1',
        atUnixMs: 1_700_000_000_000,
        source: 'run_double_ma_validation',
        phase: 'running',
        message: 'Phase changed: Evaluate',
        level: 'info',
      }),
    ).toBe('Double Ma Validation · Running · Phase changed: Evaluate')

    expect(
      describeTaskToast({
        id: 'toast_1',
        atUnixMs: 1_700_000_000_000,
        source: 'run_double_ma_validation',
        message: 'Validation failed.',
        level: 'error',
      }),
    ).toBe('Double Ma Validation · Validation failed.')

    expect(
      describeTaskEvent({
        id: 'event_1',
        atUnixMs: 1_700_000_000_000,
        source: 'cancel_current_run',
        message: 'Cancellation requested.',
        level: 'warn',
      }),
    ).toBe('Cancel Current Run · Cancellation requested.')
  })
})
