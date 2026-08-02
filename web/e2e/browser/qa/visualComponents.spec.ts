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

const degenerateEquityEnvelope = Array.from({ length: DRILL_VISUAL_BAR_COUNT }, (_, index) => {
  const value = Number((1 + index * 0.0014 + Math.sin(index / 7) * 0.012).toFixed(5))
  return [value, value]
}).flat()

const degenerateDrawdownEnvelope = Array.from({ length: DRILL_VISUAL_BAR_COUNT }, (_, index) => {
  const value = Number((-Math.abs(Math.sin(index / 8)) * 0.045).toFixed(5))
  return [value, value]
}).flat()

const drillCurves = {
  equity: { start_t: 1, end_t: 80, bins: DRILL_VISUAL_BAR_COUNT, minmax: degenerateEquityEnvelope },
  drawdown: { start_t: 1, end_t: 80, bins: DRILL_VISUAL_BAR_COUNT, minmax: degenerateDrawdownEnvelope },
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

const executionChartVisualScenario: MockTauriScenario = {
  ...drillChartVisualScenario,
  name: 'execution-chart-visual-ready',
  family: 'liveExecution',
  description:
    'A loaded AAPL dataset and fresh local-paper broker review expose price levels and order lifecycle telemetry.',
  commandResults: {
    ...drillChartVisualScenario.commandResults,
    load_price_data: {
      ...(drillChartVisualScenario.commandResults.load_price_data as Record<string, unknown>),
      id: 'AAPL',
    },
    get_execution_strategy_support_matrix: [
      {
        strategy_kind: 'DoubleMa',
        supported: true,
        adapter: 'double_ma_execution_adapter',
        support_level: 'signal_and_delta',
        notes: 'Browser visual fixture support.',
      },
    ],
    get_local_paper_execution_snapshot: {
      symbol: 'AAPL',
      account_number: 'local-paper',
      account_status: 'simulated',
      currency: 'USD',
      cash: '99850.00',
      buying_power: '99600.00',
      portfolio_value: '100002.46',
      fetched_at_unix_ms: 1_900_000_000_000,
      position: {
        symbol: 'AAPL',
        qty: '2',
        side: 'long',
        avg_entry_price: '100.65',
        current_price: '101.88',
        market_value: '203.76',
        unrealized_pl: '2.46',
        unrealized_plpc: '0.01222',
      },
      orders: [
        {
          id: 'entry-filled',
          side: 'buy',
          status: 'filled',
          qty: '2',
          filled_qty: '2',
          filled_avg_price: '100.65',
          order_type: 'market',
          submitted_at: new Date((DRILL_VISUAL_START_SEC + 10 * 60) * 1000).toISOString(),
        },
        {
          id: 'scale-partial',
          side: 'sell',
          status: 'partially_filled',
          qty: '2',
          filled_qty: '1',
          filled_avg_price: '101.42',
          order_type: 'market',
          submitted_at: new Date((DRILL_VISUAL_START_SEC + 20 * 60) * 1000).toISOString(),
        },
        {
          id: 'protective-stop',
          side: 'sell',
          status: 'accepted',
          qty: '2',
          filled_qty: '0',
          order_type: 'stop',
          stop_price: '98.70',
          bracket_leg: 'stop_loss',
          tie_break: 'stop_loss_first',
          parent_order_id: 'entry-filled',
          submitted_at: new Date((DRILL_VISUAL_START_SEC + 30 * 60) * 1000).toISOString(),
        },
        {
          id: 'profit-target',
          side: 'sell',
          status: 'new',
          qty: '2',
          filled_qty: '0',
          order_type: 'limit',
          limit_price: '104.30',
          bracket_leg: 'take_profit',
          tie_break: 'stop_loss_first',
          parent_order_id: 'entry-filled',
          submitted_at: new Date((DRILL_VISUAL_START_SEC + 31 * 60) * 1000).toISOString(),
        },
        {
          id: 'replaced-limit',
          side: 'buy',
          status: 'canceled',
          qty: '1',
          filled_qty: '0',
          order_type: 'limit',
          limit_price: '99.10',
          submitted_at: new Date((DRILL_VISUAL_START_SEC + 40 * 60) * 1000).toISOString(),
        },
        {
          id: 'working-limit',
          side: 'buy',
          status: 'held',
          qty: '1',
          filled_qty: '0',
          order_type: 'limit',
          limit_price: '99.40',
          submitted_at: new Date((DRILL_VISUAL_START_SEC + 42 * 60) * 1000).toISOString(),
        },
      ],
    },
  },
  expectedInvokes: [
    'load_price_data',
    'get_local_paper_execution_snapshot',
    'get_price_bars',
  ],
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

async function heatmapLegendSamples(page: Page): Promise<string[]> {
  return page.locator('#heatmapLegendVue').evaluate((node) => {
    const canvas = node as HTMLCanvasElement
    const context = canvas.getContext('2d', { willReadFrequently: true })
    if (!context || canvas.width <= 0 || canvas.height <= 0) return []
    const y = Math.max(0, Math.floor(canvas.height / 2))
    return [2, Math.floor(canvas.width / 2), Math.max(0, canvas.width - 3)].map((x) => {
      const pixel = context.getImageData(x, y, 1, 1).data
      return `${pixel[0] ?? 0}:${pixel[1] ?? 0}:${pixel[2] ?? 0}:${pixel[3] ?? 0}`
    })
  })
}

async function saveHeatmapSurfaceWithLegendPng(
  page: Page,
  surfaceSelector: string,
  outputPath: string,
): Promise<void> {
  await page.setViewportSize({ width: 1600, height: 1800 })
  const surface = page.locator(surfaceSelector)
  const legend = page.locator('.heatmapLegendBlock')
  await surface.evaluate((element) => {
    element.scrollIntoView({ block: 'center', inline: 'nearest' })
  })
  await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())))
  await expect(surface).toBeVisible()
  await expect(legend).toBeVisible()
  const surfaceBox = await surface.boundingBox()
  const legendBox = await legend.boundingBox()
  if (!surfaceBox || !legendBox) {
    throw new Error(`Unable to capture heatmap surface and legend for ${surfaceSelector}`)
  }
  const padding = 10
  const viewport = page.viewportSize()
  const left = Math.max(0, Math.floor(Math.min(surfaceBox.x, legendBox.x) - padding))
  const top = Math.max(0, Math.floor(Math.min(surfaceBox.y, legendBox.y) - padding))
  const right = Math.min(
    viewport?.width ?? 1600,
    Math.ceil(Math.max(surfaceBox.x + surfaceBox.width, legendBox.x + legendBox.width) + padding),
  )
  const bottom = Math.min(
    viewport?.height ?? 1800,
    Math.ceil(Math.max(surfaceBox.y + surfaceBox.height, legendBox.y + legendBox.height) + padding),
  )
  expect(right - left, 'heatmap evidence clip should have positive width').toBeGreaterThan(0)
  expect(bottom - top, 'heatmap evidence clip should have positive height').toBeGreaterThan(0)
  expect(surfaceBox.y, 'heatmap surface should be within the evidence viewport').toBeGreaterThanOrEqual(0)
  expect(
    legendBox.y + legendBox.height,
    'heatmap legend should be within the evidence viewport',
  ).toBeLessThanOrEqual(viewport?.height ?? 1800)
  await page.screenshot({
    path: outputPath,
    clip: {
      x: left,
      y: top,
      width: right - left,
      height: bottom - top,
    },
  })
}

