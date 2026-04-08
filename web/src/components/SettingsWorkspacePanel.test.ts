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
    expect(text).toContain('Desktop presence')
    expect(text).toContain('Foreground-first workstation')
    expect(text).toContain('Close exits the app by default.')
    expect(document.querySelector('.diagnosticsBadgeIcon')).not.toBeNull()
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

  it('filters saved perspectives with local fuzzy search without replacing global command search', async () => {
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

    const searchInput = document.querySelector<HTMLInputElement>('#vueSettingsPerspectiveSearch')
    expect(searchInput).toBeTruthy()

    searchInput!.value = 'librari'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    await nextTick()

    const optionText = Array.from(document.querySelectorAll<HTMLSelectElement>('#vueSettingsPerspectiveSelect option')).map(
      (option) => String(option.textContent || '').trim(),
    )
    expect(optionText.some((label) => label.includes('Run Library'))).toBe(true)
    expect(document.body.textContent || '').toContain('Search stays local to saved perspectives; use Ctrl+K for global actions.')
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
    expect(text).toContain('VectorGrid stays foreground-first')
    expect(text).toContain('close exits by default')
    expect(text).toContain('meaningful active work asks before closing')

    const requestButton = Array.from(document.querySelectorAll('button')).find((button) =>
      String(button.textContent || '').includes('Request OS notification permission'),
    ) as HTMLButtonElement | undefined

    expect(requestButton).toBeTruthy()
    requestButton?.click()
    await nextTick()

    expect(desktopProductization.requestDesktopNotificationPermission).toHaveBeenCalledTimes(1)
  })

  it('copies a desktop diagnostics snapshot for operator troubleshooting', async () => {
    const writeText = vi.fn(async () => undefined)
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    })

    const pinia = createPinia()
    setActivePinia(pinia)

    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('settings')

    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.markReady()
    runtimeStore.setDesktopNotificationPermission('granted')
    runtimeStore.setDesktopWindowState('vectorgrid-window-state.json', true)
    runtimeStore.pushDesktopLog('warn', 'Desktop bridge recovered from a transient failure.')

    const taskStore = useTaskStore(pinia)
    taskStore.$patch({
      history: [
        {
          id: 'task_success',
          command: 'run_double_ma_validation',
          label: 'Validation',
          status: 'success',
          startedAt: 1_700_000_000_000,
          endedAt: 1_700_000_001_000,
          message: 'Validation complete.',
        },
      ],
      timeline: [
        {
          id: 'timeline_copy',
          atUnixMs: 1_700_000_000_000,
          source: 'run_double_ma_validation',
          phase: 'success',
          message: 'Validation completed.',
          level: 'success',
        },
      ],
      toasts: [
        {
          id: 'toast_copy',
          atUnixMs: 1_700_000_000_000,
          source: 'run_double_ma_validation',
          message: 'Validation complete.',
          level: 'success',
        },
      ],
    })

    mount(SettingsWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const copyButton = Array.from(document.querySelectorAll('button')).find((button) =>
      String(button.textContent || '').includes('Copy diagnostics snapshot'),
    ) as HTMLButtonElement | undefined

    expect(copyButton).toBeTruthy()
    copyButton?.click()
    await nextTick()

    expect(writeText).toHaveBeenCalledTimes(1)
    const copiedCalls = writeText.mock.calls as unknown as string[][]
    const copiedPayload = String(copiedCalls[copiedCalls.length - 1]?.[0] || '')
    expect(copiedPayload).toContain('VectorAlpha desktop diagnostics snapshot')
    expect(copiedPayload).toContain('Desktop bridge recovered from a transient failure.')
    expect(copiedPayload).toContain('Validation completed.')
    expect(copiedPayload).toContain('Desktop presence')
    expect(copiedPayload).toContain('Foreground-first workstation')
  })

  it('filters and clears desktop logs from diagnostics without affecting other task surfaces', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('settings')

    const runtimeStore = useRuntimeStore(pinia)
    runtimeStore.pushDesktopLog('info', 'Desktop runtime bridge ready.')
    runtimeStore.pushDesktopLog('warn', 'Desktop bridge recovered from a transient failure.')
    runtimeStore.pushDesktopLog('error', 'Desktop bridge failed to stream one entry.')

    const taskStore = useTaskStore(pinia)
    taskStore.$patch({
      timeline: [
        {
          id: 'timeline_filter',
          atUnixMs: 1_700_000_000_000,
          source: 'run_double_ma_validation',
          phase: 'running',
          message: 'Validation is still running.',
          level: 'info',
        },
      ],
    })

    mount(SettingsWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const desktopLogsSection = Array.from(document.querySelectorAll<HTMLElement>('section')).find((section) =>
      String(section.textContent || '').includes('Desktop logs'),
    )
    expect(desktopLogsSection).toBeTruthy()

    const filter = desktopLogsSection?.querySelector<HTMLSelectElement>('#settingsDesktopLogFilter')
    expect(filter).toBeTruthy()

    filter!.value = 'attention'
    filter!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()
    await nextTick()

    const filteredText = String(desktopLogsSection?.textContent || '')
    expect(filteredText).toContain('Warnings and errors')
    expect(filteredText).toContain('Desktop bridge recovered from a transient failure.')
    expect(filteredText).toContain('Desktop bridge failed to stream one entry.')
    expect(filteredText).not.toContain('Desktop runtime bridge ready.')
    expect(filteredText).toContain('2 shown of 3')
    expect(document.body.textContent || '').toContain('Validation is still running.')

    const clearButton = Array.from(desktopLogsSection?.querySelectorAll('button') || []).find((button) =>
      String(button.textContent || '').includes('Clear desktop logs'),
    ) as HTMLButtonElement | undefined
    expect(clearButton).toBeTruthy()
    clearButton?.click()
    await nextTick()
    await nextTick()

    expect(runtimeStore.desktopLogs).toHaveLength(0)
    expect(String(desktopLogsSection?.textContent || '')).toContain('No desktop log entries yet.')
    expect(document.body.textContent || '').toContain('Validation is still running.')
  })
})
