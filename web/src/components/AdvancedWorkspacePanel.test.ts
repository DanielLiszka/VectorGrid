// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import AdvancedWorkspacePanel from './AdvancedWorkspacePanel.vue'
import { useAdvancedStore } from '../stores/advancedStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useWorkflowStore } from '../stores/workflowStore'

function installAdvancedPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="advanced"></section>
    <div id="advancedVueShellMount"></div>
  `
}

async function flushPanelTimers(): Promise<void> {
  await nextTick()
  await vi.runAllTimersAsync()
  await nextTick()
}

describe('AdvancedWorkspacePanel', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    installAdvancedPanelMountShell()
    localStorage.clear()
  })

  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('shows heatmap and export controls for Double MA', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DoubleMa'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueHeatmapBins')).not.toBeNull()
    expect(document.querySelector('#vueExportAllCsvPath')).not.toBeNull()

    wrapper.unmount()
  })

  it('hides output-shaping controls that Composite MO does not use', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'CompositeMo'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueHeatmapBins')).toBeNull()
    expect(document.querySelector('#vueTopK')).toBeNull()
    expect(document.querySelector('#vueIncludeAll')).toBeNull()
    expect(document.querySelector('#vueExportAllCsvPath')).toBeNull()
    expect(document.body.textContent || '').toContain('Composite MO uses its own search-space and prescan settings')

    wrapper.unmount()
  })

  it('keeps Top K but hides heatmap for VCC', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'Vcc'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueHeatmapBins')).toBeNull()
    expect(document.querySelector('#vueTopK')).not.toBeNull()
    expect(document.querySelector('#vueIncludeAll')).not.toBeNull()

    wrapper.unmount()
  })
})