async function supportSeriesPixelCounts(page: Page): Promise<number[]> {
  return page.locator('#equityCanvasVue, #drawdownCanvasVue').evaluateAll((nodes) =>
    nodes.map((node) => {
      const canvas = node as HTMLCanvasElement
      const ctx = canvas.getContext('2d', { willReadFrequently: true })
      if (!ctx) return 0
      const pixels = ctx.getImageData(0, 0, canvas.width, canvas.height).data
      let seriesPixels = 0
      for (let offset = 0; offset < pixels.length; offset += 4) {
        const red = pixels[offset] ?? 0
        const green = pixels[offset + 1] ?? 0
        const blue = pixels[offset + 2] ?? 0
        const alpha = pixels[offset + 3] ?? 0
        const near = (targetRed: number, targetGreen: number, targetBlue: number) =>
          Math.abs(red - targetRed) <= 4 &&
          Math.abs(green - targetGreen) <= 4 &&
          Math.abs(blue - targetBlue) <= 4
        const isEquityStroke = near(4, 120, 87) || near(34, 197, 94)
        const isDrawdownStroke = near(190, 18, 60) || near(251, 113, 133)
        if (alpha > 0 && (isEquityStroke || isDrawdownStroke)) seriesPixels += 1
      }
      return seriesPixels
    }),
  )
}

