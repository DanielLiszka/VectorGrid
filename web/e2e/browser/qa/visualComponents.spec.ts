import { expect, test, type Page } from '@playwright/test'
import fs from 'node:fs/promises'
import path from 'node:path'
import { candidateDetailReadyScenario, heatmapLayoutScenario, type MockTauriScenario } from '../fixtures'
import { expectNoHorizontalDocumentOverflow, installQaGuards } from '../support/qaAssertions'
import { gotoMockedShellWithScenario, readTauriInvokeLog } from '../support/workflowRunner'

const DRILL_VISUAL_BAR_COUNT = 80
const DRILL_VISUAL_START_SEC = 1_744_500_000

const drillPriceBars = Array.from({ length: DRILL_VISUAL_BAR_COUNT }, (_, index) => {
  const time = DRILL_VISUAL_START_SEC + index * 60
  const base = 100 + Math.sin(index / 6) * 1.6 + index * 0.018
  const open = Number((base + Math.sin(index / 3) * 0.18).toFixed(3))
  const close = Number((base + Math.cos(index / 4) * 0.22).toFixed(3))
  return {
    time,
    open,
    high: Number((Math.max(open, close) + 0.45 + (index % 5) * 0.02).toFixed(3)),
    low: Number((Math.min(open, close) - 0.42 - (index % 4) * 0.02).toFixed(3)),
    close,
    from_index: index + 1,
    to_index: index + 1,
  }
})

const drillCurves = {
  equity: { start_t: 1, end_t: 80, bins: 8, minmax: [1, 1.02, 1.01, 1.05, 1.03, 1.08, 1.06, 1.11] },
  drawdown: { start_t: 1, end_t: 80, bins: 8, minmax: [0, -0.01, -0.02, -0.01, -0.015, -0.025, -0.018, -0.012] },
  trades: [
    { entry_t: 4, exit_t: 15, direction: 1, pnl: 0.03, bars: 11, open: false },
    { entry_t: 23, exit_t: 34, direction: -1, pnl: -0.01, bars: 11, open: false },
    { entry_t: 43, exit_t: 57, direction: 1, pnl: 0.018, bars: 14, open: false },
    { entry_t: 63, exit_t: 76, direction: -1, pnl: 0.012, bars: 13, open: false },
  ],
  trade_stats: {
    closed_trades: 4,
    open_trades: 0,
    wins: 3,
    losses: 1,
    win_rate: 0.75,
    avg_win: 0.02,
    avg_loss: -0.01,
    profit_factor: 6,
    expectancy: 0.0125,
  },
  indicator_bars: drillPriceBars.map((bar, index) => ({
    time: bar.time,
    from_index: index + 1,
    to_index: index + 1,
    fast: Number((bar.close + Math.sin(index / 5) * 0.22).toFixed(3)),
    slow: Number((bar.close + Math.cos(index / 9) * 0.32 - 0.08).toFixed(3)),
  })),
  composite_indicator_bars: drillPriceBars.map((bar, index) => ({
    time: bar.time,
    from_index: index + 1,
    to_index: index + 1,
    halftrend: index % 12 < 6 ? 0 : 1,
    macz_hist: Number((Math.sin(index / 7) * 0.65).toFixed(3)),
    ott_line: Number((bar.close - 0.52 + Math.cos(index / 10) * 0.18).toFixed(3)),
  })),
}

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

async function canvasVisualDiversity(page: Page, selector: string): Promise<{
  uniqueColors: number
  visibleSamples: number
}> {
  return page.locator(selector).evaluate((node) => {
    const canvas = node as HTMLCanvasElement
    const ctx = canvas.getContext('2d', { willReadFrequently: true })
    if (!ctx || canvas.width <= 0 || canvas.height <= 0) {
      return { uniqueColors: 0, visibleSamples: 0 }
    }
    const unique = new Set<string>()
    let visibleSamples = 0
    const xStep = Math.max(1, Math.floor(canvas.width / 28))
    const yStep = Math.max(1, Math.floor(canvas.height / 28))
    for (let y = 0; y < canvas.height; y += yStep) {
      for (let x = 0; x < canvas.width; x += xStep) {
        const pixel = ctx.getImageData(x, y, 1, 1).data
        const alpha = pixel[3] ?? 0
        if (alpha <= 0) continue
        visibleSamples += 1
        unique.add(`${pixel[0] ?? 0}:${pixel[1] ?? 0}:${pixel[2] ?? 0}`)
      }
    }
    return { uniqueColors: unique.size, visibleSamples }
  })
}

