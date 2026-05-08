
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import { createTestPinia } from '../test/pinia'
import WorkflowQuickSwitch from './WorkflowQuickSwitch.vue'

describe('WorkflowQuickSwitch', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
  })

  it('toggles the workflow nav popover and persists the open state', async () => {
    const pinia = createTestPinia()
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

  it('exposes grouped navigation semantics and moves between items with arrow keys', async () => {
    const pinia = createTestPinia()
    const wrapper = mount(WorkflowQuickSwitch, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await wrapper.get('.workflowSwitchToggle').trigger('click')
    await nextTick()

    const workspaceGroup = document.querySelector('[aria-label="Workspace quick switch"]')
    const libraryButton = document.querySelector('[data-switch-group="workspace"] [data-switch-key="library"]') as HTMLButtonElement | null
    const aggregateButton = document.querySelector('[data-switch-group="workspace"] [data-switch-key="aggregate"]') as HTMLButtonElement | null
    const stepGroup = document.querySelector('[aria-label="Workflow step quick switch"]')
    const validateButton = document.querySelector('[data-switch-group="step"] [data-switch-key="validate"]') as HTMLButtonElement | null
    const finalizeButton = document.querySelector('[data-switch-group="step"] [data-switch-key="finalize"]') as HTMLButtonElement | null

    expect(workspaceGroup?.getAttribute('aria-label')).toBe('Workspace quick switch')
    expect(libraryButton?.getAttribute('tabindex')).toBe('-1')

    libraryButton?.focus()
    const arrowEvent = new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true })
    libraryButton?.dispatchEvent(arrowEvent)
    await nextTick()
    expect(arrowEvent.defaultPrevented).toBe(true)
    expect(aggregateButton?.getAttribute('aria-current')).toBe('page')
    expect(document.activeElement).toBe(aggregateButton)

    expect(stepGroup?.getAttribute('aria-label')).toBe('Workflow step quick switch')
    validateButton?.focus()
    const endEvent = new KeyboardEvent('keydown', { key: 'End', bubbles: true, cancelable: true })
    validateButton?.dispatchEvent(endEvent)
    await nextTick()
    expect(endEvent.defaultPrevented).toBe(true)
    expect(finalizeButton?.getAttribute('aria-current')).toBe('step')
    expect(document.activeElement).toBe(finalizeButton)

    wrapper.unmount()
  })
})
