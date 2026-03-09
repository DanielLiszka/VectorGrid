// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import OptimizeWorkspacePanel from './OptimizeWorkspacePanel.vue'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useWorkflowStore } from '../stores/workflowStore'

function installOptimizePanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="opt"></section>
    <div id="optVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('OptimizeWorkspacePanel', () => {
  beforeEach(() => {
    installOptimizePanelMountShell()
    localStorage.clear()
  })

  it('shows backend and mode controls for Double MA', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    workflow.selectControlsTab('opt', { persist: false })
    optimize.strategyKind = 'DoubleMa'
    vi.spyOn(optimize, 'refreshGpuStatus').mockImplementation(() => {})

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.querySelector('#vueBackend')).not.toBeNull()
    expect(document.querySelector('#vueMode')).not.toBeNull()

    wrapper.unmount()
  })

  it('hides backend and mode controls when Composite MO manages its own execution path', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    workflow.selectControlsTab('opt', { persist: false })
    optimize.strategyKind = 'CompositeMo'
    vi.spyOn(optimize, 'refreshGpuStatus').mockImplementation(() => {})

    const wrapper = mount(OptimizeWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.querySelector('#vueBackend')).toBeNull()
    expect(document.querySelector('#vueMode')).toBeNull()
    expect(document.body.textContent || '').toContain('Composite MO manages execution through its own sampler')

    wrapper.unmount()
  })
})
