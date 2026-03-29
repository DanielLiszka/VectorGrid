import { describe, expect, it } from 'vitest'
import { desktopNotificationIntentForTask } from './desktopNotificationPolicy'

describe('desktopNotificationIntentForTask', () => {
  it('suppresses success notifications when failures-only mode is enabled', () => {
    const intent = desktopNotificationIntentForTask(
      {
        id: 'task_success',
        command: 'run_double_ma_optimization',
        label: 'Optimization',
        status: 'success',
        startedAt: 1,
        endedAt: 2,
      },
      'failures',
    )

    expect(intent).toBeNull()
  })

  it('emits important completion notifications for long-running optimization work', () => {
    const intent = desktopNotificationIntentForTask(
      {
        id: 'task_success',
        command: 'run_double_ma_optimization',
        label: 'Optimization',
        status: 'success',
        startedAt: 1,
        endedAt: 2,
        message: 'Optimization completed successfully.',
      },
      'important',
    )

    expect(intent).toMatchObject({
      title: 'Double Ma Optimization complete',
      body: 'Optimization completed successfully.',
    })
  })

  it('emits failure notifications even for smaller commands when failures mode is enabled', () => {
    const intent = desktopNotificationIntentForTask(
      {
        id: 'task_failed',
        command: 'open_reports_dir',
        label: 'Open Reports Dir',
        status: 'failed',
        startedAt: 1,
        endedAt: 2,
        error: 'Access denied.',
      },
      'failures',
    )

    expect(intent).toMatchObject({
      title: 'Open Reports Dir failed',
      body: 'Access denied.',
    })
  })

  it('suppresses notifications while the desktop window remains visible', () => {
    const intent = desktopNotificationIntentForTask(
      {
        id: 'task_visible',
        command: 'run_double_ma_validation',
        label: 'Validation',
        status: 'failed',
        startedAt: 1,
        endedAt: 2,
        error: 'Validation failed.',
      },
      'important',
      { documentVisibility: 'visible' },
    )

    expect(intent).toBeNull()
  })
})
