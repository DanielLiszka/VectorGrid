import { expect, type Page } from '@playwright/test'
import fs from 'node:fs/promises'
import path from 'node:path'
import { emptyAppScenario, heatmapLayoutScenario } from '../fixtures'
import { defineWorkflow } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow } from '../support/qaAssertions'

interface LayoutRect {
  left: number
  right: number
  top: number
  bottom: number
  width: number
  height: number
}

interface NavTarget {
  label: string
  selector: string
  visibleInViewport: boolean
}

interface ShellLayoutSnapshot {
  viewport: { width: number; height: number }
  titlebar: LayoutRect | null
  header: LayoutRect | null
  main: LayoutRect | null
  footer: LayoutRect | null
  workbenchInspector: LayoutRect | null
  visibleChromeHeight: number | null
  mainVisibleHeight: number | null
  navTargets: NavTarget[]
}

interface LayoutBudget {
  maxChrome: number
  minMainVisible: number
  maxFooter?: number
}

interface ScrollMetrics {
  clientHeight: number
  scrollHeight: number
  scrollTop: number
}

interface ResearchDataLayoutSnapshot {
  main: LayoutRect | null
  cockpit: LayoutRect | null
  paneGrid: LayoutRect | null
  setupLane: LayoutRect | null
  resultsLane: LayoutRect | null
  csvPath: LayoutRect | null
  setupScroll: ScrollMetrics | null
  mainScroll: ScrollMetrics | null
}

const MIN_WORKING_LANE_HEIGHT = 180
const MIN_CSV_PATH_WIDTH = 160

async function readShellLayout(page: Page): Promise<ShellLayoutSnapshot> {
  return page.evaluate(() => {
    const readRect = (selector: string) => {
      const element = document.querySelector(selector)
      if (!element) return null
      const style = window.getComputedStyle(element)
      const rect = element.getBoundingClientRect()
      if (style.display === 'none' || style.visibility === 'hidden' || rect.width <= 0 || rect.height <= 0) return null
      return {
        left: rect.left,
        right: rect.right,
        top: rect.top,
        bottom: rect.bottom,
        width: rect.width,
        height: rect.height,
      }
    }
    const titlebar = readRect('[aria-label="Desktop titlebar"]')
    const header = readRect('[aria-label="Workspace shell header"]')
    const main = readRect('#shellMainPanel')
    const footer = readRect('#runFooterShell')
    const navTargets = Array.from(
      document.querySelectorAll('#workspaceBar .railButton, #modeBar .tabBtn, #workflowStepBar .stageButton'),
    ).map((element) => {
      const rect = element.getBoundingClientRect()
      return {
        label: String(element.textContent || '').trim(),
        selector: [
          element.closest('nav')?.id || 'unknown-nav',
          (element as HTMLElement).dataset.qaAction || (element as HTMLElement).dataset.workflowStep || 'unknown-action',
        ].join(':'),
        visibleInViewport:
          rect.width > 0
          && rect.height > 0
          && rect.left >= -0.5
          && rect.right <= window.innerWidth + 0.5
          && rect.top >= -0.5
          && rect.bottom <= window.innerHeight + 0.5,
      }
    })
    return {
      viewport: { width: window.innerWidth, height: window.innerHeight },
      titlebar,
      header,
      main,
      footer,
      workbenchInspector: readRect('[aria-label="Workbench inspector"]'),
      visibleChromeHeight: main ? main.top : null,
      mainVisibleHeight: main ? Math.max(0, Math.min(main.bottom, window.innerHeight) - Math.max(main.top, 0)) : null,
      navTargets,
    }
  })
}

async function readResearchDataLayout(page: Page): Promise<ResearchDataLayoutSnapshot> {
  return page.evaluate(() => {
    const readRect = (selector: string) => {
      const element = document.querySelector(selector)
      if (!element) return null
      const style = window.getComputedStyle(element)
      const rect = element.getBoundingClientRect()
      if (style.display === 'none' || style.visibility === 'hidden' || rect.width <= 0 || rect.height <= 0) return null
      return {
        left: rect.left,
        right: rect.right,
        top: rect.top,
        bottom: rect.bottom,
        width: rect.width,
        height: rect.height,
      }
    }
    const readScrollMetrics = (selector: string) => {
      const element = document.querySelector<HTMLElement>(selector)
      if (!element) return null
      return {
        clientHeight: element.clientHeight,
        scrollHeight: element.scrollHeight,
        scrollTop: element.scrollTop,
      }
    }

    return {
      main: readRect('#shellMainPanel'),
      cockpit: readRect('[data-qa-panel="workbench.researchCockpit"]'),
      paneGrid: readRect('.workbenchPaneGrid'),
      setupLane: readRect('[aria-label="Setup pane"]'),
      resultsLane: readRect('[aria-label="Results pane"]'),
      csvPath: readRect('#vueCsvPath'),
      setupScroll: readScrollMetrics('[aria-label="Setup pane"]'),
      mainScroll: readScrollMetrics('#shellMainPanel'),
    }
  })
}

