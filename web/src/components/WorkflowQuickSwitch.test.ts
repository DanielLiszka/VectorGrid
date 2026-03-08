// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import WorkflowQuickSwitch from './WorkflowQuickSwitch.vue'

describe('WorkflowQuickSwitch', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
  })

  it('toggles the workflow nav popover and persists the open state', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const wrapper = mount(WorkflowQuickSwitch, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    expect(document.querySelector('.workflowSwitch')).toBeNull()

    const toggle = wrapper.get('.workflowSwitchToggle')
    await toggle.trigger('click')
    await nextTick()

    expect(document.querySelector('.workflowSwitch')).not.toBeNull()
    expect(localStorage.getItem('VTA_WORKFLOW_NAV_OPEN_V1')).toBe('true')

    await toggle.trigger('click')
    await nextTick()

    expect(document.querySelector('.workflowSwitch')).toBeNull()
    expect(localStorage.getItem('VTA_WORKFLOW_NAV_OPEN_V1')).toBe('false')
    wrapper.unmount()
  })
})
