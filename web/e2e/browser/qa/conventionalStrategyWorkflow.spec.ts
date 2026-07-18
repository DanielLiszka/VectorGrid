import { expect, test, type Page } from '@playwright/test'
import fs from 'node:fs/promises'
import path from 'node:path'
import { emptyAppScenario, type MockTauriScenario } from '../fixtures'
import { expectNoHorizontalDocumentOverflow, expectNoStuckBusyState, installQaGuards } from '../support/qaAssertions'
import { gotoMockedShellWithScenario, readTauriInvokeLog } from '../support/workflowRunner'

type OptimizationCommand =
  | 'run_double_ma_optimization'
  | 'run_composite_cpu_optimization'
  | 'run_composite_mo_optimization'

type ValidationCommand = 'run_double_ma_validation' | 'run_composite_validation'

interface ConventionalStrategyCase {
  kind: string
  label: string
  optimizationCommand: OptimizationCommand
  validationCommand: ValidationCommand
  expectsHeatmap: boolean
  expectsPareto?: boolean
  expectedValidationBlock?: RegExp
  setup?: (page: Page) => Promise<void>
}

const DATASET_META = {
  csv_path: 'C:/fixtures/aapl_1m.csv',
  timeframe: '1m',
  len: 5000,
  symbol: 'AAPL',
  label: 'AAPL fixture',
  has_open: true,
  has_high: true,
  has_low: true,
  has_close: true,
  has_volume: true,
  min_ts: 1904164200000,
  max_ts: 1904250600000,
}

const DOUBLE_PARAMS = {
  fast_len: 8,
  slow_len: 32,
  fast_ma_type: 'sma',
  slow_ma_type: 'sma',
}

const COMPOSITE_PARAMS = {
  fast_len: 9,
  slow_len: 34,
  fast_ma_type: 'ema',
  slow_ma_type: 'sma',
}

const BASE_METRICS = {
  pnl: 0.12,
  sharpe: 1.4,
  max_dd: -0.05,
  trades: 18,
  exposure: 0.52,
  net_exposure: 0.08,
}

const DRILL_BAR_COUNT = 72
const DRILL_START_SEC = 1_744_500_000

const PRICE_BARS = Array.from({ length: DRILL_BAR_COUNT }, (_, index) => {
  const time = DRILL_START_SEC + index * 60
  const base = 100 + Math.sin(index / 5) * 1.3 + index * 0.015
  const open = Number((base + Math.sin(index / 3) * 0.16).toFixed(3))
  const close = Number((base + Math.cos(index / 4) * 0.2).toFixed(3))
  return {
    time,
    open,
    high: Number((Math.max(open, close) + 0.42).toFixed(3)),
    low: Number((Math.min(open, close) - 0.4).toFixed(3)),
    close,
    from_index: index + 1,
    to_index: index + 1,
  }
})

const BASE_DRILL_CURVES = {
  equity: { start_t: 1, end_t: 72, bins: 6, minmax: [1, 1.02, 1.01, 1.05, 1.03, 1.08] },
  drawdown: { start_t: 1, end_t: 72, bins: 6, minmax: [0, -0.01, -0.02, -0.01, -0.015, -0.025] },
  trades: [
    { entry_t: 4, exit_t: 14, direction: 1, pnl: 0.03, bars: 10, open: false },
    { entry_t: 22, exit_t: 33, direction: -1, pnl: -0.01, bars: 11, open: false },
    { entry_t: 45, exit_t: 62, direction: 1, pnl: 0.018, bars: 17, open: false },
  ],
  trade_stats: {
    closed_trades: 3,
    open_trades: 0,
    wins: 2,
    losses: 1,
    win_rate: 0.667,
    avg_win: 0.024,
    avg_loss: -0.01,
    profit_factor: 4.8,
    expectancy: 0.0127,
  },
  indicator_bars: PRICE_BARS.map((bar, index) => ({
    time: bar.time,
    from_index: index + 1,
    to_index: index + 1,
    fast: Number((bar.close + Math.sin(index / 5) * 0.2).toFixed(3)),
    slow: Number((bar.close + Math.cos(index / 8) * 0.3 - 0.08).toFixed(3)),
  })),
}

