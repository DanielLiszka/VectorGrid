import { createPinia, setActivePinia } from 'pinia'
import { enableAutoUnmount, mount } from '@vue/test-utils'
import { markRaw, nextTick } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import CandidateDetailWorkspacePanel from './CandidateDetailWorkspacePanel.vue'
import { drillClearTradeSelection, drillNextTrade } from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { buildDrillChartModel, type DrillChartPriceBar } from '../visualizations/drillChartModel'

enableAutoUnmount(afterEach)

const drillRuntimeMockState = vi.hoisted(() => ({
  options: null as null | {
    onMetaChange?: (message: string) => void
    onRenderStateChange?: (state: 'idle' | 'loading' | 'ready' | 'error') => void
    onMarkerAvailabilityChange?: (
      availability: 'unknown' | 'unavailable' | 'display-only' | 'interactive'
    ) => void
  },
  renderOutcome: 'ready' as 'ready' | 'error' | 'pending',
  markerAvailability: 'interactive' as 'unavailable' | 'display-only' | 'interactive',
  pendingRenderResolves: [] as Array<() => void>,
  renderCalls: [] as Array<{
    model: unknown
    selectedTradeIdx: number
    visibleSeriesIds: string[] | undefined
  }>,
}))

vi.mock('../composables/useDrillPriceChartRuntime', () => ({
  useDrillPriceChartRuntime: vi.fn((options = {}) => {
    drillRuntimeMockState.options = options
    return {
      setHost: vi.fn(),
      render: vi.fn(async (
        model: unknown,
        selectedTradeIdx = -1,
        visibleSeriesIds?: readonly string[],
      ) => {
        drillRuntimeMockState.renderCalls.push({
          model,
          selectedTradeIdx,
          visibleSeriesIds: visibleSeriesIds ? [...visibleSeriesIds] : undefined,
        })
        if (!model) {
          options.onRenderStateChange?.('idle')
          return
        }
        options.onRenderStateChange?.('loading')
        if (drillRuntimeMockState.renderOutcome === 'error') {
          options.onMetaChange?.('Unable to create drill chart runtime.')
          options.onRenderStateChange?.('error')
          return
        }
        if (drillRuntimeMockState.renderOutcome === 'pending') {
          await new Promise<void>((resolve) => {
            drillRuntimeMockState.pendingRenderResolves.push(resolve)
          })
        }
        options.onMarkerAvailabilityChange?.(drillRuntimeMockState.markerAvailability)
        options.onRenderStateChange?.('ready')
      }),
      updateSelectedTrade: vi.fn(),
      fitContent: vi.fn(),
      pan: vi.fn(),
      clear: vi.fn(),
      dispose: vi.fn(),
    }
  }),
}))

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

const nativeDrillCurves = {
  ...drillCurves,
  strategy_indicator_bundle: {
    strategy_kind: 'Vemr',
    aggregation: 'last_finite',
    bars: drillBars.map((bar) => ({
      time: bar.time,
      from_index: bar.fromIndex,
      to_index: bar.toIndex,
    })),
    series: [
      {
        id: 'vemr_bb_upper',
        label: 'Bollinger upper',
        kind: 'line',
        pane_id: 'price',
        scale_id: 'price',
        unit: 'price',
        default_visible: true,
        values: [101.1, 101.4, 101.6],
      },
      {
        id: 'vemr_zscore',
        label: 'Volume Z-score',
        kind: 'histogram',
        pane_id: 'vemr_zscore',
        scale_id: 'vemr_zscore',
        unit: 'z_score',
        default_visible: true,
        values: [null, 1.2, -0.4],
        reference_lines: [{ value: 0, label: 'Zero' }],
      },
      {
        id: 'vemr_atr',
        label: 'ATR',
        kind: 'line',
        pane_id: 'vemr_volatility',
        scale_id: 'vemr_volatility',
        unit: 'price_delta',
        default_visible: false,
        values: [0.7, 0.8, 0.75],
      },
      {
        id: 'vemr_signal_quality',
        label: 'Signal quality',
        kind: 'line',
        pane_id: 'vemr_zscore',
        scale_id: 'vemr_zscore',
        unit: 'ratio',
        default_visible: true,
        values: [null, null, null],
      },
    ],
  },
}

