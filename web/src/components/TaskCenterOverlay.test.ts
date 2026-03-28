// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import TaskCenterOverlay from './TaskCenterOverlay.vue'

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
})
