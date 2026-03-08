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
})
