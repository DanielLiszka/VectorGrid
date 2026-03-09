// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import {
  clearBaseline,
  clearTopFilters,
  drillClearTradeSelection,
  drillNextTrade,
  drillPrevTrade,
  refreshAnalysisFromWorkflowRun,
  refreshParetoFromWorkflowRun,
  refreshTopTableFromWorkflowRun,
  rerenderDrill,
  selectTopRow,
  setBaseline,
  sortTopTable,
} from './analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useHistoryStore } from '../stores/historyStore'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => ({
    ts: '2026-02-26T11:59:00.000Z',
    run: {
      req: { strategy_kind: 'DoubleMa', objective: 'Sharpe' },
      res: {
        total_evaluations: 2,
        runtime_ms: 10,
        best_params: { fast_len: 5, slow_len: 20 },
        best_metrics: { pnl: 0.2, sharpe: 1.1, max_dd: 0.1, trades: 10, exposure: 0.4 },
      },
    },
  })),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

function seedWorkflowRun(payload: { ts?: string; artifactPath?: string; req?: unknown; res?: unknown; validationRes?: unknown }): void {
  const pinia = createPinia()
  setActivePinia(pinia)
  const root = pinia.state.value as Record<string, unknown>
  root.workflowArtifacts = {
    currentRun: {
      ts: payload.ts || '2026-02-26T12:00:00.000Z',
      artifactPath: payload.artifactPath || '',
      req: payload.req ?? null,
      res: payload.res ?? null,
    },
    currentValidation: {
      res: payload.validationRes ?? null,
    },
  }
}