async function saveCanvasStackPng(page: Page, hostSelector: string, outputPath: string): Promise<void> {
  const dataUrl = await page.locator(hostSelector).evaluate((host) => {
    const hostElement = host as HTMLElement
    const hostRect = hostElement.getBoundingClientRect()
    const ratio = Math.min(3, Math.max(1, window.devicePixelRatio || 1))
    const output = document.createElement('canvas')
    output.width = Math.max(1, Math.round(hostRect.width * ratio))
    output.height = Math.max(1, Math.round(hostRect.height * ratio))
    const ctx = output.getContext('2d')
    if (!ctx) return ''
    ctx.scale(ratio, ratio)
    ctx.fillStyle = window.getComputedStyle(hostElement).backgroundColor || '#fff'
    ctx.fillRect(0, 0, hostRect.width, hostRect.height)
    for (const canvas of Array.from(hostElement.querySelectorAll('canvas')) as HTMLCanvasElement[]) {
      const rect = canvas.getBoundingClientRect()
      if (rect.width <= 0 || rect.height <= 0) continue
      ctx.drawImage(canvas, rect.left - hostRect.left, rect.top - hostRect.top, rect.width, rect.height)
    }
    return output.toDataURL('image/png')
  })
  const base64 = dataUrl.split(',')[1]
  if (!base64) throw new Error(`Unable to export canvas evidence for ${hostSelector}`)
  await fs.writeFile(outputPath, Buffer.from(base64, 'base64'))
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
  await expect(page.locator('.tvChartFrame')).toHaveAttribute('data-chart-ready', 'true')
  await expect(page.locator('.chartStateBadge')).toContainText(/Chart ready/)
  await expect(page.locator('.chartEvidenceChips')).toContainText(/price bars/)
  await expect(page.locator('.chartEvidenceChips')).toContainText(/MA overlays/)
  await expect(page.locator('.chartEvidenceChips')).toContainText(/OTT/)
  await expect(page.locator('.chartLegendOverlay')).toContainText(/Fast MA/)
  await expect(page.locator('.chartLegendOverlay')).toContainText(/Slow MA/)
  await expect(page.locator('.chartLegendOverlay')).toContainText(/HalfTrend/)
  await expect(page.locator('.chartLegendOverlay')).toContainText(/MAC-Z/)
  await expect(page.locator('.chartLegendOverlay')).toContainText(/OTT/)
  await expect(page.locator('.chartLegendOverlay')).toContainText(/Entries/)
  await expect(page.locator('.chartLegendOverlay')).toContainText(/Exits/)
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/candles=80/)
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/halftrend=80/)
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/ott=80/)
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/macz=80/)
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/entries=4/)
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/exits=4/)
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/markers=8/)
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/selected=none/)
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
  await expect(page.locator('[data-qa-action="candidateChart.panLeft"]')).toBeEnabled()
  const initialRange = await page.locator('.tvChartFrame').getAttribute('data-chart-view')
  await page.locator('[data-qa-action="candidateChart.panLeft"]').click()
  await expect.poll(async () => page.locator('.tvChartFrame').getAttribute('data-chart-view')).not.toBe(initialRange)
  const pannedRange = await page.locator('.tvChartFrame').getAttribute('data-chart-view')
  await page.locator('[data-qa-action="candidateChart.panRight"]').click()
  await expect.poll(async () => page.locator('.tvChartFrame').getAttribute('data-chart-view')).not.toBe(pannedRange)
  await page.locator('[data-qa-action="candidateChart.fitContent"]').click()
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/view=/)

  const visualEvidenceDir = path.resolve(process.cwd(), '../output/playwright/visual-components')
  await fs.mkdir(visualEvidenceDir, { recursive: true })
  await page.locator('.tvChartFrame').screenshot({
    path: path.join(visualEvidenceDir, 'drill-lightweight-chart.png'),
  })

  await guard.assertClean()
})

test('visual audit: heatmap renders high-coverage 2D surface with quality metadata @visual', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await page.setViewportSize({ width: 1600, height: 1000 })
  await gotoMockedShellWithScenario(page, context, heatmapLayoutScenario)

  await action(page, 'shell.workflowStep.analyze').click()
  await action(page, 'shell.resultsTab.heatmap').click()
  await expect(page.locator('#results_panel_heatmap')).toBeVisible()
  await expect(page.locator('#heatmapCanvasStackVue')).toBeVisible()
  await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute('data-render-ready', 'true')
  await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute('data-total-cells', '1024')
  await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute('data-finite-coverage', /0\.\d+|1\.000/)
  await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute('data-axis-summary', /[A-Z]+ \d+-\d+; [A-Z]+ \d+-\d+/)
  await expect(page.locator('#heatmapDomainSummaryVue')).toContainText(/bins are unavailable/)
  await expect(page.locator('#heatmapFitModeBadgeVue')).toContainText(/valid domain/)
  await expect(page.locator('[data-qa-action="heatmap.reviewTop"]')).toBeVisible()
  await expect(page.locator('[data-qa-action="heatmap.exportPng"]')).toBeEnabled()

  const geometry = await page.locator('#heatmapCanvasStackVue').evaluate((element) => {
    const rect = element.getBoundingClientRect()
    return { width: rect.width, height: rect.height }
  })
  expect(geometry.width, 'heatmap should have enough physical room for labels and selection').toBeGreaterThanOrEqual(680)
  expect(Math.abs(geometry.width - geometry.height), 'heatmap should stay square').toBeLessThanOrEqual(2)

  await expect
    .poll(async () => {
      const summary = await canvasPaintSummary(page, '#heatmapCanvasStackVue')
      return summary.canvasCount >= 2 && summary.paintedCanvasCount > 0 && summary.paintedSamples > 32
    })
    .toBe(true)
  const diversity = await canvasVisualDiversity(page, '#heatmapCanvasVue')
  expect(diversity.visibleSamples, 'heatmap should expose enough sampled painted cells').toBeGreaterThan(80)
  expect(diversity.uniqueColors, 'heatmap should not collapse to one color').toBeGreaterThan(8)
  await expectNoHorizontalDocumentOverflow(page)

  const visualEvidenceDir = path.resolve(process.cwd(), '../output/playwright/visual-components')
  await fs.mkdir(visualEvidenceDir, { recursive: true })
  await saveCanvasStackPng(page, '#heatmapCanvasStackVue', path.join(visualEvidenceDir, 'heatmap-quality.png'))

  await guard.assertClean()
})