async function expectSupportSeriesPainted(page: Page, lifecycle: string): Promise<void> {
  await expect
    .poll(async () => {
      const counts = await supportSeriesPixelCounts(page)
      return counts.length === 2 && counts.every((count) => count > 20)
    })
    .toBe(true)
  const counts = await supportSeriesPixelCounts(page)
  expect(counts).toHaveLength(2)
  expect(counts[0], `Equity support chart should remain painted ${lifecycle}`).toBeGreaterThan(20)
  expect(counts[1], `Drawdown support chart should remain painted ${lifecycle}`).toBeGreaterThan(20)
}

async function expectSupportCanvasTheme(
  page: Page,
  theme: 'light' | 'dark',
): Promise<void> {
  const expected = theme === 'dark'
    ? [
        { stroke: '#22c55e', rgb: [34, 197, 94] },
        { stroke: '#fb7185', rgb: [251, 113, 133] },
      ]
    : [
        { stroke: '#047857', rgb: [4, 120, 87] },
        { stroke: '#be123c', rgb: [190, 18, 60] },
      ]
  await expect
    .poll(async () =>
      page.locator('#equityCanvasVue, #drawdownCanvasVue').evaluateAll(
        (nodes, palette) =>
          nodes.map((node, index) => {
            const canvas = node as HTMLCanvasElement
            const target = palette[index]
            const context = canvas.getContext('2d', { willReadFrequently: true })
            if (!target || !context) {
              return { theme: '', stroke: '', pixels: 0 }
            }
            const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data
            let matchingPixels = 0
            for (let offset = 0; offset < pixels.length; offset += 4) {
              const alpha = pixels[offset + 3] ?? 0
              if (
                alpha > 0 &&
                Math.abs((pixels[offset] ?? 0) - target.rgb[0]) <= 4 &&
                Math.abs((pixels[offset + 1] ?? 0) - target.rgb[1]) <= 4 &&
                Math.abs((pixels[offset + 2] ?? 0) - target.rgb[2]) <= 4
              ) {
                matchingPixels += 1
              }
            }
            return {
              theme: canvas.dataset.supportChartTheme || '',
              stroke: canvas.dataset.supportChartStroke || '',
              pixels: matchingPixels,
            }
          }),
        expected,
      ),
    )
    .toEqual([
      expect.objectContaining({ theme, stroke: expected[0]?.stroke, pixels: expect.any(Number) }),
      expect.objectContaining({ theme, stroke: expected[1]?.stroke, pixels: expect.any(Number) }),
    ])
  const states = await page.locator('#equityCanvasVue, #drawdownCanvasVue').evaluateAll(
    (nodes, palette) =>
      nodes.map((node, index) => {
        const canvas = node as HTMLCanvasElement
        const target = palette[index]
        const context = canvas.getContext('2d', { willReadFrequently: true })
        if (!target || !context) return 0
        const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data
        let matchingPixels = 0
        for (let offset = 0; offset < pixels.length; offset += 4) {
          if (
            (pixels[offset + 3] ?? 0) > 0 &&
            Math.abs((pixels[offset] ?? 0) - target.rgb[0]) <= 4 &&
            Math.abs((pixels[offset + 1] ?? 0) - target.rgb[1]) <= 4 &&
            Math.abs((pixels[offset + 2] ?? 0) - target.rgb[2]) <= 4
          ) {
            matchingPixels += 1
          }
        }
        return matchingPixels
      }),
    expected,
  )
  expect(states[0], `Equity support pixels should use the ${theme} palette`).toBeGreaterThan(20)
  expect(states[1], `Drawdown support pixels should use the ${theme} palette`).toBeGreaterThan(20)
}

interface SupportCanvasGeometry {
  backingWidth: number
  backingHeight: number
  cssWidth: number
  cssHeight: number
  contentWidth: number
  contentHeight: number
  pixelRatio: number
  logicalWidth: number
  logicalHeight: number
}

