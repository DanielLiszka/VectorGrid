import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { useWorkflowStore } from '../../stores/workflowStore'
import ResearchCockpit from './ResearchCockpit.vue'

describe('ResearchCockpit', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    document.body.innerHTML = ''
  })

  it('renders the operational cockpit cards and step controls', () => {
    const wrapper = mount(ResearchCockpit, {
      global: { plugins: [createPinia()] },
      attachTo: document.body,
    })

    expect(wrapper.get('[data-qa-panel="workbench.researchCockpit"]').text()).toContain('Research Cockpit')
    expect(wrapper.find('[data-qa-state="workbench.cockpit.card.dataset"]').exists()).toBe(true)
    expect(wrapper.find('[data-qa-action="workbench.cockpit.step.configure"]').exists()).toBe(true)
    expect(wrapper.find('[data-state="active"]').attributes('aria-current')).toBe('step')

    wrapper.unmount()
  })

  it('moves to configure without changing backend contracts', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)

    const wrapper = mount(ResearchCockpit, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await wrapper.get('[data-qa-action="workbench.cockpit.step.configure"]').trigger('click')
    expect(workflowStore.workflowStep).toBe('optimize')
    expect(workflowStore.controlsTab).toBe('strategy')

    wrapper.unmount()
  })
})
