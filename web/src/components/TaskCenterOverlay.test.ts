// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import TaskCenterOverlay from './TaskCenterOverlay.vue'
import { useTaskStore } from '../stores/taskStore'

describe('TaskCenterOverlay', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
  })

  it('is collapsed by default and persists open/closed toggle state', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const wrapper = mount(TaskCenterOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    const toggle = wrapper.get('.overlayToggle')
    expect(document.querySelector('.taskPanel')).toBeNull()
    expect(localStorage.getItem('VTA_TASK_CENTER_OPEN_V1')).toBe('false')

    await toggle.trigger('click')
    await nextTick()
    expect(document.querySelector('.taskPanel')).not.toBeNull()
    expect(localStorage.getItem('VTA_TASK_CENTER_OPEN_V1')).toBe('true')

    await toggle.trigger('click')
    await nextTick()
    expect(document.querySelector('.taskPanel')).toBeNull()
    expect(localStorage.getItem('VTA_TASK_CENTER_OPEN_V1')).toBe('false')

    wrapper.unmount()
  })

  it('moves focus into the task panel on open and restores focus to the toggle on close', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const wrapper = mount(TaskCenterOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const toggle = document.querySelector<HTMLButtonElement>('.overlayToggle')
    expect(toggle).not.toBeNull()

    toggle?.focus()
    await wrapper.get('.overlayToggle').trigger('click')
    await nextTick()
    await nextTick()

    const panel = document.querySelector<HTMLElement>('.taskPanel')
    expect(panel).not.toBeNull()
    expect(panel?.getAttribute('role')).toBe('dialog')
    expect(panel?.contains(document.activeElement)).toBe(true)
    expect(panel?.querySelector('.panelCloseBtn')).not.toBeNull()

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await nextTick()
    await nextTick()

    expect(document.querySelector('.taskPanel')).toBeNull()
    expect(document.activeElement).toBe(toggle)

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

    await wrapper.get('.overlayToggle').trigger('click')
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

    wrapper.unmount()
  })
})
