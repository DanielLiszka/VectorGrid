
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import AnalysisContextDock from './AnalysisContextDock.vue'
import { useWorkflowStore } from '../stores/workflowStore'

describe('AnalysisContextDock', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
  })

  it('is collapsed by default and persists open state when toggled', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('optimize', { persist: false })

    const wrapper = mount(AnalysisContextDock, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    const toggle = wrapper.get('.dockToggle')
    expect(document.querySelector('.analysisDock')).toBeNull()

    await toggle.trigger('click')
    await nextTick()

    expect(document.querySelector('.analysisDock')).not.toBeNull()
    expect(localStorage.getItem('VTA_ANALYSIS_DOCK_OPEN_V1')).toBe('true')
    wrapper.unmount()
  })

  it('closes when workflow context is no longer eligible', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('optimize', { persist: false })

    const wrapper = mount(AnalysisContextDock, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    await wrapper.get('.dockToggle').trigger('click')
    await nextTick()
    expect(document.querySelector('.analysisDock')).not.toBeNull()

    workflowStore.selectWorkflow('data', { persist: false })
    await nextTick()

    expect(document.querySelector('.analysisDock')).toBeNull()
    expect(document.querySelector('.analysisDockRoot')).toBeNull()
    wrapper.unmount()
  })
})