const COMPOSITE_DRILL_CURVES = {
  ...BASE_DRILL_CURVES,
  composite_indicator_bars: PRICE_BARS.map((bar, index) => ({
    time: bar.time,
    from_index: index + 1,
    to_index: index + 1,
    halftrend: index % 10 < 5 ? 0 : 1,
    macz_hist: Number((Math.sin(index / 6) * 0.58).toFixed(3)),
    ott_line: Number((bar.close - 0.45 + Math.cos(index / 9) * 0.16).toFixed(3)),
  })),
}

const STRATEGIES: ConventionalStrategyCase[] = [
  {
    kind: 'DoubleMa',
    label: 'Double MA',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'DynamicDoubleMa',
    label: 'Dynamic Double MA',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
    setup: async (page) => {
      await page.locator('#vueBackend').selectOption('GpuKernel')
      await page.locator('#vueFastType').selectOption('sma')
      await page.locator('#vueSlowType').selectOption('ema')
      await page.locator('#vueDynamicStrategyIrJson').fill('{"profile":"double_ma"}')
    },
  },
  {
    kind: 'Pit',
    label: 'Pressure Impulse Trend',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Set',
    label: 'Slope + Efficiency Trend',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Mts',
    label: 'Momentum Triple Stack',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Orc',
    label: 'Oscillator Reversion Confluence',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Marrt',
    label: 'MA Ribbon Regime Trend',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Vfct',
    label: 'Volume Flow Confirmation Trend',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
    setup: async (page) => {
      await page.locator('#vueFastType').selectOption('vwma')
      await page.locator('#vueSlowType').selectOption('vwma')
    },
  },
  {
    kind: 'CompositeTrend',
    label: 'Composite Trend',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'RegimeHybrid',
    label: 'Regime Hybrid',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'BreakoutMomentum',
    label: 'Breakout Momentum',
    optimizationCommand: 'run_double_ma_optimization',
    validationCommand: 'run_double_ma_validation',
    expectsHeatmap: true,
  },
  {
    kind: 'Rocb',
    label: 'Risk Overlay Channel Breakout',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'Vcet',
    label: 'VIDYA Chandelier Exit Trend',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'Kqsr',
    label: 'KDJ QQE Stochastic Reversion',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'Carm',
    label: 'Coppock AO Relative Momentum',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'Kvdf',
    label: 'Keltner VWAP Directional Flow',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'Mkcs',
    label: 'Market Kurtosis Compression Setup',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'Cdmc',
    label: 'Cycle Decycler MAMA Composite',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'Asr',
    label: 'Adaptive Squeeze Regime',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'Vcc',
    label: 'Volatility Compression Confirmation',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'Vemr',
    label: 'Volatility Expansion Mean Reversion',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'CompositeCpu',
    label: 'Composite CPU',
    optimizationCommand: 'run_composite_cpu_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
  },
  {
    kind: 'CompositeMo',
    label: 'Composite MO',
    optimizationCommand: 'run_composite_mo_optimization',
    validationCommand: 'run_composite_validation',
    expectsHeatmap: false,
    expectsPareto: true,
  },
]

function action(page: Page, id: string) {
  return page.locator(`[data-qa-action="${id}"]`)
}

function sanitizeFilePart(value: string): string {
  return String(value || '').replace(/[^a-z0-9_-]+/gi, '_').toLowerCase()
}

function buildHeatmapFixture(bins = 16): Record<string, unknown> {
  const values = Array.from({ length: bins * bins }, (_, index) => {
    const logicalRow = Math.floor(index / bins)
    const col = index % bins
    if (logicalRow <= col) return null
    const ridge = Math.cos((logicalRow - col) / 4) * 0.16
    const slope = (logicalRow / Math.max(1, bins - 1)) * 0.11 - (col / Math.max(1, bins - 1)) * 0.05
    return Number((ridge + slope).toFixed(4))
  })
  return {
    bins_fast: bins,
    bins_slow: bins,
    fast_min: 5,
    fast_max: 20,
    slow_min: 20,
    slow_max: 35,
    values,
  }
}