function expectLayoutBudget(snapshot: ShellLayoutSnapshot, budget: LayoutBudget): void {
  expect(snapshot.titlebar, 'desktop titlebar should be measurable').not.toBeNull()
  expect(snapshot.header, 'workspace shell header should be measurable').not.toBeNull()
  expect(snapshot.main, 'shell main panel should be measurable').not.toBeNull()
  expect(snapshot.footer, 'run footer should be measurable').not.toBeNull()
  expect(snapshot.visibleChromeHeight, 'visible chrome height').toBeLessThanOrEqual(budget.maxChrome)
  expect(snapshot.mainVisibleHeight, 'visible main content height').toBeGreaterThanOrEqual(budget.minMainVisible)
  if (budget.maxFooter !== undefined) {
    expect(snapshot.footer?.height ?? 0, 'footer height').toBeLessThanOrEqual(budget.maxFooter)
  }
}

function expectVisibleNavTargets(snapshot: ShellLayoutSnapshot): void {
  const hiddenTargets = snapshot.navTargets
    .filter((target) => !target.visibleInViewport)
    .map((target) => `${target.selector} (${target.label})`)
  expect(hiddenTargets, 'shell navigation targets outside viewport').toEqual([])
}

function expectNoInspectorOverlap(inspector: LayoutRect, reference: LayoutRect, label: string): void {
  const overlaps =
    inspector.left < reference.right
    && inspector.right > reference.left
    && inspector.top < reference.bottom
    && inspector.bottom > reference.top
  expect(overlaps, `${label} should not overlap the workbench inspector`).toBe(false)
}

function expectRectInside(inner: LayoutRect, outer: LayoutRect, label: string, tolerance = 0.5): void {
  expect(inner.left, `${label} left edge`).toBeGreaterThanOrEqual(outer.left - tolerance)
  expect(inner.right, `${label} right edge`).toBeLessThanOrEqual(outer.right + tolerance)
  expect(inner.top, `${label} top edge`).toBeGreaterThanOrEqual(outer.top - tolerance)
  expect(inner.bottom, `${label} bottom edge`).toBeLessThanOrEqual(outer.bottom + tolerance)
}

