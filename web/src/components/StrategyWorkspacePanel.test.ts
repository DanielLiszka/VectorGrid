// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import StrategyWorkspacePanel from './StrategyWorkspacePanel.vue'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useStrategyStore } from '../stores/strategyStore'
import { useWorkflowStore } from '../stores/workflowStore'

function installStrategyPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="strategy"></section>
    <div id="strategyVueShellMount"></div>
    <input id="allowFlip" type="checkbox" />
  `
}

async function flushPanelTimers(): Promise<void> {
  await nextTick()
  await vi.runAllTimersAsync()
  await nextTick()
}

describe('StrategyWorkspacePanel', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    installStrategyPanelMountShell()
    localStorage.clear()
  })

  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('ignores duplicate legacy ids outside the strategy panel root', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const strategy = useStrategyStore(pinia)
    workflow.selectControlsTab('strategy', { persist: false })
    optimize.strategyKind = 'CompositeTrend'

    const outerAllowFlip = document.querySelector('#allowFlip') as HTMLInputElement | null
    if (outerAllowFlip) outerAllowFlip.checked = false

    const wrapper = mount(StrategyWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(strategy.allowFlip).toBe(true)

    wrapper.unmount()
  })
})
