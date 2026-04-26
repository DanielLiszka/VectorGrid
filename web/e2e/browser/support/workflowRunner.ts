import { expect, test, type BrowserContext, type Locator, type Page } from '@playwright/test'
import { dismissStartupOverlay, installMockedTauri } from './mockTauri'
import { waitForShellReady } from './appHarness'
import { installQaGuards, type QaGuard } from './qaAssertions'
import type { MockTauriScenario } from '../fixtures/scenarios'

export interface WorkflowContext {
  page: Page
  context: BrowserContext
  guard: QaGuard
  action: (id: string) => Locator
  state: (id: string) => Locator
  panel: (id: string) => Locator
}

export interface WorkflowCase {
  id: string
  title: string
  tags: string[]
  scenario: MockTauriScenario
  viewport?: { width: number; height: number }
  run: (ctx: WorkflowContext) => Promise<void>
}

export async function gotoMockedShellWithScenario(
  page: Page,
  context: BrowserContext,
  scenario: MockTauriScenario,
): Promise<void> {
  await context.addInitScript(({ localStorageValues, sessionStorageValues }) => {
    for (const [key, value] of Object.entries(localStorageValues || {})) {
      try {
        localStorage.setItem(key, String(value))
      } catch {
        // Ignore storage failures in the browser harness.
      }
    }
    for (const [key, value] of Object.entries(sessionStorageValues || {})) {
      try {
        sessionStorage.setItem(key, String(value))
      } catch {
        // Ignore storage failures in the browser harness.
      }
    }
  }, {
    localStorageValues: scenario.localStorage ?? {},
    sessionStorageValues: scenario.sessionStorage ?? {},
  })
  await installMockedTauri(context, {
    holdStartupOverlay: true,
    commandResults: scenario.commandResults,
  })
  await page.goto('/', { waitUntil: 'networkidle' })
  await dismissStartupOverlay(page)
  await waitForShellReady(page)
}

export function defineWorkflow(testCase: WorkflowCase): void {
  test(`${testCase.title} ${testCase.tags.join(' ')}`, async ({ page, context }) => {
    if (testCase.viewport) await page.setViewportSize(testCase.viewport)
    const guard = installQaGuards(page)
    await gotoMockedShellWithScenario(page, context, testCase.scenario)

    const ctx: WorkflowContext = {
      page,
      context,
      guard,
      action: (id) => page.locator(`[data-qa-action="${id}"]`),
      state: (id) => page.locator(`[data-qa-state="${id}"]`),
      panel: (id) => page.locator(`[data-qa-panel="${id}"]`),
    }

    await testCase.run(ctx)
    await guard.assertClean()
  })
}

export async function expectActionVisible(page: Page, actionId: string): Promise<void> {
  await expect(page.locator(`[data-qa-action="${actionId}"]`).first()).toBeVisible()
}
