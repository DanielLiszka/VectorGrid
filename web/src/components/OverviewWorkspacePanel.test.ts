// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import OverviewWorkspacePanel from './OverviewWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useHistoryStore } from '../stores/historyStore'
import { useWorkflowStore } from '../stores/workflowStore'

function installOverviewPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="overview"></section>
    <div id="overviewVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('OverviewWorkspacePanel', () => {
  beforeEach(() => {
    installOverviewPanelMountShell()
    localStorage.clear()
  })

  it('keeps winner framing and baseline review while providing an explicit Top handoff', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const historyStore = useHistoryStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('overview', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.bestKicker = 'Winner ready'
    analysisStore.bestTitle = 'Rank #1 · SMA 10 / EMA 40'
    analysisStore.bestMeta = 'Strategy DoubleMa · Objective Sharpe'
    analysisStore.bestSummary = 'Selected candidate summary.'
    analysisStore.bestStats = [
      { label: 'Sharpe', value: '1.80', tone: 'positive' },
      { label: 'MaxDD', value: '0.12', tone: 'warning' },
    ]
    historyStore.compareOptions = [
      { value: '', label: 'No baseline', disabled: false },
      { value: 'hist:run_a', label: 'Baseline A', disabled: false },
    ]

    mount(OverviewWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('Review Top results')
    expect(text).toContain('Export top CSV')
    expect(text).toContain('Export JSON')
    expect(text).toContain('Baseline review')
    expect(text).toContain('Set baseline = current')
    expect(text).toContain('Comparison details')

    const reviewTopButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Review Top results'),
    )
    reviewTopButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()

    expect(workflowStore.resultsTab).toBe('top')
  })
})
