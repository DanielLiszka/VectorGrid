// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import OverviewWorkspacePanel from './OverviewWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useHistoryStore } from '../stores/historyStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
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

  it('keeps the explicit empty-state actions wired through the analysis store', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('overview', { persist: false })
    analysisStore.overviewEmptyVisible = true
    analysisStore.overviewEmptyTitle = 'Load data to begin'
    analysisStore.overviewEmptyBody = 'Load a CSV dataset or download candles from Polygon.'
    analysisStore.overviewEmptyHint = 'Start with a smaller dataset.'
    analysisStore.overviewPrimaryLabel = 'Browse CSV…'
    analysisStore.overviewSecondaryVisible = true
    analysisStore.overviewSecondaryLabel = 'Polygon…'
    const primarySpy = vi.spyOn(analysisStore, 'overviewEmptyPrimary').mockImplementation(() => undefined)
    const secondarySpy = vi.spyOn(analysisStore, 'overviewEmptySecondary').mockImplementation(() => undefined)

    mount(OverviewWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('Load data to begin')
    expect(text).toContain('Browse CSV…')
    expect(text).toContain('Polygon…')

    const buttons = Array.from(document.querySelectorAll<HTMLButtonElement>('button'))
    buttons.find((button) => String(button.textContent || '').includes('Browse CSV'))?.click()
    buttons.find((button) => String(button.textContent || '').includes('Polygon'))?.click()
    await flushPanel()

    expect(primarySpy).toHaveBeenCalledTimes(1)
    expect(secondarySpy).toHaveBeenCalledTimes(1)
  })

  it('shows the pending overview state when data is loaded but no analyzed winner exists yet', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('overview', { persist: false })
    dataStore.dataId = 'dataset_1'
    dataStore.datasetLabel = 'AAPL'
    analysisStore.overviewEmptyVisible = false
    analysisStore.statusLine = 'Ready to review once results exist.'

    mount(OverviewWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('No analyzed winner yet.')
    expect(text).toContain('Results will appear here after an optimization run is scored')
    expect(text).toContain('Ready to review once results exist.')
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
    expect(document.querySelectorAll('img.actionIcon')).toHaveLength(2)
    expect(document.querySelector('img.sectionGlyph')).not.toBeNull()

    const reviewTopButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Review Top results'),
    )
    reviewTopButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()

    expect(workflowStore.resultsTab).toBe('top')
  })

  it('surfaces execution-risk review when the current run enables short same-bar execution', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowArtifactsStore = useWorkflowArtifactsStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('overview', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.bestKicker = 'Winner ready'
    analysisStore.bestTitle = 'Rank #1 · SMA 10 / EMA 40'
    analysisStore.bestMeta = 'Strategy DoubleMa · Objective Sharpe'
    workflowArtifactsStore.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-04-09T12:00:00Z',
        req: {
          strategy: {
            long_only: false,
            trade_on_next_bar: false,
          },
        },
        res: { top: [] },
      },
    })

    mount(OverviewWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('Optimistic execution mode')
    expect(text).toContain('Same-bar execution')

    const reviewExecutionButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Review execution settings'),
    )
    reviewExecutionButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()

    expect(workflowStore.workspace).toBe('settings')
  })

  it('shows the compact execution route summary under the winner metadata', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowArtifactsStore = useWorkflowArtifactsStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('overview', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.bestKicker = 'Winner ready'
    analysisStore.bestTitle = 'Rank #1 · SMA 10 / EMA 40'
    analysisStore.bestMeta = 'Strategy DoubleMa · Objective Sharpe'
    workflowArtifactsStore.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-04-09T12:00:00Z',
        route_provenance: {
          requestedRoute: 'GpuOnly',
          resolvedRoute: 'GpuKernel',
          realizedRoute: 'CpuOnly',
          routeChanged: true,
          fallbackReason: 'cuda unavailable',
        },
        req: {
          backend: { backend: 'GpuOnly', device_id: 0 },
        },
        res: { top: [] },
      },
    })

    mount(OverviewWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('Route:')
    expect(text).toContain('requested=GpuOnly')
    expect(text).toContain('actual=CpuOnly')
    expect(text).toContain('fallback=cuda unavailable')
  })
})