function buildHeatmapSliceRows(): { all: unknown[]; all_scores: number[] } {
  const all: unknown[] = []
  const all_scores: number[] = []
  for (const offset of [0.75, 0.85]) {
    for (let fast = 5; fast <= 18; fast += 1) {
      for (let slow = Math.max(20, fast + 1); slow <= 35; slow += 2) {
        const score = Number((
          Math.cos((slow - fast) / 5) * 0.16
          + (offset === 0.75 ? 0.08 : -0.03)
          + slow / 360
          - fast / 520
        ).toFixed(4))
        all.push([
          {
            fast_len: fast,
            slow_len: slow,
            fast_ma_type: 'alma',
            slow_ma_type: 'sma',
            fast_ma_params: { offset, sigma: 6 },
            slow_ma_params: null,
          },
          {
            pnl: score * 1000,
            sharpe: score,
            max_dd: Math.max(0.01, 0.12 - score / 10),
            trades: 24,
            exposure: 0.5,
            net_exposure: 0.1,
          },
        ])
        all_scores.push(score)
      }
    }
  }
  return { all, all_scores }
}

function buildDoubleMaRunResult(): Record<string, unknown> {
  const sliceRows = buildHeatmapSliceRows()
  return {
    best_params: DOUBLE_PARAMS,
    best_metrics: BASE_METRICS,
    top: [
      [DOUBLE_PARAMS, BASE_METRICS],
      [
        { fast_len: 10, slow_len: 36, fast_ma_type: 'ema', slow_ma_type: 'sma' },
        { ...BASE_METRICS, pnl: 0.1, sharpe: 1.22, max_dd: -0.06, trades: 16 },
      ],
    ],
    heatmap: buildHeatmapFixture(16),
    all: sliceRows.all,
    all_scores: sliceRows.all_scores,
    runtime_ms: 84,
    backend_used: 'cpu',
    backend_route: 'CpuOnly',
    followup_cache_key: 'qa-double-family-cache',
  }
}

function buildCompositeCpuRunResult(): Record<string, unknown> {
  return {
    best_params: COMPOSITE_PARAMS,
    best_metrics: BASE_METRICS,
    top: [
      [COMPOSITE_PARAMS, BASE_METRICS],
      [
        { fast_len: 11, slow_len: 39, fast_ma_type: 'ema', slow_ma_type: 'sma' },
        { ...BASE_METRICS, pnl: 0.09, sharpe: 1.18, max_dd: -0.07, trades: 15 },
      ],
    ],
    runtime_ms: 96,
    backend_used: 'cpu',
    backend_route: 'CompositeCpu',
    followup_cache_key: 'qa-composite-cpu-cache',
  }
}

function buildCompositeMoRunResult(): Record<string, unknown> {
  return {
    strategy_kind: 'CompositeMo',
    backend_route: 'CompositeMo',
    runtime_ms: 112,
    pareto: [
      {
        params: {
          fast_len: 8,
          slow_len: 31,
          fast_ma_type: 'ema',
          slow_ma_type: 'sma',
          signal_mode: 'entry_core_exit_overlay',
          halftrend: { amplitude: 9, atr_period: 21, channel_deviation: 1.6 },
          macz: { fast_length: 12, slow_length: 26, signal_length: 9 },
          ott: { period: 6, percent: 1.3, ma_type: 'EMA' },
          strategy: { long_only: false, allow_flip: false },
        },
        metrics: BASE_METRICS,
        maximize_value: 1.4,
        minimize_value: 0.05,
        violation: 0,
      },
    ],
  }
}

function buildValidationResult(route: string): Record<string, unknown> {
  return {
    backend_used: 'Cpu',
    backend_route: route,
    backend_fallback_reason: '',
    runtime_ms: 42,
    candidates: [
      {
        params: DOUBLE_PARAMS,
        in_sample: BASE_METRICS,
        robust_score: 1.21,
        psr: 0.91,
        dsr: 0.84,
        robust_pareto: true,
        validation_verdict: {
          passed: true,
          approved: true,
          final_recommendation: 'approved',
        },
      },
    ],
    cross_ticker: { datasets: [] },
    run_verdict: {
      passed: true,
      approved: true,
      candidates_tested: 1,
      candidates_survived_hard_gates: 1,
      approved_candidate_count: 1,
      first_approved_rank: 1,
      final_recommendation: 'approved',
    },
  }
}