describe('CandidateDetailWorkspacePanel', () => {
  beforeEach(() => {
    installDrillPanelMountShell()
    localStorage.clear()
    drillRuntimeMockState.options = null
    drillRuntimeMockState.renderOutcome = 'ready'
    drillRuntimeMockState.markerAvailability = 'interactive'
    drillRuntimeMockState.pendingRenderResolves = []
    drillRuntimeMockState.renderCalls = []
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
    expect(document.querySelector<HTMLElement>('#drillEmptyHintVue')?.style.display).not.toBe('none')
    expect(document.querySelector('#tradeTableBodyVue')).not.toBeNull()
    expect(document.querySelector('#stabilityTableBodyVue')).not.toBeNull()
    for (const id of ['drillMetaVue', 'stabilityMetaVue', 'rejectMetaVue', 'tradeStatsVue']) {
      expect(document.querySelector(`#${id}`)?.hasAttribute('aria-live')).toBe(false)
      expect(document.querySelector(`#${id}`)?.hasAttribute('role')).toBe(false)
    }
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

  it('keeps candidate detail metadata and completion status synchronized with restored store state', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)
    analysisStore.drillWrapVisible = true
    analysisStore.drillMeta = 'SMA(17) vs ALMA(41) | bins=160'
    analysisStore.drillStatus = 'Done (cached).'
    analysisStore.stabilityWrapVisible = true
    analysisStore.stabilityMeta = 'objective=Sharpe | points=9'
    analysisStore.stabilityStatus = 'Done (cached).'
    analysisStore.stabilityRejectMeta = 'PASS PnL>=0'
    analysisStore.stabilityTableRows = [
      {
        key: '17:41',
        cells: [
          { text: '17', className: 'left mono' },
          { text: '41', className: 'left mono' },
          { text: '1.250000', className: 'mono' },
        ],
      },
    ]

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.querySelector('#drillMetaVue')?.textContent).toBe('SMA(17) vs ALMA(41) | bins=160')
    expect(document.querySelector('#drillStatusVue')?.textContent).toBe('Done (cached).')
    expect(document.querySelector<HTMLElement>('#drillEmptyHintVue')?.style.display).toBe('none')
    expect(document.querySelector<HTMLElement>('#stabilityWrapVue')?.style.display).toBe('block')
    expect(document.querySelector('#stabilityMetaVue')?.textContent).toBe('objective=Sharpe | points=9')
    expect(document.querySelector('#stabilityStatusVue')?.textContent).toBe('Done (cached).')
    expect(document.querySelector('#stabilityStatusVue')?.classList.contains('ok')).toBe(true)
    expect(document.querySelector('#rejectMetaVue')?.textContent).toBe('PASS PnL>=0')
    expect(document.querySelectorAll('#stabilityTableBodyVue tr')).toHaveLength(1)
    expect(document.querySelector('#stabilityTableBodyVue')?.textContent).toContain('1.250000')

    analysisStore.drillMeta = 'SMA(18) vs ALMA(42) | bins=160'
    analysisStore.drillStatus = 'Done.'
    analysisStore.stabilityMeta = '-'
    analysisStore.stabilityStatus = 'Stability failed: Error: unavailable'
    analysisStore.stabilityRejectMeta = '-'
    analysisStore.stabilityTableRows = []
    await flushPanel()

    expect(document.querySelector('#drillMetaVue')?.textContent).toBe('SMA(18) vs ALMA(42) | bins=160')
    expect(document.querySelector('#drillStatusVue')?.textContent).toBe('Done.')
    expect(document.querySelector<HTMLElement>('#stabilityWrapVue')?.style.display).toBe('block')
    expect(document.querySelector('#stabilityMetaVue')?.textContent).toBe('-')
    expect(document.querySelector('#stabilityStatusVue')?.textContent).toContain('Stability failed:')
    expect(document.querySelector('#stabilityStatusVue')?.classList.contains('badText')).toBe(true)
    expect(document.querySelector('#stabilityStatusVue')?.classList.contains('ok')).toBe(false)
    expect(document.querySelectorAll('#stabilityTableBodyVue tr')).toHaveLength(0)
    expect(document.querySelector<HTMLElement>('#stabilityTableWrapVue')?.style.display).toBe('none')

    analysisStore.stabilityWrapVisible = false
    analysisStore.stabilityStatus = '-'
    await flushPanel()
    expect(document.querySelector<HTMLElement>('#stabilityWrapVue')?.style.display).toBe('none')
  })

  it('keeps historical candidate context visible while presenting stale dataset detail', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)
    analysisStore.selectedTopTitle = 'Rank #1 · EMA 7 / EMA 30'
    analysisStore.drillStatus = 'Candidate detail stale/incompatible: saved result data_id dataset-a does not match loaded data_id dataset-b. Load the matching dataset; historical candidate rows remain available in Top Results.'

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('Rank #1 · EMA 7 / EMA 30')
    expect(text).toContain('Candidate detail stale/incompatible')
    expect(text).toContain('Historical candidate rows remain available in Top Results')
    expect(analysisStore.drillWrapVisible).toBe(false)
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
    expect(rows[0]?.tabIndex).toBe(0)
    expect(rows[0]?.getAttribute('aria-label')).toBe('Select trade 1')

    rows[1]?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()
    expect(document.querySelector('#tradeTableBodyVue tr.selected')?.getAttribute('data-trade-idx')).toBe('1')

    rows[0]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await flushPanel()
    expect(document.querySelector('#tradeTableBodyVue tr.selected')?.getAttribute('data-trade-idx')).toBe('0')

    expect(await drillClearTradeSelection()).toBeUndefined()
    await flushPanel()
    expect(document.querySelector('#tradeTableBodyVue tr.selected')).toBeNull()

    rows[1]?.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }))
    await flushPanel()
    expect(document.querySelector('#tradeTableBodyVue tr.selected')?.getAttribute('data-trade-idx')).toBe('1')

    expect(await drillClearTradeSelection()).toBeUndefined()
    await flushPanel()

    expect(await drillNextTrade()).toBeUndefined()
    await flushPanel()
    expect(document.querySelector('#tradeTableBodyVue tr.selected')?.getAttribute('data-trade-idx')).toBe('0')
  })

  it('keeps chart navigation disabled and reports runtime creation failure', async () => {
    drillRuntimeMockState.renderOutcome = 'error'
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)
    analysisStore.drillWrapVisible = true
    analysisStore.drillChartModel = markRaw(buildDrillChartModel(drillCurves, drillBars))

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const frame = document.querySelector<HTMLElement>('.tvChartFrame')
    expect(frame?.dataset.chartState).toBe('error')
    expect(frame?.dataset.chartReady).toBe('false')
    expect(document.querySelector('.chartStateBadge')?.textContent).toContain('Chart unavailable')
    expect(document.body.textContent || '').toContain(
      'Candidate evidence loaded, but the price chart is unavailable.',
    )
    expect(document.body.textContent || '').not.toContain(
      'Chart, trade, drawdown, and stability evidence are ready',
    )
    expect(document.querySelector('#drillTvChartMetaVue')?.textContent).toContain(
      'Unable to create drill chart runtime.',
    )
    const chartActionButtons = Array.from(
      document.querySelectorAll<HTMLButtonElement>('[data-qa-action]'),
    )
    for (const actionId of [
      'candidateChart.panLeft',
      'candidateChart.fitContent',
      'candidateChart.panRight',
    ]) {
      expect(
        chartActionButtons.find((button) => button.dataset.qaAction === actionId)?.disabled,
      ).toBe(true)
    }
  })

  it('keeps chart navigation pending until the runtime confirms rendering', async () => {
    drillRuntimeMockState.renderOutcome = 'pending'
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)
    analysisStore.drillWrapVisible = true
    analysisStore.drillChartModel = markRaw(buildDrillChartModel(drillCurves, drillBars))

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await vi.waitFor(() => {
      expect(drillRuntimeMockState.pendingRenderResolves.length).toBeGreaterThan(0)
    })
    await flushPanel()

    const frame = document.querySelector<HTMLElement>('.tvChartFrame')
    expect(frame?.dataset.chartState).toBe('loading')
    expect(frame?.dataset.chartReady).toBe('false')
    expect(document.querySelector('.chartStateBadge')?.textContent).toContain('Rendering chart')
    expect(document.body.textContent || '').toContain('the price chart is rendering')
    expect(
      document.querySelector<HTMLButtonElement>('[data-qa-action="candidateChart.fitContent"]')
        ?.disabled,
    ).toBe(true)

    drillRuntimeMockState.renderOutcome = 'ready'
    for (const resolve of drillRuntimeMockState.pendingRenderResolves.splice(0)) resolve()
    await flushPanel()

    expect(frame?.dataset.chartState).toBe('ready')
    expect(frame?.dataset.chartReady).toBe('true')
    expect(document.querySelector('.chartStateBadge')?.textContent).toContain('Chart ready')
    expect(
      document.querySelector<HTMLButtonElement>('[data-qa-action="candidateChart.fitContent"]')
        ?.disabled,
    ).toBe(false)
  })

  it('keeps a marker-less chart navigable while directing trade review to the table', async () => {
    drillRuntimeMockState.markerAvailability = 'unavailable'
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)
    analysisStore.drillWrapVisible = true
    analysisStore.drillChartModel = markRaw(buildDrillChartModel(drillCurves, drillBars))

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const frame = document.querySelector<HTMLElement>('.tvChartFrame')
    expect(frame?.dataset.chartState).toBe('warning')
    expect(frame?.dataset.chartReady).toBe('true')
    expect(document.body.textContent || '').toContain('Trade markers are unavailable')
    expect(document.querySelector('#drillKeyboardHint')?.textContent).toContain(
      'use the Trades table',
    )
    expect(document.querySelector('#drillKeyboardHint')?.textContent).not.toContain(
      'click chart markers',
    )
    const seriesKeys = Array.from(
      document.querySelectorAll<HTMLElement>('[data-chart-series]'),
      (element) => element.dataset.chartSeries,
    )
    expect(seriesKeys).not.toContain('entries')
    expect(seriesKeys).not.toContain('exits')
    expect(
      document.querySelector<HTMLButtonElement>('[data-qa-action="candidateChart.fitContent"]')
        ?.disabled,
    ).toBe(false)
  })

  it('retains display-only marker counts without advertising marker clicks', async () => {
    drillRuntimeMockState.markerAvailability = 'display-only'
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)
    analysisStore.drillWrapVisible = true
    analysisStore.drillChartModel = markRaw(buildDrillChartModel(drillCurves, drillBars))

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.querySelector<HTMLElement>('.tvChartFrame')?.dataset.chartState).toBe('warning')
    expect(document.body.textContent || '').toContain(
      'Trade markers are visible, but direct marker selection is unavailable.',
    )
    expect(document.querySelector('#drillKeyboardHint')?.textContent).not.toContain(
      'click chart markers',
    )
    const seriesKeys = Array.from(
      document.querySelectorAll<HTMLElement>('[data-chart-series]'),
      (element) => element.dataset.chartSeries,
    )
    expect(seriesKeys).toEqual(expect.arrayContaining(['entries', 'exits']))
  })

  it('groups native strategy indicators, applies finite defaults, and rerenders accessible toggles', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)
    analysisStore.drillWrapVisible = true
    analysisStore.drillChartModel = markRaw(buildDrillChartModel(nativeDrillCurves, drillBars))

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const controls = document.querySelector<HTMLDetailsElement>(
      'details[data-testid="native-indicator-controls"][data-native-strategy="Vemr"]',
    )
    expect(controls).not.toBeNull()
    expect(controls?.open).toBe(false)
    expect(controls?.getAttribute('aria-label')).toBe('Strategy indicators')
    expect(controls?.querySelector('summary')?.textContent).toContain(
      '2 shown · 3 available · 4 total',
    )
    const paneLabels = Array.from(
      controls?.querySelectorAll('fieldset legend') ?? [],
      (legend) => legend.textContent?.trim(),
    )
    expect(paneLabels).toEqual(['Price pane', 'Z-score pane', 'Volatility pane'])
    expect(
      controls?.querySelector('fieldset[data-indicator-pane="vemr_zscore"]'),
    ).not.toBeNull()

    const upperInput = controls?.querySelector<HTMLInputElement>(
      '[data-indicator-series-id="vemr_bb_upper"]',
    )
    const zscoreInput = controls?.querySelector<HTMLInputElement>(
      '[data-indicator-series-id="vemr_zscore"]',
    )
    const atrInput = controls?.querySelector<HTMLInputElement>(
      '[data-indicator-series-id="vemr_atr"]',
    )
    const unavailableInput = controls?.querySelector<HTMLInputElement>(
      '[data-indicator-series-id="vemr_signal_quality"]',
    )
    expect(upperInput?.checked).toBe(true)
    expect(zscoreInput?.checked).toBe(true)
    expect(atrInput?.checked).toBe(false)
    expect(unavailableInput?.checked).toBe(false)
    expect(unavailableInput?.disabled).toBe(true)
    expect(unavailableInput?.getAttribute('aria-describedby')).toBe(
      'drillIndicatorUnavailableSummary',
    )
    expect(upperInput?.getAttribute('aria-label')).toBe('Bollinger upper')
    for (const input of [upperInput, zscoreInput, atrInput, unavailableInput]) {
      expect(input?.getAttribute('aria-controls')).toBe('drillTvChartVue')
    }
    expect(document.querySelector('#drillIndicatorUnavailableSummary')?.textContent).toContain(
      '1 series is unavailable because this candidate returned no finite points.',
    )

    const frame = document.querySelector<HTMLElement>('.tvChartFrame')
    expect(frame?.dataset.nativeStrategy).toBe('Vemr')
    expect(frame?.dataset.nativeSeriesVisible).toBe('2')
    expect(frame?.dataset.nativeSeriesTotal).toBe('4')
    expect(frame?.dataset.nativePaneCount).toBe('2')
    const nativeLegendText = document.querySelector('.chartLegendOverlay')?.textContent ?? ''
    const nativeEvidenceText = document.querySelector('.chartEvidenceChips')?.textContent ?? ''
    expect(nativeLegendText).toContain('Bollinger upper')
    expect(nativeLegendText).toContain('Volume Z-score')
    expect(nativeLegendText).not.toContain('ATR')
    expect(nativeLegendText).not.toContain('Fast MA')
    expect(nativeLegendText).not.toContain('Slow MA')
    expect(nativeEvidenceText).toContain('Bollinger upper')
    expect(nativeEvidenceText).toContain('Volume Z-score')
    expect(nativeEvidenceText).not.toContain('MA overlays')

    const initialRender = drillRuntimeMockState.renderCalls[
      drillRuntimeMockState.renderCalls.length - 1
    ]
    expect(initialRender?.visibleSeriesIds).toEqual(['vemr_bb_upper', 'vemr_zscore'])

    if (!atrInput) throw new Error('Expected the ATR checkbox.')
    atrInput.checked = true
    atrInput.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()

    const toggledRender = drillRuntimeMockState.renderCalls[
      drillRuntimeMockState.renderCalls.length - 1
    ]
    expect(toggledRender?.visibleSeriesIds).toEqual([
      'vemr_bb_upper',
      'vemr_zscore',
      'vemr_atr',
    ])
    expect(document.querySelector('.chartLegendOverlay')?.textContent).toContain('ATR')
    expect(frame?.dataset.nativeSeriesVisible).toBe('3')
    expect(frame?.dataset.nativePaneCount).toBe('3')
  })

  it('resets native visibility for each candidate model and preserves the legacy chart presentation', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)
    analysisStore.drillWrapVisible = true
    analysisStore.drillChartModel = markRaw(buildDrillChartModel(nativeDrillCurves, drillBars))

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const atrInput = document.querySelector<HTMLInputElement>(
      '[data-indicator-series-id="vemr_atr"]',
    )
    if (!atrInput) throw new Error('Expected the ATR checkbox.')
    atrInput.checked = true
    atrInput.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()

    analysisStore.drillChartModel = markRaw(buildDrillChartModel({
      ...drillCurves,
      strategy_indicator_bundle: {
        strategy_kind: 'Rocb',
        aggregation: 'last_finite',
        bars: drillBars.map((bar) => ({
          time: bar.time,
          from_index: bar.fromIndex,
          to_index: bar.toIndex,
        })),
        series: [
          {
            id: 'rocb_channel_upper',
            label: 'Donchian upper',
            kind: 'line',
            pane_id: 'price',
            scale_id: 'price',
            unit: 'price',
            default_visible: true,
            values: [101.2, 101.5, 101.7],
          },
          {
            id: 'rocb_roc',
            label: 'Rate of change',
            kind: 'histogram',
            pane_id: 'rocb_momentum',
            scale_id: 'rocb_momentum',
            unit: 'oscillator',
            default_visible: false,
            values: [0.1, 0.3, -0.2],
          },
        ],
      },
    }, drillBars))
    await flushPanel()

    expect(document.querySelector('[data-indicator-series-id="vemr_atr"]')).toBeNull()
    expect(
      document.querySelector<HTMLInputElement>('[data-indicator-series-id="rocb_channel_upper"]')
        ?.checked,
    ).toBe(true)
    expect(
      document.querySelector<HTMLInputElement>('[data-indicator-series-id="rocb_roc"]')?.checked,
    ).toBe(false)
    const resetRender = drillRuntimeMockState.renderCalls[
      drillRuntimeMockState.renderCalls.length - 1
    ]
    expect(resetRender?.visibleSeriesIds).toEqual(['rocb_channel_upper'])
    expect(document.querySelector<HTMLElement>('.tvChartFrame')?.dataset.nativeStrategy).toBe('Rocb')

    analysisStore.drillChartModel = markRaw(buildDrillChartModel(drillCurves, drillBars))
    await flushPanel()

    expect(document.querySelector('[data-testid="native-indicator-controls"]')).toBeNull()
    const legacyFrame = document.querySelector<HTMLElement>('.tvChartFrame')
    expect(legacyFrame?.hasAttribute('data-native-strategy')).toBe(false)
    expect(document.querySelector('.chartTitle')?.textContent).toContain('Price + trades')
    const legacyLegend = document.querySelector('.chartLegendOverlay')?.textContent ?? ''
    expect(legacyLegend).toContain('Fast MA')
    expect(legacyLegend).toContain('Slow MA')
    const legacyRender = drillRuntimeMockState.renderCalls[
      drillRuntimeMockState.renderCalls.length - 1
    ]
    expect(legacyRender?.visibleSeriesIds).toBeUndefined()
  })

  it('shows selected-candidate context and navigable trade evidence', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)
    analysisStore.selectedTopTitle = 'Rank #2 · SMA 12 / EMA 48'
    analysisStore.selectedTopSummary = 'Selected candidate 2 of 4: Sharpe 1.7 · MaxDD 0.08.'
    analysisStore.selectedTopDecisionNote = 'Candidate is ready for detailed review.'
    analysisStore.selectedTopProvenanceChips = ['Filtered shortlist', 'Risk view']

    mount(CandidateDetailWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    analysisStore.drillWrapVisible = true
    analysisStore.drillChartModel = markRaw(buildDrillChartModel(drillCurves, drillBars))
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('Current review target')
    expect(text).toContain('Rank #2 · SMA 12 / EMA 48')
    expect(text).toContain('Filtered shortlist')
    expect(text).toContain('3 price bars')
    expect(text).toContain('Win rate')
    expect(text).toContain('Profit factor')
    const chartSeriesKeys = Array.from(
      document.querySelectorAll<HTMLElement>('[data-chart-series]'),
      (element) => element.dataset.chartSeries,
    )
    expect(chartSeriesKeys).toEqual(expect.arrayContaining([
      'candles',
      'fast',
      'slow',
      'entries',
      'exits',
    ]))

    const nextTradeButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Next trade'),
    )
    nextTradeButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()
    expect(document.querySelector('#tradeTableBodyVue tr.selected')?.getAttribute('data-trade-idx')).toBe('0')
    expect(document.body.textContent || '').toContain('Selected trade 1.')
  })
})