async function expectResearchDataLayoutUsable(page: Page, scrollingMain = false): Promise<void> {
  await expect(page.locator('[data-qa-panel="workbench.researchCockpit"]')).toBeVisible()
  await expect(page.locator('[aria-label="Setup pane"]')).toBeVisible()
  await expect(page.locator('[aria-label="Results pane"]')).toBeVisible()
  await expect(page.locator('#vueCsvPath')).toBeAttached()
  await expectNoHorizontalDocumentOverflow(page)

  const snapshot = await readResearchDataLayout(page)
  expect(snapshot.main, 'Research main panel should be measurable').not.toBeNull()
  expect(snapshot.cockpit, 'Research cockpit should be measurable').not.toBeNull()
  expect(snapshot.paneGrid, 'Research pane grid should be measurable').not.toBeNull()
  expect(snapshot.setupLane, 'Research setup lane should be measurable').not.toBeNull()
  expect(snapshot.resultsLane, 'Research results lane should be measurable').not.toBeNull()
  expect(snapshot.csvPath, 'CSV path input should be measurable').not.toBeNull()
  expect(snapshot.setupScroll, 'Research setup scroll metrics should be measurable').not.toBeNull()
  expect(snapshot.mainScroll, 'Research main scroll metrics should be measurable').not.toBeNull()

  const main = snapshot.main as LayoutRect
  const cockpit = snapshot.cockpit as LayoutRect
  const paneGrid = snapshot.paneGrid as LayoutRect
  const setupLane = snapshot.setupLane as LayoutRect
  const resultsLane = snapshot.resultsLane as LayoutRect
  const csvPath = snapshot.csvPath as LayoutRect
  const setupScroll = snapshot.setupScroll as ScrollMetrics
  const mainScroll = snapshot.mainScroll as ScrollMetrics

  expectRectInside(cockpit, main, 'Research cockpit', 8)
  if (scrollingMain) {
    expect(mainScroll.scrollHeight, 'Compact Research main should scroll to keep both lanes usable').toBeGreaterThan(mainScroll.clientHeight)
  } else {
    expectRectInside(paneGrid, main, 'Research pane grid')
  }
  expectRectInside(setupLane, paneGrid, 'Research setup lane')
  expectRectInside(resultsLane, paneGrid, 'Research results lane')
  expect(paneGrid.top, 'Research pane grid should start below the cockpit').toBeGreaterThanOrEqual(cockpit.bottom - 0.5)
  expect(setupLane.height, 'Research setup lane usable height').toBeGreaterThanOrEqual(MIN_WORKING_LANE_HEIGHT)
  expect(resultsLane.height, 'Research results lane usable height').toBeGreaterThanOrEqual(MIN_WORKING_LANE_HEIGHT)
  expect(csvPath.width, 'CSV path input usable width').toBeGreaterThanOrEqual(MIN_CSV_PATH_WIDTH)
  expect(setupScroll.scrollHeight, 'Data Source form should own vertical scrolling inside the setup lane').toBeGreaterThan(
    setupScroll.clientHeight,
  )

  const bottomAction = page.locator('[data-qa-action="data.storage.manageSettings"]')
  await bottomAction.scrollIntoViewIfNeeded()
  await expect(bottomAction).toBeVisible()
  const reachability = await page.evaluate(() => {
    const lane = document.querySelector<HTMLElement>('[aria-label="Setup pane"]')
    const action = document.querySelector<HTMLElement>('[data-qa-action="data.storage.manageSettings"]')
    if (!lane || !action) return null
    const laneRect = lane.getBoundingClientRect()
    const actionRect = action.getBoundingClientRect()
    return {
      lane: {
        left: laneRect.left,
        right: laneRect.right,
        top: laneRect.top,
        bottom: laneRect.bottom,
        width: laneRect.width,
        height: laneRect.height,
      },
      action: {
        left: actionRect.left,
        right: actionRect.right,
        top: actionRect.top,
        bottom: actionRect.bottom,
        width: actionRect.width,
        height: actionRect.height,
      },
      scrollTop: lane.scrollTop,
    }
  })
  expect(reachability, 'Bottom Data Source action should be measurable after scrolling').not.toBeNull()
  expect((reachability as { scrollTop: number }).scrollTop, 'Setup lane should scroll to reach lower Data Source controls').toBeGreaterThan(0)
  expectRectInside(
    (reachability as { action: LayoutRect }).action,
    (reachability as { lane: LayoutRect }).lane,
    'Bottom Data Source action after scrolling',
  )

  if (scrollingMain) {
    const results = page.locator('[aria-label="Results pane"]')
    await results.scrollIntoViewIfNeeded()
    await expect(results).toBeVisible()
    const visibleResults = await readResearchDataLayout(page)
    expectRectInside(
      visibleResults.resultsLane as LayoutRect,
      visibleResults.main as LayoutRect,
      'Compact Research results lane after scrolling',
      1,
    )
  }
}

async function expectCanvasHasPaint(page: Page): Promise<void> {
  const painted = await page.locator('#heatmapCanvasVue').evaluate((node) => {
    const canvas = node as HTMLCanvasElement
    const ctx = canvas.getContext('2d', { willReadFrequently: true })
    if (!ctx || canvas.width <= 0 || canvas.height <= 0) return false
    const image = ctx.getImageData(0, 0, canvas.width, canvas.height)
    for (let index = 3; index < image.data.length; index += 128) {
      if (image.data[index] > 0) return true
    }
    return false
  })
  expect(painted, 'heatmap canvas should contain painted pixels').toBe(true)
}

