import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import TaskCenterOverlay from './TaskCenterOverlay.vue'
import { openTaskCenter, useTaskCenterVisibility } from '../composables/useTaskCenterVisibility'
import { useTaskStore } from '../stores/taskStore'
import { useWorkflowStore } from '../stores/workflowStore'

describe('TaskCenterOverlay', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
    useTaskCenterVisibility().value = false
  })

  it('is collapsed by default and persists open/closed state when opened by the shell', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const wrapper = mount(TaskCenterOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(document.querySelector('.taskPanel')).toBeNull()
    expect(document.querySelector('.taskOverlayRoot')).toBeNull()
    expect(useTaskCenterVisibility().value).toBe(false)

    openTaskCenter(useTaskCenterVisibility())
    await nextTick()
    expect(document.querySelector('.taskPanel')).not.toBeNull()
    expect(localStorage.getItem('VTA_TASK_CENTER_OPEN_V1')).toBe('true')

    await wrapper.get('.overlayToggle').trigger('click')
    await nextTick()
    expect(document.querySelector('.taskPanel')).toBeNull()
    expect(localStorage.getItem('VTA_TASK_CENTER_OPEN_V1')).toBe('false')

    wrapper.unmount()
  })

  it('hides the idle launcher and clamps the shared-open launcher back into the viewport', async () => {
    localStorage.setItem('VTA_TASK_CENTER_POS_V1', JSON.stringify({ x: -640, y: 9_999 }))
    localStorage.setItem('VTA_TASK_CENTER_POS_INIT_V1', 'true')
    window.innerWidth = 360
    window.innerHeight = 240

    const pinia = createPinia()
    setActivePinia(pinia)

    const wrapper = mount(TaskCenterOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    expect(document.querySelector('.taskOverlayRoot')).toBeNull()

    openTaskCenter(useTaskCenterVisibility())
    await nextTick()
    await nextTick()

    const toggle = wrapper.get('.overlayToggle')
    const root = document.querySelector<HTMLElement>('.taskOverlayRoot')
    const stored = JSON.parse(String(localStorage.getItem('VTA_TASK_CENTER_POS_V1') || '{}')) as {
      x?: number
      y?: number
    }

    expect(toggle.text()).toContain('Task Center')
    expect(typeof stored.x).toBe('number')
    expect(typeof stored.y).toBe('number')
    expect(Number(stored.x)).toBeGreaterThanOrEqual(8)
    expect(Number(stored.y)).toBeGreaterThanOrEqual(8)
    expect(Number(stored.x)).toBeLessThanOrEqual(window.innerWidth - 8)
    expect(Number(stored.y)).toBeLessThanOrEqual(window.innerHeight - 8)
    expect(root?.style.left).toBe(`${Math.round(Number(stored.x))}px`)
    expect(root?.style.top).toBe(`${Math.round(Number(stored.y))}px`)

    wrapper.unmount()
  })

  it('keeps the shared-open launcher below the desktop titlebar hit area', async () => {
    localStorage.setItem('VTA_TASK_CENTER_POS_V1', JSON.stringify({ x: 320, y: 12 }))
    localStorage.setItem('VTA_TASK_CENTER_POS_INIT_V1', 'true')
    document.documentElement.style.setProperty('--desktop-titlebar-h', '48px')
    window.innerWidth = 720
    window.innerHeight = 480

    const pinia = createPinia()
    setActivePinia(pinia)

    const wrapper = mount(TaskCenterOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    expect(document.querySelector('.taskOverlayRoot')).toBeNull()

    openTaskCenter(useTaskCenterVisibility())
    await nextTick()
    await nextTick()

    const root = document.querySelector<HTMLElement>('.taskOverlayRoot')
    const stored = JSON.parse(String(localStorage.getItem('VTA_TASK_CENTER_POS_V1') || '{}')) as {
      y?: number
    }

    expect(Number(stored.y)).toBeGreaterThanOrEqual(56)
    expect(root?.style.top).toBe(`${Math.round(Number(stored.y))}px`)

    wrapper.unmount()
    document.documentElement.style.removeProperty('--desktop-titlebar-h')
  })

  it('moves focus into the task panel on open and restores focus to the opener on close', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const wrapper = mount(TaskCenterOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const opener = document.createElement('button')
    opener.type = 'button'
    opener.textContent = 'Open task center'
    document.body.appendChild(opener)

    opener.focus()
    openTaskCenter(useTaskCenterVisibility(), opener)
    await nextTick()
    await nextTick()

    const panel = document.querySelector<HTMLElement>('.taskPanel')
    expect(panel).not.toBeNull()
    expect(panel?.getAttribute('role')).toBe('dialog')
    expect(panel?.contains(document.activeElement)).toBe(true)
    expect(panel?.querySelector('.panelCloseBtn')).not.toBeNull()
    expect(panel?.querySelector('[aria-label="Task center active posture"]')).not.toBeNull()
    expect(panel?.querySelector('[aria-label="Task center recent outcomes"]')).not.toBeNull()
    expect(panel?.querySelector('[aria-label="Task center events"]')).not.toBeNull()
    expect(panel?.querySelector('[aria-label="Task center timeline"]')).not.toBeNull()

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await nextTick()
    await nextTick()

    expect(document.querySelector('.taskPanel')).toBeNull()
    expect(document.activeElement).toBe(opener)

    wrapper.unmount()
  })

  it('opens from another shell control through the shared task-center state', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const opener = document.createElement('button')
    opener.type = 'button'
    opener.textContent = 'Open task center from shell'
    document.body.appendChild(opener)

    const wrapper = mount(TaskCenterOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    opener.focus()
    openTaskCenter(useTaskCenterVisibility(), opener)
    await nextTick()
    await nextTick()

    expect(document.querySelector('.taskPanel')).not.toBeNull()
    expect(document.querySelector('.taskPanel')?.contains(document.activeElement)).toBe(true)

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await nextTick()
    await nextTick()

    expect(document.querySelector('.taskPanel')).toBeNull()

    wrapper.unmount()
  })

  it('renders shared alert and timeline summaries with normalized severity labels', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const taskStore = useTaskStore(pinia)
    taskStore.$patch({
      centerStatus: 'failed',
      events: [
        {
          id: 'event_1',
          atUnixMs: 1_700_000_000_000,
          source: 'cancel_current_run',
          message: 'Cancellation requested.',
          level: 'warn',
        },
      ],
      timeline: [
        {
          id: 'timeline_1',
          atUnixMs: 1_700_000_000_000,
          source: 'run_double_ma_validation',
          phase: 'running',
          message: 'Phase changed: Evaluate',
          level: 'info',
        },
      ],
      toasts: [
        {
          id: 'toast_1',
          atUnixMs: 1_700_000_000_000,
          source: 'run_double_ma_validation',
          message: 'Validation failed.',
          level: 'error',
        },
      ],
    })

    const wrapper = mount(TaskCenterOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    openTaskCenter(useTaskCenterVisibility())
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Warning')
    expect(text).toContain('Error')
    expect(text).toContain('Cancel Current Run')
    expect(text).toContain('Double Ma Validation')

    const toastCard = document.querySelector('.toastCard')
    const timelineRow = Array.from(document.querySelectorAll('.eventRow')).find((row) =>
      String(row.getAttribute('aria-label') || '').includes('Phase changed: Evaluate'),
    )
    expect(toastCard?.getAttribute('aria-label')).toContain('Validation failed.')
    expect(timelineRow?.getAttribute('aria-label')).toContain('Double Ma Validation')
    expect(document.querySelector('.toastLevel svg')).not.toBeNull()
    expect(document.querySelector('.eventRow .eventBadge svg')).not.toBeNull()

    wrapper.unmount()
  })

  it('groups task posture and routes related actions to the correct review surface', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const workflowStore = useWorkflowStore(pinia)
    const taskStore = useTaskStore(pinia)
    taskStore.$patch({
      activeById: {
        task_running: {
          id: 'task_running',
          command: 'run_double_ma_optimization',
          label: 'Optimization',
          status: 'running',
          startedAt: 1_700_000_000_000,
          phase: 'Evaluate',
          message: 'Evaluate (42.0%)',
        },
        task_queued: {
          id: 'task_queued',
          command: 'run_double_ma_validation',
          label: 'Validation',
          status: 'queued',
          startedAt: 1_700_000_000_100,
          message: 'Queued.',
        },
      },
      activeOrder: ['task_running', 'task_queued'],
      history: [
        {
          id: 'task_failed',
          command: 'run_double_ma_validation',
          label: 'Validation',
          status: 'failed',
          startedAt: 1_700_000_000_000,
          endedAt: 1_700_000_001_000,
          message: 'Failed. Open Details for technical information.',
          error: 'run_double_ma_validation: unknown data id data-1',
        },
        {
          id: 'task_success',
          command: 'run_double_ma_optimization',
          label: 'Optimization',
          status: 'success',
          startedAt: 1_700_000_000_000,
          endedAt: 1_700_000_001_500,
          message: 'Completed.',
        },
      ],
    })

    const wrapper = mount(TaskCenterOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    openTaskCenter(useTaskCenterVisibility())
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Active posture')
    expect(text).toContain('Running')
    expect(text).toContain('Queued')
    expect(text).toContain('Recent outcomes')
    expect(text).toContain('Attention')
    expect(text).toContain('Completed')
    expect(document.querySelector('.taskStatus img, .taskStatus svg')).not.toBeNull()
    const attentionCard = document.querySelector(
      '.section[aria-label="Task center recent outcomes"] .postureColumn:first-child .taskCard',
    )
    expect(attentionCard?.textContent).toContain('Failed. Open Details')
    expect(attentionCard?.textContent).not.toContain('Queued.')
    expect(attentionCard?.querySelector('summary')?.textContent).toContain('Details')

    const action = Array.from(document.querySelectorAll<HTMLButtonElement>('.relatedBtn')).find((button) =>
      String(button.textContent || '').includes('Open validation'),
    )
    expect(action).toBeTruthy()
    action?.click()
    await nextTick()

    expect(workflowStore.workflowStep).toBe('validate')
    expect(document.querySelector('.taskPanel')).toBeNull()

    wrapper.unmount()
  })
})
