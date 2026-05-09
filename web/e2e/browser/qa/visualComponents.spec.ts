import { expect, test, type Page } from '@playwright/test'
import fs from 'node:fs/promises'
import path from 'node:path'
import { candidateDetailReadyScenario, type MockTauriScenario } from '../fixtures'
import { installQaGuards } from '../support/qaAssertions'
import { gotoMockedShellWithScenario, readTauriInvokeLog } from '../support/workflowRunner'

const drillCurves = {
  equity: { start_t: 1, end_t: 4, bins: 2, minmax: [1, 1.02, 1.01, 1.05] },
  drawdown: { start_t: 1, end_t: 4, bins: 2, minmax: [0, -0.01, -0.02, -0.01] },
  trades: [
    { entry_t: 1, exit_t: 2, direction: 1, pnl: 0.03, bars: 1, open: false },
    { entry_t: 3, exit_t: 4, direction: -1, pnl: -0.01, bars: 1, open: false },
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
    { time: 1_744_500_120, from_index: 3, to_index: 3, fast: 100.3, slow: 100.2 },
    { time: 1_744_500_180, from_index: 4, to_index: 4, fast: 100.1, slow: 100.25 },
  ],
}

const drillPriceBars = [
  { time: 1_744_500_000, open: 100, high: 101, low: 99.8, close: 100.6, from_index: 1, to_index: 1 },
  { time: 1_744_500_060, open: 100.6, high: 101.2, low: 100.3, close: 101.0, from_index: 2, to_index: 2 },
  { time: 1_744_500_120, open: 101.0, high: 101.1, low: 100.2, close: 100.4, from_index: 3, to_index: 3 },
  { time: 1_744_500_180, open: 100.4, high: 100.8, low: 99.9, close: 100.0, from_index: 4, to_index: 4 },
]

const drillChartVisualScenario: MockTauriScenario = {
  ...candidateDetailReadyScenario,
  name: 'drill-chart-visual-ready',
  description: 'A selected DoubleMa candidate has OHLC bars and drilldown curves for visual chart smoke testing.',
  commandResults: {
    ...candidateDetailReadyScenario.commandResults,
    compute_double_ma_drilldown: drillCurves,
    get_price_bars: drillPriceBars,
  },
}

function action(page: Page, id: string) {
  return page.locator(`[data-qa-action="${id}"]`)
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

test('visual smoke: drill Lightweight Chart renders candlesticks, overlays, and markers @visual', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await page.setViewportSize({ width: 1600, height: 1000 })
  await gotoMockedShellWithScenario(page, context, drillChartVisualScenario)

  await action(page, 'shell.workflowStep.data').click()
  await action(page, 'data.sample.load').first().click()
  await expect
    .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === 'load_price_data'))
    .toBe(true)

  await action(page, 'shell.workflowStep.optimize').click()
  await action(page, 'optimize.run').click()
  await expect
    .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === 'run_double_ma_optimization'))
    .toBe(true)

  await action(page, 'shell.resultsTab.top').click()
  await expect(action(page, 'top.selected.openCandidateDetail')).toBeEnabled()
  await action(page, 'top.selected.openCandidateDetail').click()

  await expect(page.locator('#results_panel_drill')).toBeVisible()
  await expect(page.locator('#drillTvChartVue')).toBeVisible()
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/candles=4/)
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/markers=4/)
  await expect
    .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === 'get_price_bars'))
    .toBe(true)

  await expect
    .poll(async () => {
      const summary = await canvasPaintSummary(page, '#drillTvChartVue')
      return summary.canvasCount > 0 && summary.paintedCanvasCount > 0 && summary.paintedSamples > 16
    })
    .toBe(true)
  const summary = await canvasPaintSummary(page, '#drillTvChartVue')
  expect(summary.canvasCount, 'Lightweight Chart should mount canvas layers').toBeGreaterThan(0)
  expect(summary.paintedCanvasCount, 'Lightweight Chart canvas layers should contain painted pixels').toBeGreaterThan(0)
  expect(summary.paintedSamples, 'Lightweight Chart canvas should not be visually blank').toBeGreaterThan(16)

  const visualEvidenceDir = path.resolve(process.cwd(), '../output/playwright/visual-components')
  await fs.mkdir(visualEvidenceDir, { recursive: true })
  await page.screenshot({
    path: path.join(visualEvidenceDir, 'drill-lightweight-chart.png'),
    fullPage: true,
  })

  await guard.assertClean()
})
