import { expect, type Page } from '@playwright/test'
import { emptyAppScenario, legacyShellScenario } from '../fixtures/scenarios'
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
  validationDock: LayoutRect | null
  historyDock: LayoutRect | null
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
      document.querySelectorAll('#uiModeBar .tabBtn, #workspaceBar .tabBtn, #modeBar .tabBtn, #workflowStepBar .tabBtn'),
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
      validationDock: readRect('.valDock'),
      historyDock: readRect('.historyDock'),
      workbenchInspector: readRect('[aria-label="Workbench inspector"]'),
      visibleChromeHeight: main ? main.top : null,
      mainVisibleHeight: main ? Math.max(0, Math.min(main.bottom, window.innerHeight) - Math.max(main.top, 0)) : null,
      navTargets,
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

function expectNoDockOverlap(dock: LayoutRect, reference: LayoutRect, label: string, dockLabel = 'dock'): void {
  const overlaps =
    dock.left < reference.right
    && dock.right > reference.left
    && dock.top < reference.bottom
    && dock.bottom > reference.top
  expect(overlaps, `${label} should not overlap the ${dockLabel}`).toBe(false)
}

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
    expect(snapshot.validationDock, 'validation dock should not be visible on data step').toBeNull()

    await action('shell.workflowStep.validate').click()
    await expect(page.locator('[data-qa-action="validation.run"]')).toBeVisible()
    await expectNoHorizontalDocumentOverflow(page)
    snapshot = await readShellLayout(page)
    expectLayoutBudget(snapshot, { maxChrome: 360, minMainVisible: 380, maxFooter: 120 })
    expectVisibleNavTargets(snapshot)
    expect(snapshot.validationDock, 'validation dock should not duplicate the validation workspace').toBeNull()
  },
})

defineWorkflow({
  id: 'layout.desktop.validation',
  title: 'keeps desktop validation layout clear of duplicate validation dock',
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
    expect(snapshot.validationDock, 'validation dock should not duplicate the validation workspace').toBeNull()
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
    expect(snapshot.historyDock, 'legacy history dock should not duplicate the workbench inspector').toBeNull()
    expect(snapshot.workbenchInspector, 'workbench inspector should be visible on desktop library').not.toBeNull()
    expect(snapshot.main, 'main panel should be available for inspector comparison').not.toBeNull()
    expect(snapshot.footer, 'footer should be available for inspector comparison').not.toBeNull()
    expect(snapshot.header, 'header should be available for inspector comparison').not.toBeNull()
    const historyDock = snapshot.workbenchInspector as LayoutRect
    expect(historyDock.left, 'workbench inspector should sit to the right of the library panel').toBeGreaterThanOrEqual(
      (snapshot.main as LayoutRect).right - 0.5,
    )
    expect(historyDock.top, 'workbench inspector should sit below the desktop titlebar').toBeGreaterThanOrEqual(
      (snapshot.titlebar as LayoutRect).bottom - 0.5,
    )
    expect(historyDock.right, 'workbench inspector should stay inside viewport').toBeLessThanOrEqual(snapshot.viewport.width)
    expect(historyDock.bottom, 'workbench inspector should stay above footer').toBeLessThanOrEqual(
      (snapshot.footer as LayoutRect).top,
    )
    expectNoDockOverlap(historyDock, snapshot.main as LayoutRect, 'main panel', 'workbench inspector')
    expectNoDockOverlap(historyDock, snapshot.footer as LayoutRect, 'footer', 'workbench inspector')
  },
})

defineWorkflow({
  id: 'layout.desktop.legacyHistoryDockRollback',
  title: 'keeps desktop legacy shell rollback path measurable while workbench is default',
  tags: ['@workflow', '@responsive', '@layout'],
  viewport: { width: 1366, height: 768 },
  scenario: legacyShellScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.library').click()
    await expect(page.locator('[aria-label="History context dock"]')).toBeVisible()
    await expect(page.locator('[aria-label="Workbench inspector"]')).toBeHidden()
    await expectNoHorizontalDocumentOverflow(page)
    const snapshot = await readShellLayout(page)
    expectLayoutBudget(snapshot, { maxChrome: 260, minMainVisible: 500, maxFooter: 80 })
    expectVisibleNavTargets(snapshot)
    expect(snapshot.historyDock, 'legacy history dock should remain available on rollback path').not.toBeNull()
    expect(snapshot.workbenchInspector, 'workbench inspector should not render on legacy rollback path').toBeNull()
  },
})

defineWorkflow({
  id: 'layout.mobile.library',
  title: 'keeps mobile library usable without a fixed history overlay',
  tags: ['@workflow', '@responsive', '@layout'],
  viewport: { width: 390, height: 844 },
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.workspace.library').click()
    await expect(page.locator('[aria-label="History context dock"]')).toBeHidden()
    await expectNoHorizontalDocumentOverflow(page)
    const snapshot = await readShellLayout(page)
    expectLayoutBudget(snapshot, { maxChrome: 300, minMainVisible: 500, maxFooter: 40 })
    expectVisibleNavTargets(snapshot)
    expect(snapshot.historyDock, 'history dock should not consume mobile viewport space').toBeNull()
  },
})
