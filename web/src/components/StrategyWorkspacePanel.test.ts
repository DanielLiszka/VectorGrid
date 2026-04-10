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

  it('groups long-tail constraints behind one disclosure and auto-opens it when a non-default guard is active', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const strategy = useStrategyStore(pinia)
    workflow.selectControlsTab('strategy', { persist: false })
    optimize.strategyKind = 'DoubleMa'
    strategy.dmConstraintMaxVolatility = '0.12'

    const wrapper = mount(StrategyWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    const moreConstraints = Array.from(
      document.querySelectorAll<HTMLDetailsElement>('details.panelDetails'),
    ).find((details) => String(details.querySelector('summary')?.textContent || '').includes('More constraints'))

    expect(document.querySelector('#vueDmMinTrades')).not.toBeNull()
    expect(document.querySelector('#vueDmMinSharpe')).not.toBeNull()
    expect(moreConstraints).not.toBeUndefined()
    expect(moreConstraints?.hasAttribute('open')).toBe(true)
    expect(document.querySelector('#vueDmMaxVolatility')).not.toBeNull()

    wrapper.unmount()
  })

  it('keeps the shared constraint disclosure summary focusable', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    workflow.selectControlsTab('strategy', { persist: false })
    optimize.strategyKind = 'DoubleMa'

    const wrapper = mount(StrategyWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    const summary = document.querySelector('.strategyConstraintDetails summary') as HTMLElement | null
    expect(summary).not.toBeNull()

    summary?.focus()
    expect(document.activeElement).toBe(summary)

    wrapper.unmount()
  })

  it('shows the derived cost hint in the strategy panel after preset-backed cost updates', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const strategy = useStrategyStore(pinia)
    workflow.selectControlsTab('strategy', { persist: false })
    optimize.strategyKind = 'DoubleMa'
    strategy.setSelectField('costPreset', 'stocks_liquid')
    strategy.setInputField('spreadBps', '1.00')
    strategy.setInputField('slippageBps', '0.50')

    const wrapper = mount(StrategyWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    const text = document.body.textContent || ''
    expect(text).toContain('Execution cost model')
    expect(text).toContain('Total: 1.10 bps/side (2.20 bps round-trip)')

    wrapper.unmount()
  })

  it('shows only the selected strategy family section for surfaced family-local controls', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    workflow.selectControlsTab('strategy', { persist: false })

    optimize.strategyKind = 'Vcc'
    const vccWrapper = mount(StrategyWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueVccSqueezePeriod')).not.toBeNull()
    expect(document.querySelector('#vueVcetVidyaShortPeriod')).toBeNull()

    vccWrapper.unmount()
    installStrategyPanelMountShell()

    optimize.strategyKind = 'Vcet'
    const vcetWrapper = mount(StrategyWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueVcetVidyaShortPeriod')).not.toBeNull()
    expect(document.querySelector('#vueVccSqueezePeriod')).toBeNull()

    vcetWrapper.unmount()
  })
})
