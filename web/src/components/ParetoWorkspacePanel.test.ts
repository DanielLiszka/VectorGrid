// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ParetoWorkspacePanel from './ParetoWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useWorkflowStore } from '../stores/workflowStore'

vi.mock('../services/analysisService', () => ({
  refreshParetoFromWorkflowRun: vi.fn(),
}))

function installParetoPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="pareto"></section>
    <div id="paretoVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('ParetoWorkspacePanel', () => {
  beforeEach(() => {
    installParetoPanelMountShell()
    localStorage.clear()
  })

  it('distinguishes no-data and no-run empty states before frontier data exists', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('pareto', { persist: false })

    const wrapper = mount(ParetoWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.body.textContent || '').toContain('Load data before opening the Pareto view.')

    dataStore.dataId = 'dataset_1'
    await flushPanel()

    expect(document.body.textContent || '').toContain('Run an optimization to generate Pareto points.')
    wrapper.unmount()
  })

  it('keeps visible axis and meta cues when Pareto output is ready', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('pareto', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.bestTitle = 'Rank #1 · Winner'
    analysisStore.paretoX = 'max_dd'
    analysisStore.paretoY = 'pnl'
    analysisStore.paretoXOptions = [{ value: 'max_dd', label: 'MaxDD', disabled: false }]
    analysisStore.paretoYOptions = [{ value: 'pnl', label: 'PnL', disabled: false }]
    analysisStore.paretoMeta = '12 points • x=max_dd • y=pnl'
    analysisStore.paretoTruthNote = 'Pareto points show tradeoffs, not proof of robustness.'
    analysisStore.paretoRepresentativeSummary = 'x-extreme #2 • y-extreme #5 • selected #3'
    analysisStore.paretoConfidenceNote = 'Validation has not run. Frontier membership alone does not approve a candidate.'

    mount(ParetoWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('X axis')
    expect(text).toContain('Y axis')
    expect(text).toContain('12 points • x=max_dd • y=pnl')
    expect(text).toContain('Pareto points show tradeoffs, not proof of robustness.')
    expect(text).toContain('Representative points: x-extreme #2 • y-extreme #5 • selected #3')
    expect(text).toContain('Validation has not run. Frontier membership alone does not approve a candidate.')
    expect(text).toContain('Use the Top results table as the keyboard-friendly way')
  })

  it('keeps an explicit Review Top results handoff', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('pareto', { persist: false })

    mount(ParetoWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const button = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((candidate) =>
      String(candidate.textContent || '').includes('Review Top results'),
    )
    button?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()

    expect(workflowStore.resultsTab).toBe('top')
  })
})
