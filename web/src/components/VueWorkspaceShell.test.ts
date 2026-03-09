// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import VueWorkspaceShell from './VueWorkspaceShell.vue'
import { useSettingsStore } from '../stores/settingsStore'
import { useWorkflowStore } from '../stores/workflowStore'

describe('VueWorkspaceShell', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
    window.innerWidth = 1440
  })

  it('shows only workspace and workflow-step navigation, with local tabs for the active step', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('optimize', { persist: false })

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(document.querySelector('#workflowTabBar')).toBeNull()

    const controlsLabels = Array.from(document.querySelectorAll('#controlsTabBar .tabBtn')).map((el) => el.textContent?.trim())
    const resultsLabels = Array.from(document.querySelectorAll('#resultsTabBar .tabBtn')).map((el) => el.textContent?.trim())

    expect(controlsLabels).toEqual(['Optimize', 'Strategy', 'Advanced'])
    expect(resultsLabels).toEqual(['Overview', 'Top', 'Pareto', 'Heatmap'])

    wrapper.unmount()
  })

  it('hides the controls column when the persisted controls-collapsed setting is enabled', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    workflowStore.selectWorkflowStep('optimize', { persist: false })
    settingsStore.setCheckboxField('settingsControlsCollapsed', true)

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect((document.querySelector('.controlsColumn') as HTMLElement | null)?.style.display).toBe('none')
    expect(document.querySelector('#resultsTabBar')).not.toBeNull()

    wrapper.unmount()
  })

  it('mounts the finalizer workflow step on the meta surface', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('finalize', { persist: false })

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    const stepLabels = Array.from(document.querySelectorAll('#workflowStepBar .tabBtn')).map((el) => el.textContent?.trim())
    expect(stepLabels).toContain('5 Finalizer')
    expect(document.querySelector('#controlsTabBar')).toBeNull()
    expect(document.querySelector('#resultsTabBar')).toBeNull()
    expect(document.querySelector('[data-vue-panel=\"meta\"]')).not.toBeNull()

    wrapper.unmount()
  })
})