describe('analysis service', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
    vi.clearAllMocks()
    if (typeof document !== 'undefined') document.body.innerHTML = ''
    if (typeof HTMLCanvasElement !== 'undefined') {
      vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(() => ({
        clearRect: () => undefined,
        fillRect: () => undefined,
        strokeRect: () => undefined,
        beginPath: () => undefined,
        arc: () => undefined,
        fill: () => undefined,
        stroke: () => undefined,
        fillText: () => undefined,
        moveTo: () => undefined,
        lineTo: () => undefined,
        save: () => undefined,
        restore: () => undefined,
        translate: () => undefined,
        rotate: () => undefined,
        createLinearGradient: () => ({ addColorStop: () => undefined }),
      }) as never)
    }
    invokeCommandMock.mockResolvedValue({
      ts: '2026-02-26T11:59:00.000Z',
      run: {
        req: { strategy_kind: 'DoubleMa', objective: 'Sharpe' },
        res: {
          total_evaluations: 2,
          runtime_ms: 10,
          best_params: { fast_len: 5, slow_len: 20 },
          best_metrics: { pnl: 0.2, sharpe: 1.1, max_dd: 0.1, trades: 10, exposure: 0.4 },
        },
      },
    } as never)
  })

  it('sorts top table rows in Vue store state', async () => {
    seedWorkflowRun({
      req: { strategy_kind: 'DoubleMa', objective: 'Sharpe' },
      res: {
        top: [
          [{ fast_ma_type: 'ema', fast_len: 5, slow_ma_type: 'ema', slow_len: 20 }, { pnl: 0.1, sharpe: 0.5, max_dd: 0.2, trades: 10, exposure: 0.4 }],
          [{ fast_ma_type: 'ema', fast_len: 7, slow_ma_type: 'ema', slow_len: 30 }, { pnl: 0.2, sharpe: 1.2, max_dd: 0.1, trades: 20, exposure: 0.5 }],
          [{ fast_ma_type: 'ema', fast_len: 9, slow_ma_type: 'ema', slow_len: 40 }, { pnl: 0.15, sharpe: 0.8, max_dd: 0.12, trades: 15, exposure: 0.45 }],
        ],
      },
    })
    const store = useAnalysisStore()

    await sortTopTable(' sharpe ')

    const sharpeCol = store.topColumns.find((column) => column.key === 'sharpe')
    expect(String(sharpeCol?.sort || '')).toBe('desc')
    expect(String(store.topTableRows[0]?.cells?.[5]?.text || '')).toBe('1.2')
    expect(String(store.topTableRows[1]?.cells?.[5]?.text || '')).toBe('0.8')
    expect(String(store.topTableRows[2]?.cells?.[5]?.text || '')).toBe('0.5')
    expect(store.topTableRows[0]?.selected).toBe(true)
    expect(store.source).toBe('vue')
  })

  it('selects top row in Vue store state when row model is unavailable', async () => {
    const store = useAnalysisStore()
    store.topTableRows = [
      { selected: false, cells: [{ text: 'A', className: '', title: '' }] },
      { selected: false, cells: [{ text: 'B', className: '', title: '' }] },
      { selected: false, cells: [{ text: 'C', className: '', title: '' }] },
    ]

    await selectTopRow(3.9)

    expect(store.topTableRows[0]?.selected).toBe(false)
    expect(store.topTableRows[1]?.selected).toBe(false)
    expect(store.topTableRows[2]?.selected).toBe(true)
    expect(String(store.topSelectionGuardrail || '')).toContain('3')
    expect(store.source).toBe('vue')
  })

  it('computes drilldown/stability via Vue command path on top-row selection', async () => {
    seedWorkflowRun({
      req: {
        strategy_kind: 'DoubleMa',
        data_id: 'dataset-1',
        ma_source: 'close',
        objective: 'Sharpe',
        strategy: {},
        halftrend_filter: null,
      },
      res: {
        top: [
          [{ fast_ma_type: 'ema', fast_len: 5, slow_ma_type: 'ema', slow_len: 20 }, { pnl: 0.1, sharpe: 0.5, max_dd: 0.2, trades: 10, exposure: 0.4 }],
          [{ fast_ma_type: 'ema', fast_len: 7, slow_ma_type: 'ema', slow_len: 30 }, { pnl: 0.2, sharpe: 1.2, max_dd: 0.1, trades: 20, exposure: 0.5 }],
        ],
      },
    })
    const store = useAnalysisStore()
    refreshTopTableFromWorkflowRun()
    invokeCommandMock.mockImplementation(async (command: unknown) => {
      if (command === 'compute_double_ma_drilldown') {
        return {
          equity: { start_t: 1, end_t: 2, bins: 1, minmax: [1, 1] },
          drawdown: { start_t: 1, end_t: 2, bins: 1, minmax: [0, 0] },
          trades: [],
          trade_stats: null,
          indicator_bars: [],
        } as never
      }
      if (command === 'compute_double_ma_local_stability') {
        return {
          base: { fast_len: 7, slow_len: 30, fast_ma_type: 'ema', slow_ma_type: 'ema' },
          objective: 'Sharpe',
          radius: 1,
          step: 1,
          start_t: 1,
          points: [],
          summary: { base: 0, min: 0, median: 0, max: 0 },
        } as never
      }
      return {
        ts: '2026-02-26T11:59:00.000Z',
        run: {
          req: { strategy_kind: 'DoubleMa', objective: 'Sharpe' },
          res: {
            total_evaluations: 2,
            runtime_ms: 10,
            best_params: { fast_len: 5, slow_len: 20 },
            best_metrics: { pnl: 0.2, sharpe: 1.1, max_dd: 0.1, trades: 10, exposure: 0.4 },
          },
        },
      } as never
    })

    await selectTopRow(1)

    expect(invokeCommandMock).toHaveBeenCalledWith('compute_double_ma_drilldown', expect.any(Object))
    expect(invokeCommandMock).toHaveBeenCalledWith('compute_double_ma_local_stability', expect.any(Object))
    expect(store.topTableRows[1]?.selected).toBe(true)
  })

  it('clears top filters in Vue store state', async () => {
    const store = useAnalysisStore()
    store.filterMinPnl = '1.0'
    store.filterMinSharpe = '0.5'
    store.filterMaxDd = '20'
    store.filterMinTrades = '50'
    store.filterMinExposure = '0.1'
    store.canClearTopFilters = true

    await clearTopFilters()

    expect(store.filterMinPnl).toBe('')
    expect(store.filterMinSharpe).toBe('')
    expect(store.filterMaxDd).toBe('')
    expect(store.filterMinTrades).toBe('')
    expect(store.filterMinExposure).toBe('')
    expect(store.canClearTopFilters).toBe(false)
    expect(store.source).toBe('vue')
  })

  it('rebuilds top table rows from workflow run artifacts', () => {
    seedWorkflowRun({
      req: { strategy_kind: 'DoubleMa', objective: 'Sharpe' },
      res: {
        top: [
          [{ fast_ma_type: 'ema', fast_len: 5, slow_ma_type: 'ema', slow_len: 20 }, { pnl: 0.3, sharpe: 1.8, max_dd: 0.1, trades: 40, exposure: 0.5 }],
          [{ fast_ma_type: 'sma', fast_len: 7, slow_ma_type: 'sma', slow_len: 21 }, { pnl: 0.1, sharpe: 1.2, max_dd: 0.2, trades: 22, exposure: 0.4 }],
        ],
      },
    })
    const store = useAnalysisStore()
    store.filterMinSharpe = '1.5'
    const ok = refreshTopTableFromWorkflowRun()
    expect(ok).toBe(true)
    expect(store.topRows).toBe(1)
    expect(String(store.topFilterMeta || '')).toBe('showing 1/2 rows')
    expect(Array.isArray(store.topColumns)).toBe(true)
    expect(Array.isArray(store.topTableRows)).toBe(true)
  })

  it('does not fabricate ranked rows when top_k is disabled for the run', () => {
    seedWorkflowRun({
      req: { strategy_kind: 'DoubleMa', objective: 'Sharpe', top_k: 0 },
      res: {
        num_combos: 136605,
        best_params: { fast_ma_type: 'sma', fast_len: 8, slow_ma_type: 'alma', slow_len: 20 },
        best_metrics: { pnl: 0.3, sharpe: 1.8, max_dd: 0.1, trades: 40, exposure: 0.5 },
        top: [],
      },
    })
    const store = useAnalysisStore()

    const ok = refreshTopTableFromWorkflowRun()

    expect(ok).toBe(true)
    expect(store.topRows).toBe(0)
    expect(String(store.topFilterMeta || '')).toContain('Top K disabled')
    expect(String(store.topSelectionGuardrail || '')).toContain('Top K above 0')
  })

  it('marks heatmap as disabled when heatmap_bins is 0', () => {
    seedWorkflowRun({
      req: { strategy_kind: 'DoubleMa', objective: 'Sharpe', top_k: 20, heatmap_bins: 0 },
      res: {
        best_params: { fast_ma_type: 'sma', fast_len: 8, slow_ma_type: 'alma', slow_len: 20 },
        best_metrics: { pnl: 0.3, sharpe: 1.8, max_dd: 0.1, trades: 40, exposure: 0.5 },
      },
    })
    const store = useAnalysisStore()

    const ok = refreshAnalysisFromWorkflowRun()

    expect(ok).toBe(true)
    expect(store.heatmapWrapVisible).toBe(false)
    expect(store.canHeatmapExportPng).toBe(false)
    expect(String(store.heatmapMeta || '')).toContain('heatmap_bins=0')
    expect(String(store.heatmapStatusLine || '')).toContain('Heatmap bins')
  })

  it('renders pareto scatter into the Vue canvas host when mounted', () => {
    document.body.innerHTML = `
      <div id="paretoWrapVue" style="display:none"></div>
      <div id="paretoMetaVue">-</div>
      <canvas id="paretoCanvasVue" width="1024" height="240"></canvas>
    `
    seedWorkflowRun({
      req: { strategy_kind: 'DoubleMa', objective: 'Sharpe', top_k: 20 },
      res: {
        top: [
          [{ fast_ma_type: 'sma', fast_len: 5, slow_ma_type: 'alma', slow_len: 20 }, { pnl: -0.1, sharpe: -0.2, max_dd: 0.4, trades: 10, exposure: 0.4 }],
          [{ fast_ma_type: 'sma', fast_len: 8, slow_ma_type: 'alma', slow_len: 24 }, { pnl: 0.05, sharpe: 0.3, max_dd: 0.2, trades: 18, exposure: 0.5 }],
        ],
      },
    })
    const store = useAnalysisStore()

    const ok = refreshParetoFromWorkflowRun()

    expect(ok).toBe(true)
    expect(store.paretoWrapVisible).toBe(true)
    expect(String(store.paretoMeta || '')).toContain('2 points')
    expect((document.querySelector('#paretoWrapVue') as HTMLElement | null)?.style.display).toBe('block')
  })

  it('sets and clears baseline from Vue stores when run context is available', async () => {
    seedWorkflowRun({
      ts: '2026-02-26T12:01:00.000Z',
      artifactPath: 'C:/runs/run_a.json',
      req: { objective: 'Sharpe' },
      res: { top: [] },
    })
    const analysisStore = useAnalysisStore()
    const historyStore = useHistoryStore()

    await setBaseline()
    expect(historyStore.compareSelectedValue).toBe('disk:C:/runs/run_a.json')
    expect(String(analysisStore.statusLine || '')).toContain('Set baseline')
    expect(String(analysisStore.compareSummary || '')).toContain('Optimization best:')
    expect(invokeCommandMock).toHaveBeenCalledWith('load_run_artifact', { path: 'C:/runs/run_a.json' })

    analysisStore.compareSummary = 'placeholder'
    await clearBaseline()
    expect(historyStore.compareSelectedValue).toBe('')
    expect(analysisStore.compareSummary).toBe('-')
  })

  it('is a no-op when Vue rows/columns are unavailable', async () => {
    const store = useAnalysisStore()
    store.topColumns = []
    store.topTableRows = []

    await sortTopTable('sharpe')
    await selectTopRow(2)

    expect(Array.isArray(store.topColumns)).toBe(true)
    expect(Array.isArray(store.topTableRows)).toBe(true)
  })

  it('rerenders drill by selecting current or first Vue top row', async () => {
    const store = useAnalysisStore()
    store.topTableRows = [
      { selected: false, cells: [{ text: 'A', className: '', title: '' }] },
      { selected: true, cells: [{ text: 'B', className: '', title: '' }] },
      { selected: false, cells: [{ text: 'C', className: '', title: '' }] },
    ]

    await rerenderDrill()

    expect(store.topTableRows[0]?.selected).toBe(false)
    expect(store.topTableRows[1]?.selected).toBe(true)
    expect(store.topTableRows[2]?.selected).toBe(false)
  })

  it('keeps drill navigation non-throwing when DOM surface is unavailable', async () => {
    await drillNextTrade()
    await drillPrevTrade()
  })

  it('keeps drill selection clear non-throwing when DOM surface is unavailable', async () => {
    await drillClearTradeSelection()
    expect(true).toBe(true)
  })
})