async function expectWebglCanvasHasPaint(page: Page): Promise<void> {
  const painted = await page.locator('#heatmapSurface3DCanvasVue').evaluate((node) => {
    const canvas = node as HTMLCanvasElement
    const gl =
      canvas.getContext('webgl2', { preserveDrawingBuffer: true }) ||
      canvas.getContext('webgl', { preserveDrawingBuffer: true })
    if (!gl || canvas.width <= 0 || canvas.height <= 0) return false
    const pixels = new Uint8Array(canvas.width * canvas.height * 4)
    gl.readPixels(0, 0, canvas.width, canvas.height, gl.RGBA, gl.UNSIGNED_BYTE, pixels)
    let paintedSamples = 0
    for (let index = 0; index < pixels.length; index += 96) {
      const alpha = pixels[index + 3] ?? 0
      const rgb = (pixels[index] ?? 0) + (pixels[index + 1] ?? 0) + (pixels[index + 2] ?? 0)
      if (alpha > 0 && rgb > 0) paintedSamples += 1
      if (paintedSamples > 24) return true
    }
    return false
  })
  expect(painted, '3D heatmap WebGL canvas should contain painted pixels').toBe(true)
}

for (const viewport of [
  { id: 'default', label: 'default desktop', width: 1280, height: 720 },
  { id: 'minimum', label: 'minimum desktop', width: 1024, height: 640 },
]) {
  defineWorkflow({
    id: `layout.desktop.researchData.${viewport.id}`,
    title: `keeps Research Data Source usable at the ${viewport.label} size`,
    tags: ['@workflow', '@responsive', '@layout'],
    viewport: { width: viewport.width, height: viewport.height },
    scenario: emptyAppScenario,
    run: async ({ page, action }) => {
      await action('shell.workspace.workflow').click()
      await action('shell.workflowStep.data').click()
      await expect(page.locator('#controls_panel_data')).toBeAttached()
      await expect(page.locator('#controls_panel_data')).toHaveAttribute('aria-hidden', 'false')
      await expect(page.locator('.dataPanelVueRoot')).toBeAttached()
      await expectResearchDataLayoutUsable(page)
      await expect(page.locator('#workflowStepBar')).toBeVisible()
      await expect(page.locator('#runFooterShell')).toBeVisible()
    },
  })
}

defineWorkflow({
  id: 'layout.mobile.researchData',
  title: 'keeps the Data Source workspace usable on a compact mobile viewport',
  tags: ['@workflow', '@responsive', '@layout'],
  viewport: { width: 390, height: 844 },
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.workflow').click()
    await action('shell.workflowStep.data').click()
    await expectResearchDataLayoutUsable(page, true)
  },
})

defineWorkflow({
  id: 'layout.wide.researchData',
  title: 'uses the available Research workspace on a wide desktop',
  tags: ['@workflow', '@responsive', '@layout'],
  viewport: { width: 2400, height: 1200 },
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.workflow').click()
    await action('shell.workflowStep.data').click()
    await expectResearchDataLayoutUsable(page)
    const snapshot = await readResearchDataLayout(page)
    const main = snapshot.main as LayoutRect
    const paneGrid = snapshot.paneGrid as LayoutRect
    const setupLane = snapshot.setupLane as LayoutRect
    const resultsLane = snapshot.resultsLane as LayoutRect
    expect(main.width / 2400, 'main workbench should consume the wide viewport').toBeGreaterThan(0.78)
    expect(paneGrid.width / main.width, 'pane grid should fill the wide main workbench').toBeGreaterThan(0.98)
    expect(setupLane.width, 'wide setup lane should grow beyond its minimum').toBeGreaterThan(540)
    expect(resultsLane.width, 'wide results lane should use the remaining workspace').toBeGreaterThan(1000)
  },
})

defineWorkflow({
  id: 'layout.compact.inspectorDrawer',
  title: 'opens the inspector as an explicit contained drawer on compact desktop',
  tags: ['@workflow', '@responsive', '@layout'],
  viewport: { width: 1024, height: 640 },
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    const inspector = page.locator('[aria-label="Workbench inspector"]')
    await expect(inspector).toBeHidden()
    await action('workbench.inspector.compactOpen').click()
    await expect(inspector).toBeVisible()
    await expect.poll(async () => inspector.evaluate((element) => element.getBoundingClientRect().right)).toBeLessThanOrEqual(1024)
    const rect = await inspector.evaluate((element) => {
      const bounds = element.getBoundingClientRect()
      return { left: bounds.left, right: bounds.right, top: bounds.top, bottom: bounds.bottom }
    })
    expect(rect.left).toBeGreaterThanOrEqual(0)
    expect(rect.right).toBeLessThanOrEqual(1024)
    expect(rect.top).toBeGreaterThanOrEqual(0)
    expect(rect.bottom).toBeLessThanOrEqual(640)
    await action('workbench.inspector.toggle').click()
    await expect(inspector).toBeHidden()
  },
})