function conventionalScenario(): MockTauriScenario {
  return {
    ...emptyAppScenario,
    name: 'conventional-strategy-workflow-ready',
    family: 'optimization',
    description: 'Every conventional strategy can load data, optimize, inspect visuals, validate, and enter analysis.',
    commandResults: {
      ...emptyAppScenario.commandResults,
      load_price_data: {
        id: 'fixture_aapl_1m',
        meta: DATASET_META,
        cache_hit: true,
        cache_path: 'C:/VectorGridData/cache/aapl_1m.vta',
        load_source: 'binary_cache',
      },
      preview_dynamic_double_ma_strategy_ir_json: {
        schema_compatibility: 'Compatible',
        ir: { profile: 'double_ma' },
        pretty_json: '{\n  "profile": "double_ma"\n}',
        preview: {
          requested_backend: 'gpu_unified',
          resolved_backend: 'cpu',
          plan_fingerprint_hex: 'qa-dynamic-plan',
          indicator_count: 2,
          condition_count: 2,
          group_count: 2,
          capability: {
            gpu_dynamic_supported: false,
            cpu_dynamic_supported: true,
            reasons: ['browser QA fixture uses CPU dynamic route'],
          },
        },
      },
      run_double_ma_optimization: buildDoubleMaRunResult(),
      run_composite_cpu_optimization: buildCompositeCpuRunResult(),
      run_composite_mo_optimization: buildCompositeMoRunResult(),
      compute_double_ma_drilldown: BASE_DRILL_CURVES,
      compute_composite_cpu_drilldown: COMPOSITE_DRILL_CURVES,
      compute_double_ma_local_stability: {
        base: DOUBLE_PARAMS,
        objective: 'Sharpe',
        radius: 1,
        step: 1,
        start_t: 1,
        points: [],
        summary: { base: 0, min: 0, median: 0, max: 0 },
      },
      compute_composite_cpu_local_stability: {
        base: COMPOSITE_PARAMS,
        objective: 'Sharpe',
        radius: 1,
        step: 1,
        start_t: 1,
        points: [],
        summary: { base: 0, min: 0, median: 0, max: 0 },
      },
      get_price_bars: PRICE_BARS,
      run_double_ma_validation: buildValidationResult('CpuOnly'),
      run_composite_validation: buildValidationResult('CompositeCpu'),
      save_run_artifact: {
        path: 'C:/runs/vectorgrid_run_conventional_strategy_qa.json',
        bytes: 73728,
        artifact_updated: true,
      },
    },
  }
}

async function loadData(page: Page): Promise<void> {
  await action(page, 'shell.workflowStep.data').click()
  await action(page, 'data.sample.load').first().click()
  await expect
    .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === 'load_price_data'))
    .toBe(true)
}

async function canvasPaintSummary(page: Page, selector: string): Promise<{
  canvasCount: number
  paintedCanvasCount: number
  paintedSamples: number
}> {
  return page.locator(selector).evaluate((host) => {
    const canvases = Array.from(host.querySelectorAll('canvas')) as HTMLCanvasElement[]
    let paintedCanvasCount = 0
    let paintedSamples = 0
    for (const canvas of canvases) {
      if (canvas.width <= 0 || canvas.height <= 0) continue
      const ctx = canvas.getContext('2d', { willReadFrequently: true })
      if (!ctx) continue
      const xStep = Math.max(1, Math.floor(canvas.width / 20))
      const yStep = Math.max(1, Math.floor(canvas.height / 16))
      let canvasSamples = 0
      for (let y = 0; y < canvas.height; y += yStep) {
        for (let x = 0; x < canvas.width; x += xStep) {
          const pixel = ctx.getImageData(x, y, 1, 1).data
          const alpha = pixel[3] ?? 0
          const rgb = (pixel[0] ?? 0) + (pixel[1] ?? 0) + (pixel[2] ?? 0)
          if (alpha > 0 && rgb > 0) {
            canvasSamples += 1
            paintedSamples += 1
          }
        }
      }
      if (canvasSamples > 4) paintedCanvasCount += 1
    }
    return { canvasCount: canvases.length, paintedCanvasCount, paintedSamples }
  })
}

async function heatmapPainted(page: Page): Promise<boolean> {
  return page.locator('#heatmapCanvasVue').evaluate((node) => {
    const canvas = node as HTMLCanvasElement
    const ctx = canvas.getContext('2d', { willReadFrequently: true })
    if (!ctx || canvas.width <= 0 || canvas.height <= 0) return false
    const image = ctx.getImageData(0, 0, canvas.width, canvas.height)
    let paintedSamples = 0
    for (let index = 3; index < image.data.length; index += 128) {
      if (image.data[index] > 0) paintedSamples += 1
      if (paintedSamples > 24) return true
    }
    return false
  })
}

async function ensureEvidenceDir(): Promise<string> {
  const evidenceDir = path.resolve(process.cwd(), '../output/playwright/conventional-strategy-workflow')
  await fs.mkdir(evidenceDir, { recursive: true })
  return evidenceDir
}

