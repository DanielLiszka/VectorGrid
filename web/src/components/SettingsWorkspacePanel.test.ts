// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import SettingsWorkspacePanel from './SettingsWorkspacePanel.vue'
import * as desktopProductization from '../services/desktopProductization'
import { useRuntimeStore } from '../stores/runtimeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useTaskStore } from '../stores/taskStore'
import { useWorkflowStore } from '../stores/workflowStore'

vi.mock('../services/desktopProductization', () => ({
  requestDesktopNotificationPermission: vi.fn(async () => undefined),
}))

describe('SettingsWorkspacePanel', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = `
      <div id="settingsShellGrid">
        <div id="settingsVueShellMount"></div>
        <div id="executionShellPane"></div>
      </div>
    `
  })

  it('renders the diagnostics surface from runtime and task state', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('settings')

    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markError(new Error('Renderer boot failed'))
    runtimeStore.setDesktopNotificationPermission('default')
    runtimeStore.setDesktopWindowState('vectorgrid-window-state.json', true)
    runtimeStore.pushDesktopLog('info', 'Desktop runtime bridge ready.')

    const taskStore = useTaskStore(pinia)
    taskStore.$patch({
      activeById: {
        task_active: {
          id: 'task_active',
          command: 'run_double_ma_optimization',
          label: 'Optimization',
          status: 'running',
          startedAt: 1_700_000_000_000,
          message: 'Running',
        },
      },
      activeOrder: ['task_active'],
      centerStatus: 'running',
      overallProgress: {
        pct: 42,
        label: 'Optimize 42/100',
        source: 'double_ma_progress',
        atUnixMs: 1_700_000_000_000,
      },
      history: [
        {
          id: 'task_finished',
          command: 'run_double_ma_validation',
          label: 'Validation',
          status: 'failed',
          startedAt: 1_700_000_000_000,
          endedAt: 1_700_000_001_000,
          error: 'Validation failed.',
        },
      ],
      timeline: [
        {
          id: 'timeline_1',
          atUnixMs: 1_700_000_000_000,
          source: 'run_double_ma_optimization',
          phase: 'running',
          message: 'Phase changed: Evaluate',
          level: 'info',
        },
      ],
      toasts: [
        {
          id: 'toast_1',
          atUnixMs: 1_700_000_000_000,
          source: 'run_double_ma_optimization',
          message: 'Optimization failed.',
          level: 'error',
        },
      ],
    })

    mount(SettingsWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Runtime and task center')
    expect(text).toContain('Renderer boot failed')
    expect(text).toContain('Optimize 42/100')
    expect(text).toContain('Task posture')
    expect(text).toContain('Running')
    expect(text).toContain('Failed')
    expect(text).toContain('Validation failed.')
    expect(text).toContain('No recently completed tasks.')
    expect(text).toContain('Running')
    expect(text).toContain('Phase changed: Evaluate')
    expect(text).toContain('Optimization failed.')
    expect(text).toContain('Error')
    expect(text).toContain('Desktop logs')
    expect(text).toContain('Desktop runtime bridge ready.')
    expect(text).toContain('Window geometry persists to vectorgrid-window-state.json.')
  })

  it('opens the shared task center from diagnostics actions', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('settings')

    const wrapper = mount(SettingsWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const openTaskCenter = Array.from(document.querySelectorAll('button')).find((button) =>
      String(button.textContent || '').includes('Open task center'),
    ) as HTMLButtonElement | undefined

    expect(openTaskCenter).toBeTruthy()
    openTaskCenter?.click()
    await nextTick()

    expect(localStorage.getItem('VTA_TASK_CENTER_OPEN_V1')).toBe('true')
    wrapper.unmount()
  })

  it('renders density and saved-perspective controls for workstation state', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('settings')

    mount(SettingsWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Saved perspectives')
    expect(text).toContain('Density')
    expect(text).toContain('Top Results Triage')
    expect(text).toContain('Save current to selected')
  })

  it('renders desktop-notification controls and requests permission on demand', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('settings')
    const settingsStore = useSettingsStore(pinia)
    settingsStore.setSelectField('settingsDesktopNotifications', 'failures')

    mount(SettingsWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Desktop notifications')
    expect(text).toContain('Failures and cancellations')

    const requestButton = Array.from(document.querySelectorAll('button')).find((button) =>
      String(button.textContent || '').includes('Request OS notification permission'),
    ) as HTMLButtonElement | undefined

    expect(requestButton).toBeTruthy()
    requestButton?.click()
    await nextTick()

    expect(desktopProductization.requestDesktopNotificationPermission).toHaveBeenCalledTimes(1)
  })
})