defineWorkflow({
  id: 'layout.mobile.workflow',
  title: 'keeps mobile workflow chrome within budget with visible navigation',
  tags: ['@workflow', '@responsive', '@layout'],
  viewport: { width: 390, height: 844 },
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await expectNoHorizontalDocumentOverflow(page)
    let snapshot = await readShellLayout(page)
    expectLayoutBudget(snapshot, { maxChrome: 360, minMainVisible: 360, maxFooter: 130 })
    expectVisibleNavTargets(snapshot)

    await action('shell.workflowStep.validate').click()
    await expect(page.locator('[data-qa-action="validation.run"]')).toBeVisible()
    await expectNoHorizontalDocumentOverflow(page)
    snapshot = await readShellLayout(page)
    expectLayoutBudget(snapshot, { maxChrome: 360, minMainVisible: 380, maxFooter: 120 })
    expectVisibleNavTargets(snapshot)
  },
})

defineWorkflow({
  id: 'layout.desktop.validation',
  title: 'keeps desktop validation layout clear of duplicate inspector context',
  tags: ['@workflow', '@responsive', '@layout'],
  viewport: { width: 1720, height: 1080 },
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workflowStep.validate').click()
    await expect(page.locator('[data-qa-action="validation.run"]')).toBeVisible()
    await expectNoHorizontalDocumentOverflow(page)
    const snapshot = await readShellLayout(page)
    expectLayoutBudget(snapshot, { maxChrome: 260, minMainVisible: 720, maxFooter: 80 })
    expectVisibleNavTargets(snapshot)
  },
})

defineWorkflow({
  id: 'layout.desktop.heatmapSurface',
  title: 'lets desktop heatmap surfaces use available panel space and labels masked bins',
  tags: ['@workflow', '@responsive', '@layout'],
  viewport: { width: 1600, height: 1000 },
  scenario: heatmapLayoutScenario,
  run: async ({ page, action }) => {
    await action('shell.workflowStep.analyze').click()
    await action('shell.resultsTab.heatmap').click()
    await expect(page.locator('#results_panel_heatmap')).toBeVisible()
    await expect(page.locator('#heatmapCanvasStackVue')).toBeVisible()
    await expect(page.locator('#heatmapDomainSummaryVue')).toContainText(/bins are unavailable/)
    await expect(page.locator('#vueHeatmapFitMode')).toHaveValue('valid')
    await expect(page.locator('#vueHeatmap2dSource')).toHaveValue('slice')
    await expect(page.locator('#vueHeatmapSliceSelect')).toBeVisible()
    await expect(page.locator('#heatmapMarginalSummaryVue')).toContainText(/Slice|Coverage/)

    await page.locator('#vueHeatmap2dSource').selectOption('delta')
    await expect(page.locator('#vueHeatmapBaselineSliceSelect')).toBeVisible()
    await expect(page.locator('#heatmapMetaVue')).toContainText(/delta heatmap/)
    await expect(page.locator('#heatmapMarginalSummaryVue')).toContainText(/Best Δ/)

    const geometry = await page.locator('#heatmapCanvasStackVue').evaluate((element) => {
      const rect = element.getBoundingClientRect()
      const panelRect = document.querySelector('#results_panel_heatmap')?.getBoundingClientRect()
      return {
        width: rect.width,
        height: rect.height,
        right: rect.right,
        panelWidth: panelRect?.width ?? window.innerWidth,
        panelRight: panelRect?.right ?? window.innerWidth,
      }
    })

    expect(geometry.width, 'heatmap should grow beyond the old 512px canvas footprint').toBeGreaterThanOrEqual(680)
    expect(
      geometry.width / Math.max(1, geometry.panelWidth),
      'heatmap should use most of the available results panel width',
    ).toBeGreaterThanOrEqual(0.86)
    expect(Math.abs(geometry.width - geometry.height), 'heatmap should remain square').toBeLessThanOrEqual(2)
    expect(geometry.right, 'heatmap should stay inside its results panel').toBeLessThanOrEqual(geometry.panelRight + 0.5)
    await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute('data-render-ready', 'true')
    await expect(page.locator('#heatmapCanvasStackVue')).toHaveAttribute('data-finite-coverage', /0\.\d+|1\.000/)
    await expectCanvasHasPaint(page)
    await expectNoHorizontalDocumentOverflow(page)

    await action('heatmap.reviewTop').click()
    await expect(page.locator('#results_panel_top')).toBeVisible()
    await action('shell.resultsTab.heatmap').click()
    await expect(page.locator('#results_panel_heatmap')).toBeVisible()

    await page.locator('#vueHeatmapViewMode').selectOption('3d')
    await expect(page.locator('#heatmapSurface3DViewportVue')).toBeVisible()
    await expect(page.locator('#heatmapSurface3DSliceSelectVue')).toBeVisible()
    await expect(page.locator('#heatmapSurface3DMetaVue')).toContainText(/MA type \/ parameter slice/)
    await expect(page.locator('#heatmapSurface3DViewportVue')).toHaveAttribute('data-render-ready', 'true')
    await expect(page.locator('#heatmapSurface3DViewportVue')).toHaveAttribute('data-export-ready', 'true')
    await expect(page.locator('[data-qa-action="heatmap.exportPng"]')).toBeEnabled()
    await expect(page.locator('#heatmapSurface3DCanvasVue')).toBeVisible()
    await expectWebglCanvasHasPaint(page)
    await expectNoHorizontalDocumentOverflow(page)
    await page.locator('#heatmapSurface3DSliceSelectVue').scrollIntoViewIfNeeded()
    await expect(page.locator('#heatmapSurface3DSliceSelectVue')).toBeVisible()
    const visualEvidenceDir = path.resolve(process.cwd(), '../output/playwright/heatmap-visual')
    await fs.mkdir(visualEvidenceDir, { recursive: true })
    await page.screenshot({
      path: path.join(visualEvidenceDir, 'heatmap-3d-slices.png'),
      fullPage: true,
    })
  },
})

