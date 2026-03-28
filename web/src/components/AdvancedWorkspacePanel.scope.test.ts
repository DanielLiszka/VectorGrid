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
    <select id="maSource">
      <option value="outer">Outer option</option>
    </select>
  `
}

async function flushPanelTimers(): Promise<void> {
  await nextTick()
  await vi.runAllTimersAsync()
  await nextTick()
}

describe('AdvancedWorkspacePanel scope', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    installAdvancedPanelMountShell()
    localStorage.clear()
  })

  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('does not adopt legacy select options from outside the advanced panel root', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const advanced = useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DoubleMa'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(advanced.maSourceOptions).toEqual([])

    wrapper.unmount()
  })
})