async function supportCanvasGeometry(page: Page): Promise<SupportCanvasGeometry[]> {
  return page.locator('#equityCanvasVue, #drawdownCanvasVue').evaluateAll((nodes) =>
    nodes.map((node) => {
      const canvas = node as HTMLCanvasElement
      const rect = canvas.getBoundingClientRect()
      return {
        backingWidth: canvas.width,
        backingHeight: canvas.height,
        cssWidth: rect.width,
        cssHeight: rect.height,
        contentWidth: canvas.clientWidth,
        contentHeight: canvas.clientHeight,
        pixelRatio: Number(canvas.dataset.pixelRatio || window.devicePixelRatio || 1),
        logicalWidth: Number(canvas.dataset.logicalWidth || 0),
        logicalHeight: Number(canvas.dataset.logicalHeight || 0),
      }
    }),
  )
}

async function expectSupportCanvasResolution(page: Page, lifecycle: string): Promise<void> {
  await expect
    .poll(async () => {
      const geometry = await supportCanvasGeometry(page)
      return geometry.length === 2 && geometry.every((item) => {
        const expectedWidth = Math.round(item.contentWidth * item.pixelRatio)
        const expectedHeight = Math.round(item.contentHeight * item.pixelRatio)
        return (
          Math.abs(item.backingWidth - expectedWidth) <= 1 &&
          Math.abs(item.backingHeight - expectedHeight) <= 1 &&
          item.logicalWidth === Math.round(item.contentWidth) &&
          item.logicalHeight === Math.round(item.contentHeight)
        )
      })
    })
    .toBe(true)
  const geometry = await supportCanvasGeometry(page)
  expect(geometry, `Support canvases should be mounted ${lifecycle}`).toHaveLength(2)
  for (const item of geometry) {
    expect(
      item.backingWidth,
      `Support canvas backing width should match content width × DPR ${lifecycle}`,
    ).toBeGreaterThan(item.contentWidth - 1)
    expect(
      item.backingHeight,
      `Support canvas backing height should match content height × DPR ${lifecycle}`,
    ).toBeGreaterThan(item.contentHeight - 1)
  }
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
  const tradingViewAttribution = page.locator(
    '#drillTvChartVue a[href^="https://www.tradingview.com/"]',
  )
  await expect(tradingViewAttribution).toBeVisible()
  await expect(tradingViewAttribution).toHaveAccessibleName(/Charting by TradingView/i)
  await tradingViewAttribution.focus()
  await expect(tradingViewAttribution).toBeFocused()
  await expect(page.locator('.tvChartFrame')).toHaveAttribute('data-chart-ready', 'true')
  await expect(page.locator('.chartStateBadge')).toContainText(/Chart ready/)
  await expect(page.locator('.chartEvidenceChips')).toContainText(/price bars/)
  await expect(page.locator('.chartEvidenceChips')).toContainText(/MA overlays/)
  await expect(page.locator('.chartEvidenceChips')).toContainText(/OTT/)
  await expect(page.locator('#drillTvChartVue')).toHaveAttribute('tabindex', '0')
  await expect(page.locator('#drillTvChartVue')).toHaveAttribute(
    'aria-keyshortcuts',
    'ArrowLeft ArrowRight Home End Escape',
  )
  await expect(page.locator('.chartInteractionHint')).toContainText(/inspect/)
  await expect(page.locator('.chartInteractionHint')).toContainText(/Home\/End jump/)
  await expect(page.locator('.chartInteractionHint')).toContainText(/Esc clear/)
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
  await expectSupportSeriesPainted(page, 'after initial render')
  await expectSupportCanvasResolution(page, 'after initial render')
  const initialSupportGeometry = await supportCanvasGeometry(page)
  await expect(page.locator('[data-qa-action="candidateChart.panLeft"]')).toBeEnabled()
  const initialRange = await page.locator('.tvChartFrame').getAttribute('data-chart-view')
  await page.locator('[data-qa-action="candidateChart.panLeft"]').click()
  await expect.poll(async () => page.locator('.tvChartFrame').getAttribute('data-chart-view')).not.toBe(initialRange)
  const pannedRange = await page.locator('.tvChartFrame').getAttribute('data-chart-view')
  await page.locator('[data-qa-action="candidateChart.panRight"]').click()
  await expect.poll(async () => page.locator('.tvChartFrame').getAttribute('data-chart-view')).not.toBe(pannedRange)
  await page.locator('[data-qa-action="candidateChart.fitContent"]').click()
  await expect(page.locator('#drillTvChartMetaVue')).toContainText(/view=/)

  await action(page, 'shell.resultsTab.top').click()
  await expect(page.locator('#results_panel_top')).toBeVisible()
  await expect(action(page, 'top.selected.openCandidateDetail')).toBeEnabled()
  await action(page, 'top.selected.openCandidateDetail').click()
  await expect(page.locator('#results_panel_drill')).toBeVisible()
  await expect(page.locator('.tvChartFrame')).toHaveAttribute('data-chart-ready', 'true')
  await expectSupportSeriesPainted(page, 'after drill → top → drill remount')
  await expectSupportCanvasResolution(page, 'after drill → top → drill remount')

  await page.setViewportSize({ width: 1240, height: 820 })
  await expect(page.locator('#equityCanvasVue')).toBeVisible()
  await expectSupportSeriesPainted(page, 'after viewport resize')
  await expectSupportCanvasResolution(page, 'after viewport resize')
  const resizedSupportGeometry = await supportCanvasGeometry(page)
  expect(
    resizedSupportGeometry[0]?.backingWidth,
    'Support chart backing width should follow the narrower responsive pane',
  ).toBeLessThan(initialSupportGeometry[0]?.backingWidth ?? Number.POSITIVE_INFINITY)
  await page.setViewportSize({ width: 1600, height: 1000 })
  await expectSupportCanvasResolution(page, 'after restoring the viewport')

  const visualEvidenceDir = path.resolve(process.cwd(), '../output/playwright/visual-components')
  await fs.mkdir(visualEvidenceDir, { recursive: true })
  const chartHost = page.locator('#drillTvChartVue')
  await chartHost.focus()
  await chartHost.press('End')
  const pointInspector = page.locator('.chartPointInspector')
  await expect(pointInspector).toBeVisible()
  await expect(pointInspector).toHaveAttribute('data-inspector-source', 'keyboard')
  await expect(pointInspector).toContainText(/Keyboard point/)
  await expect(pointInspector).toContainText(/ET \(UTC-04:00\)/)
  await expect(pointInspector).toContainText(/O \d/)
  await expect(pointInspector).toContainText(/H \d/)
  await expect(pointInspector).toContainText(/L \d/)
  await expect(pointInspector).toContainText(/C \d/)
  await expect(pointInspector).toContainText(/Fast MA/)
  const endPointTime = await pointInspector.locator('strong').innerText()
  await page.locator('.tvChartFrame').screenshot({
    path: path.join(visualEvidenceDir, 'drill-keyboard-point-inspector.png'),
  })
  await chartHost.press('ArrowLeft')
  await expect
    .poll(async () => pointInspector.locator('strong').innerText())
    .not.toBe(endPointTime)
  await chartHost.press('Escape')
  await expect(pointInspector).toHaveCount(0)

  await page.locator('.tvChartFrame').screenshot({
    path: path.join(visualEvidenceDir, 'drill-lightweight-chart.png'),
  })

  const fastLegendSwatch = page.locator(
    '.chartLegendSwatch[data-chart-series="fast"]',
  )
  await page.evaluate(() => {
    document.documentElement.dataset.themePref = 'light'
    document.documentElement.dataset.theme = 'light'
  })
  await expect
    .poll(async () => fastLegendSwatch.evaluate((element) => getComputedStyle(element).backgroundColor))
    .toBe('rgb(3, 105, 161)')
  await expectSupportCanvasTheme(page, 'light')

  await page.evaluate(() => {
    document.documentElement.dataset.themePref = 'dark'
    document.documentElement.dataset.theme = 'dark'
  })
  await expect
    .poll(async () => fastLegendSwatch.evaluate((element) => getComputedStyle(element).backgroundColor))
    .toBe('rgb(56, 189, 248)')
  await expect(tradingViewAttribution).toBeVisible()
  await expectSupportSeriesPainted(page, 'after switching from light to dark')
  await expectSupportCanvasTheme(page, 'dark')
  await page.locator('.tvChartFrame').screenshot({
    path: path.join(visualEvidenceDir, 'drill-lightweight-chart-dark.png'),
  })

  await page.evaluate(() => {
    document.documentElement.dataset.themePref = 'light'
    document.documentElement.dataset.theme = 'light'
  })
  await expect
    .poll(async () => fastLegendSwatch.evaluate((element) => getComputedStyle(element).backgroundColor))
    .toBe('rgb(3, 105, 161)')
  await expectSupportCanvasTheme(page, 'light')

  await guard.assertClean()
})