defineWorkflow({
  id: 'layout.desktop.workbenchHistoryInspector',
  title: 'keeps desktop workbench history inspector outside the main panel and footer',
  tags: ['@workflow', '@responsive', '@layout'],
  viewport: { width: 1366, height: 768 },
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.library').click()
    await expect(page.locator('[aria-label="Workbench inspector"]')).toBeVisible()
    await expectNoHorizontalDocumentOverflow(page)
    const snapshot = await readShellLayout(page)
    expectLayoutBudget(snapshot, { maxChrome: 220, minMainVisible: 500, maxFooter: 40 })
    expectVisibleNavTargets(snapshot)
    expect(snapshot.workbenchInspector, 'workbench inspector should be visible on desktop library').not.toBeNull()
    expect(snapshot.main, 'main panel should be available for inspector comparison').not.toBeNull()
    expect(snapshot.footer, 'footer should be available for inspector comparison').not.toBeNull()
    expect(snapshot.header, 'header should be available for inspector comparison').not.toBeNull()
    const inspector = snapshot.workbenchInspector as LayoutRect
    expect(inspector.left, 'workbench inspector should sit to the right of the library panel').toBeGreaterThanOrEqual(
      (snapshot.main as LayoutRect).right - 0.5,
    )
    expect(inspector.top, 'workbench inspector should sit below the desktop titlebar').toBeGreaterThanOrEqual(
      (snapshot.titlebar as LayoutRect).bottom - 0.5,
    )
    expect(inspector.right, 'workbench inspector should stay inside viewport').toBeLessThanOrEqual(snapshot.viewport.width)
    expect(inspector.bottom, 'workbench inspector should stay above footer').toBeLessThanOrEqual(
      (snapshot.footer as LayoutRect).top,
    )
    expectNoInspectorOverlap(inspector, snapshot.main as LayoutRect, 'main panel')
    expectNoInspectorOverlap(inspector, snapshot.footer as LayoutRect, 'footer')
  },
})

defineWorkflow({
  id: 'layout.mobile.library',
  title: 'keeps mobile library usable without a persistent inspector overlay',
  tags: ['@workflow', '@responsive', '@layout'],
  viewport: { width: 390, height: 844 },
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.library').click()
    await expect(page.locator('[aria-label="Workbench inspector"]')).toBeHidden()
    await expectNoHorizontalDocumentOverflow(page)
    const snapshot = await readShellLayout(page)
    expectLayoutBudget(snapshot, { maxChrome: 300, minMainVisible: 500, maxFooter: 40 })
    expectVisibleNavTargets(snapshot)
  },
})