async function writeEvidence(kind: string, evidence: Record<string, unknown>): Promise<void> {
  const evidenceDir = await ensureEvidenceDir()
  await fs.writeFile(
    path.join(evidenceDir, `${sanitizeFilePart(kind)}.json`),
    `${JSON.stringify(evidence, null, 2)}\n`,
    'utf8',
  )
}

test.describe('conventional strategy workflow matrix', () => {
  for (const strategy of STRATEGIES) {
    test(`runs ${strategy.label} through optimization, visual review, validation, and analysis @workflow @visual`, async ({
      page,
      context,
    }) => {
      const guard = installQaGuards(page)
      const evidenceDir = await ensureEvidenceDir()
      await page.setViewportSize({ width: 1600, height: 1000 })
      await gotoMockedShellWithScenario(page, context, conventionalScenario())

      await loadData(page)
      await action(page, 'shell.workflowStep.optimize').click()
      await page.locator('#vueStrategyKind').selectOption(strategy.kind)
      await strategy.setup?.(page)

      await expect(action(page, 'optimize.run')).toBeEnabled()
      await action(page, 'optimize.run').click()
      await expect
        .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === strategy.optimizationCommand))
        .toBe(true)

      const optimizationEntry = (await readTauriInvokeLog(page)).find((entry) => entry.command === strategy.optimizationCommand)
      expect(optimizationEntry?.args?.req, `${strategy.kind} optimization request`).toMatchObject({
        strategy_kind: strategy.kind,
      })
      if (strategy.kind === 'DynamicDoubleMa') {
        expect((await readTauriInvokeLog(page)).some((entry) => entry.command === 'preview_dynamic_double_ma_strategy_ir_json')).toBe(true)
      }

      await action(page, 'shell.resultsTab.top').click()
      await expect(page.locator('#results_panel_top')).toBeVisible()
      await expect(page.locator('.topTableVue [data-top-row="1"]').first()).toBeVisible()
      await expect(action(page, 'top.selected.openCandidateDetail')).toBeEnabled()

      if (strategy.expectsPareto) {
        await action(page, 'shell.resultsTab.pareto').click()
        await expect(page.locator('#results_panel_pareto')).toBeVisible()
        await expect(page.getByText(/Pareto/i).first()).toBeVisible()
        await action(page, 'shell.resultsTab.top').click()
      }

      let heatmapSummary: Record<string, unknown> = { expected: strategy.expectsHeatmap }
      if (strategy.expectsHeatmap) {
        await action(page, 'shell.resultsTab.heatmap').click()
        await expect(page.locator('#results_panel_heatmap')).toBeVisible()
        await expect(page.locator('#heatmapCanvasStackVue')).toBeVisible()
        await expect(page.locator('#heatmapMetaVue')).toContainText(/heatmap/i)
        await expect.poll(async () => heatmapPainted(page)).toBe(true)
        heatmapSummary = {
          expected: true,
          painted: await heatmapPainted(page),
          meta: await page.locator('#heatmapMetaVue').innerText(),
        }
        await page.locator('#results_panel_heatmap').screenshot({
          path: path.join(evidenceDir, `${sanitizeFilePart(strategy.kind)}-heatmap.png`),
        })
      }

      await action(page, 'shell.resultsTab.top').click()
      await action(page, 'top.selected.openCandidateDetail').click()
      await expect(page.locator('#results_panel_drill')).toBeVisible()
      await expect(page.locator('#drillTvChartVue')).toBeVisible()
      await expect(page.locator('#drillTvChartMetaVue')).toContainText(/candles=72/)
      await expect(page.locator('#drillTvChartMetaVue')).toContainText(/fast=72/)
      await expect(page.locator('#drillTvChartMetaVue')).toContainText(/slow=72/)
      await expect(page.locator('#drillTvChartMetaVue')).toContainText(/entries=3/)
      await expect(page.locator('#drillTvChartMetaVue')).toContainText(/exits=3/)
      await expect(page.locator('#drillTvChartMetaVue')).toContainText(/markers=6/)
      if (strategy.optimizationCommand !== 'run_double_ma_optimization' || strategy.expectsPareto) {
        await expect(page.locator('#drillTvChartMetaVue')).toContainText(/halftrend=72/)
        await expect(page.locator('#drillTvChartMetaVue')).toContainText(/ott=72/)
        await expect(page.locator('#drillTvChartMetaVue')).toContainText(/macz=72/)
        await expect(page.locator('.chartLegendOverlay').getByText(/HalfTrend/)).toBeVisible()
        await expect(page.locator('.chartLegendOverlay').getByText('OTT')).toBeVisible()
        await expect(page.locator('.chartLegendOverlay').getByText('MAC-Z')).toBeVisible()
      }
      await expect
        .poll(async () => {
          const summary = await canvasPaintSummary(page, '#drillTvChartVue')
          return summary.canvasCount > 0 && summary.paintedCanvasCount > 0 && summary.paintedSamples > 16
        })
        .toBe(true)
      const initialChartView = await page.locator('.tvChartFrame[data-chart-ready="true"]').getAttribute('data-chart-view')
      await action(page, 'candidateChart.panLeft').click()
      await expect
        .poll(async () => page.locator('.tvChartFrame[data-chart-ready="true"]').getAttribute('data-chart-view'))
        .not.toBe(initialChartView)
      const pannedLeftChartView = await page.locator('.tvChartFrame[data-chart-ready="true"]').getAttribute('data-chart-view')
      await action(page, 'candidateChart.panRight').click()
      await expect
        .poll(async () => page.locator('.tvChartFrame[data-chart-ready="true"]').getAttribute('data-chart-view'))
        .not.toBe(pannedLeftChartView)
      await action(page, 'candidateChart.fitContent').click()
      await expect(page.locator('#drillTvChartMetaVue')).toContainText(/view=/)
      const chartSummary = await canvasPaintSummary(page, '#drillTvChartVue')
      const chartView = await page.locator('.tvChartFrame[data-chart-ready="true"]').getAttribute('data-chart-view')
      await page.locator('.tvChartFrame').screenshot({
        path: path.join(evidenceDir, `${sanitizeFilePart(strategy.kind)}-candidate-detail-chart.png`),
      })

      await action(page, 'shell.workflowStep.validate').click()
      await expect(action(page, 'validation.run')).toBeEnabled()
      await action(page, 'validation.run').click()
      let validationEntry = (await readTauriInvokeLog(page)).find((entry) => entry.command === strategy.validationCommand)
      let validationBlocked = false
      let validationStatusText = ''
      if (strategy.expectedValidationBlock) {
        await expect(page.getByText(strategy.expectedValidationBlock).first()).toBeVisible()
        validationBlocked = true
        validationStatusText = await page.getByText(strategy.expectedValidationBlock).first().innerText()
      } else {
        await expect
          .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === strategy.validationCommand))
          .toBe(true)
        validationEntry = (await readTauriInvokeLog(page)).find((entry) => entry.command === strategy.validationCommand)
        expect(validationEntry?.args?.req, `${strategy.kind} validation request`).toBeTruthy()
        if (strategy.kind === 'DynamicDoubleMa') {
          expect((validationEntry?.args?.req as Record<string, unknown>).dynamic_strategy_ir).toEqual({
            profile: 'double_ma',
          })
        }
        await expect(page.getByText(/Validation completed in/i).first()).toBeVisible()
        await expect(page.locator('.validationTableVue tbody tr').first()).toBeVisible()
      }

      await action(page, 'shell.workflowStep.analyze').click()
      await expect(page.locator('#resultsTabBar')).toBeVisible()
      await expect(action(page, 'shell.resultsTab.drill')).toBeVisible()
      if (strategy.expectsHeatmap) {
        await expect(action(page, 'shell.resultsTab.heatmap')).toBeVisible()
      }
      await expectNoHorizontalDocumentOverflow(page)
      await expectNoStuckBusyState(page)

      const invokes = await readTauriInvokeLog(page)
      await writeEvidence(strategy.kind, {
        strategy: strategy.kind,
        optimizationCommand: strategy.optimizationCommand,
        validationCommand: strategy.validationCommand,
        optimizationStrategyKind: (optimizationEntry?.args?.req as Record<string, unknown> | undefined)?.strategy_kind,
        optimizationInvokeCount: invokes.filter((entry) => entry.command === strategy.optimizationCommand).length,
        validationInvokeCount: invokes.filter((entry) => entry.command === strategy.validationCommand).length,
        validationBlocked,
        validationStatusText,
        chartSummary,
        chartView,
        heatmapSummary,
      })

      await guard.assertClean()
    })
  }
})