test('visual smoke: execution chart renders broker price levels and lifecycle telemetry @visual', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await page.setViewportSize({ width: 1600, height: 1000 })
  await gotoMockedShellWithScenario(page, context, executionChartVisualScenario)

  await action(page, 'shell.workflowStep.data').click()
  await action(page, 'data.sample.load').first().click()
  await expect
    .poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === 'load_price_data'))
    .toBe(true)

  await action(page, 'shell.workspace.live').click()
  await page.getByLabel('Execution target').selectOption('local_paper')
  await page.getByLabel('Symbol').fill('AAPL')
  await page.getByLabel('Symbol').press('Tab')
  await expect(action(page, 'live.account.refreshOrders')).toBeEnabled()
  await action(page, 'live.account.refreshOrders').click()
  await expect(page.getByText(/Loaded 6 recent orders for AAPL/i).first()).toBeVisible()
  await expect
    .poll(async () => {
      const invokes = await readTauriInvokeLog(page)
      return (
        invokes.some((entry) => entry.command === 'get_local_paper_execution_snapshot') &&
        invokes.some((entry) => entry.command === 'get_price_bars')
      )
    })
    .toBe(true)

  const chartPanel = page.locator('.priceChartPanel').filter({
    has: page.locator('#execTvChartVue'),
  })
  await expect(chartPanel.locator('.tvChartFrame')).toHaveAttribute('data-chart-ready', 'true')
  await expect(chartPanel.locator('.chartLegendOverlay')).toBeVisible()
  await expect(chartPanel.locator('.chartStateBadge')).toContainText(
    /6 markers · 5 broker price levels/,
  )
  await expect(chartPanel.locator('.chartEvidenceChips')).toContainText(/80 price bars/)
  await expect(chartPanel.locator('.chartEvidenceChips')).toContainText(/6 broker markers/)
  await expect(chartPanel.locator('.chartEvidenceChips')).toContainText(/5 broker price levels/)
  await expect(chartPanel.locator('.chartEvidenceChips')).toContainText(/1 filled/)
  await expect(chartPanel.locator('.chartEvidenceChips')).toContainText(/1 partial/)
  await expect(chartPanel.locator('.chartLegendOverlay')).toContainText(/Average entry/)
  await expect(chartPanel.locator('.chartLegendOverlay')).toContainText(/100\.65/)
  await expect(chartPanel.locator('.chartLegendOverlay')).toContainText(/Broker mark/)
  await expect(chartPanel.locator('.chartLegendOverlay')).toContainText(/101\.88/)
  await expect(chartPanel.locator('.chartLegendOverlay')).toContainText(/Stop/)
  await expect(chartPanel.locator('.chartLegendOverlay')).toContainText(/98\.7/)
  await expect(chartPanel.locator('.chartLegendOverlay')).toContainText(/Filled arrows/)
  await expect(chartPanel.locator('.chartLegendOverlay')).toContainText(/Partial arrows/)
  await expect(chartPanel.locator('.chartLegendOverlay')).toContainText(/Working circles/)
  await expect(chartPanel.locator('.chartLegendOverlay')).toContainText(/Closed squares/)
  await expect(chartPanel.locator('.chartLegendMore')).toHaveText('+3')
  await expect(page.locator('#execTvChartMetaVue')).toContainText(
    /5 broker-confirmed price levels displayed/,
  )
  const chartDescription = page.locator('#execTvChartMetaVueDescription')
  await expect(chartDescription).toContainText(/1 filled/)
  await expect(chartDescription).toContainText(/1 partial/)
  await expect(chartDescription).toContainText(/3 working/)
  await expect(chartDescription).toContainText(/1 closed/)
  await expect(page.locator('#execTvChartVue')).not.toHaveAttribute('aria-keyshortcuts', /./)
  await expect(chartPanel.locator('.chartInteractionHint')).toHaveCount(0)
  await expect
    .poll(async () => {
      const summary = await canvasPaintSummary(page, '#execTvChartVue')
      return summary.canvasCount > 0 && summary.paintedCanvasCount > 0 && summary.paintedSamples > 16
    })
    .toBe(true)
  const executionChartGeometry = await page.evaluate(() => {
    const scrollport = document.querySelector<HTMLElement>('#liveShellGrid')
    const frame = document.querySelector<HTMLElement>(
      '.priceChartPanel:has(#execTvChartVue) .tvChartFrame',
    )
    const host = document.querySelector<HTMLElement>('#execTvChartVue')
    const legend = frame?.querySelector<HTMLElement>('.chartLegendOverlay') ?? null
    if (!scrollport || !frame || !host || !legend) return null
    const portRect = scrollport.getBoundingClientRect()
    const fits = (rect: DOMRect) =>
      rect.left >= portRect.left - 1 && rect.right <= portRect.right + 1
    const canvasRects = Array.from(host.querySelectorAll('canvas')).map((canvas) =>
      canvas.getBoundingClientRect(),
    )
    const overflowing = Array.from(scrollport.querySelectorAll<HTMLElement>('*'))
      .map((element) => ({ element, rect: element.getBoundingClientRect() }))
      .filter(({ rect }) => rect.width > 0 && rect.right > portRect.right + 1)
      .slice(0, 12)
      .map(({ element, rect }) => ({
        tag: element.tagName.toLowerCase(),
        id: element.id,
        className: String(element.className || ''),
        left: Math.round(rect.left),
        right: Math.round(rect.right),
        width: Math.round(rect.width),
      }))
    return {
      scrollWidth: scrollport.scrollWidth,
      clientWidth: scrollport.clientWidth,
      frameFits: fits(frame.getBoundingClientRect()),
      hostFits: fits(host.getBoundingClientRect()),
      legendFits: fits(legend.getBoundingClientRect()),
      canvasesFit: canvasRects.length > 0 && canvasRects.every(fits),
      overflowing,
    }
  })
  expect(
    executionChartGeometry,
    `Execution chart must remain inside the live scrollport: ${JSON.stringify(executionChartGeometry)}`,
  ).toEqual({
    scrollWidth: executionChartGeometry?.clientWidth,
    clientWidth: executionChartGeometry?.clientWidth,
    frameFits: true,
    hostFits: true,
    legendFits: true,
    canvasesFit: true,
    overflowing: [],
  })

  const visualEvidenceDir = path.resolve(process.cwd(), '../output/playwright/visual-components')
  await fs.mkdir(visualEvidenceDir, { recursive: true })
  await chartPanel.locator('.tvChartFrame').screenshot({
    path: path.join(visualEvidenceDir, 'execution-broker-telemetry.png'),
  })
  await expectNoHorizontalDocumentOverflow(page)
  await guard.assertClean()
})

