import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { markRaw, nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import CandidateDetailWorkspacePanel from './CandidateDetailWorkspacePanel.vue'
import { drillClearTradeSelection, drillNextTrade } from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { buildDrillChartModel, type DrillChartPriceBar } from '../visualizations/drillChartModel'

vi.mock('lightweight-charts', () => {
  const series = () => ({ setData: vi.fn() })
  return {
    CandlestickSeries: {},
    LineSeries: {},
    HistogramSeries: {},
    createChart: vi.fn(() => ({
      addSeries: vi.fn(() => series()),
      addCandlestickSeries: vi.fn(() => series()),
      applyOptions: vi.fn(),
      remove: vi.fn(),
      subscribeClick: vi.fn(),
      unsubscribeClick: vi.fn(),
      timeScale: () => ({ fitContent: vi.fn() }),
    })),
    createSeriesMarkers: vi.fn(() => ({ setMarkers: vi.fn() })),
  }
})

function installDrillPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="drill">
      <div id="drillVueShellMount"></div>
    </section>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

const drillBars: DrillChartPriceBar[] = [
  { time: 1_744_500_000, open: 100, high: 101, low: 99.8, close: 100.6, fromIndex: 1, toIndex: 1, volume: 0 },
  { time: 1_744_500_060, open: 100.6, high: 101.2, low: 100.3, close: 101.0, fromIndex: 2, toIndex: 2, volume: 0 },
  { time: 1_744_500_120, open: 101.0, high: 101.1, low: 100.2, close: 100.4, fromIndex: 3, toIndex: 3, volume: 0 },
]

const drillCurves = {
  trades: [
    { entry_t: 1, exit_t: 2, direction: 1, pnl: 0.03, bars: 1, open: false },
    { entry_t: 2, exit_t: 3, direction: -1, pnl: -0.01, bars: 1, open: false },
  ],
  trade_stats: {
    closed_trades: 2,
    open_trades: 0,
    wins: 1,
    losses: 1,
    win_rate: 0.5,
    avg_win: 0.03,
    avg_loss: -0.01,
    profit_factor: 3,
    expectancy: 0.01,
  },
  indicator_bars: [
    { time: 1_744_500_000, from_index: 1, to_index: 1, fast: 100.2, slow: 100.0 },
    { time: 1_744_500_060, from_index: 2, to_index: 2, fast: 100.5, slow: 100.1 },
  ],
}

describe('CandidateDetailWorkspacePanel', () => {
  beforeEach(() => {
    installDrillPanelMountShell()
    localStorage.clear()
  })

  it('rerenders on visible mount and keeps the candidate detail evidence surface visible to contributors', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    analysisStore.statusLine = 'Candidate detail ready.'
    const rerenderSpy = vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(rerenderSpy).toHaveBeenCalled()
    expect(text).toContain('Price + trades')
    expect(text).toContain('Stability')
    expect(text).toContain('Trades')
    expect(text).toContain('If the charts stay empty')
    expect(text).toContain('Candidate detail ready.')
    expect(document.querySelector('#tradeTableBodyVue')).not.toBeNull()
    expect(document.querySelector('#stabilityTableBodyVue')).not.toBeNull()
  })

  it('keeps an explicit Review Top results handoff', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)

    mount(CandidateDetailWorkspacePanel, {
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
    expect(document.querySelector("[data-vue-panel='drill']")?.getAttribute('data-vue-drill')).toBe('1')
  })

  it('renders the trade table from the store model and owns shortcut selection', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    analysisStore.drillWrapVisible = true
    analysisStore.drillChartModel = markRaw(buildDrillChartModel(drillCurves, drillBars))
    await flushPanel()

    const rows = document.querySelectorAll<HTMLTableRowElement>('#tradeTableBodyVue tr[data-trade-idx]')
    expect(rows.length).toBe(2)

    rows[1]?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()
    expect(document.querySelector('#tradeTableBodyVue tr.selected')?.getAttribute('data-trade-idx')).toBe('1')

    expect(await drillClearTradeSelection()).toBeUndefined()
    await flushPanel()
    expect(document.querySelector('#tradeTableBodyVue tr.selected')).toBeNull()

    expect(await drillNextTrade()).toBeUndefined()
    await flushPanel()
    expect(document.querySelector('#tradeTableBodyVue tr.selected')?.getAttribute('data-trade-idx')).toBe('0')
  })
})