test('visual audit: heatmap renders high-coverage 2D surface with quality metadata @visual', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await page.setViewportSize({ width: 1600, height: 1000 })
  await gotoMockedShellWithScenario(page, context, heatmapLayoutScenario)
  await page.evaluate(() => {
    document.documentElement.dataset.themePref = 'light'
    document.documentElement.dataset.theme = 'light'
  })

  await action(page, 'shell.workflowStep.analyze').click()
  await action(page, 'shell.resultsTab.heatmap').click()
  await expect(page.locator('#results_panel_heatmap')).toBeVisible()
  await expect(page.locator('#heatmapCanvasStackVue')).toBeVisible()
  await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute('data-render-ready', 'true')
  await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute('data-total-cells', '1024')
  await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute('data-finite-coverage', /0\.\d+|1\.000/)
  await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute(
    'data-axis-summary',
    /Fast .* length \d+-\d+; Slow .* length \d+-\d+/,
  )
  await expect(page.locator('#heatmapDomainSummaryVue')).toContainText(/bins are unavailable/)
  await expect(page.locator('#heatmapFitModeBadgeVue')).toContainText(/valid domain/)
  await expect(page.locator('[data-qa-action="heatmap.reviewTop"]')).toBeVisible()
  await expect(page.locator('[data-qa-action="heatmap.exportPng"]')).toBeEnabled()
  const heatmapLegend = page.locator('.heatmapLegendBlock')
  await expect(heatmapLegend).toHaveAttribute('data-palette-kind', 'diverging')
  await expect(heatmapLegend.locator('.heatmapLegendSemantics')).toContainText(/Negative/)
  await expect(heatmapLegend.locator('.heatmapLegendSemantics')).toContainText(/Neutral · 0/)
  await expect(heatmapLegend.locator('.heatmapLegendSemantics')).toContainText(/Positive/)
  await expect(page.locator('#heatmapLegendMidVue')).toHaveText('0')
  const objectiveLightSamples = await heatmapLegendSamples(page)
  expect(objectiveLightSamples).toHaveLength(3)
  expect(
    new Set(objectiveLightSamples).size,
    'objective legend should expose visibly distinct negative, neutral, and positive anchors',
  ).toBe(3)

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
  await saveHeatmapSurfaceWithLegendPng(
    page,
    '#heatmapCanvasStackVue',
    path.join(visualEvidenceDir, 'heatmap-semantic-objective.png'),
  )

  await page.locator('#vueHeatmapViewMode').selectOption('3d')
  await expect(page.locator('#heatmapSurface3DViewportVue')).toBeVisible()
  await expect(page.locator('#heatmapSurface3DViewportVue')).toHaveAttribute('data-render-ready', 'true')
  await expect(page.locator('#heatmapSurface3DViewportVue')).toHaveAttribute('data-export-ready', 'true')
  await expect(heatmapLegend).toHaveAttribute('data-palette-kind', 'diverging')
  await expect(heatmapLegend.locator('.heatmapLegendSemantics')).toContainText(
    /Negative.*Neutral · 0.*Positive/,
  )
  await saveHeatmapSurfaceWithLegendPng(
    page,
    '#heatmapSurface3DViewportVue',
    path.join(visualEvidenceDir, 'heatmap-semantic-3d.png'),
  )

  await page.locator('#vueHeatmapViewMode').selectOption('2d')
  await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute('data-render-ready', 'true')
  await page.locator('#vueHeatmapMode').selectOption('plateau')
  await expect(heatmapLegend).toHaveAttribute('data-palette-kind', 'sequential')
  await expect(heatmapLegend.locator('.heatmapLegendSemantics')).toContainText(
    /Fragile.*Developing.*Resilient/,
  )
  await expect(page.locator('#heatmapLegendMinVue')).toHaveText('0')
  await expect(page.locator('#heatmapLegendMidVue')).toHaveText('0.5')
  await expect(page.locator('#heatmapLegendMaxVue')).toHaveText('1')
  const plateauLightSamples = await heatmapLegendSamples(page)
  expect(plateauLightSamples).toHaveLength(3)
  expect(
    new Set(plateauLightSamples).size,
    'plateau legend should expose visibly distinct fragile, developing, and resilient anchors',
  ).toBe(3)
  expect(plateauLightSamples).not.toEqual(objectiveLightSamples)
  await saveHeatmapSurfaceWithLegendPng(
    page,
    '#heatmapCanvasStackVue',
    path.join(visualEvidenceDir, 'heatmap-semantic-plateau.png'),
  )

  await page.evaluate(() => {
    document.documentElement.dataset.themePref = 'dark'
    document.documentElement.dataset.theme = 'dark'
  })
  await expect
    .poll(async () => heatmapLegendSamples(page))
    .not.toEqual(plateauLightSamples)
  await expect(heatmapLegend).toHaveAttribute('data-palette-kind', 'sequential')
  await expect(heatmapLegend.locator('.heatmapLegendSemantics')).toContainText(
    /Fragile.*Developing.*Resilient/,
  )
  await saveHeatmapSurfaceWithLegendPng(
    page,
    '#heatmapCanvasStackVue',
    path.join(visualEvidenceDir, 'heatmap-semantic-plateau-dark.png'),
  )

  await guard.assertClean()
})
