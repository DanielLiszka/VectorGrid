import AxeBuilder from '@axe-core/playwright'
import { expect, test, type Page } from '@playwright/test'
import {
  aggregateMetaReadyScenario,
  emptyAppScenario,
  optimizationSuccessScenario,
  optionsValidationReadyScenario,
} from '../fixtures'
import { gotoMockedShellWithScenario, readTauriInvokeLog } from '../support/workflowRunner'
import { installQaGuards } from '../support/qaAssertions'

async function expectAccessible(page: Page, name: string): Promise<void> {
  const result = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa'])
    .analyze()
  const summary = result.violations.map((violation) => ({
    id: violation.id,
    impact: violation.impact,
    targets: violation.nodes.slice(0, 4).map((node) => ({
      selector: node.target.join(' '),
      summary: node.failureSummary,
    })),
  }))
  expect(summary, `${name} accessibility violations`).toEqual([])
}

async function loadAndOptimize(page: Page): Promise<void> {
  await page.locator('[data-qa-action="shell.workflowStep.data"]').click()
  await page.locator('[data-qa-action="data.sample.load"]').first().click()
  await expect.poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === 'load_price_data')).toBe(true)
  await page.locator('[data-qa-action="shell.workflowStep.optimize"]').click()
  await expect(page.locator('[data-qa-action="optimize.run"]')).toBeEnabled()
  await page.locator('[data-qa-action="optimize.run"]').click()
  await expect.poll(async () => (await readTauriInvokeLog(page)).some((entry) => entry.command === 'run_double_ma_optimization')).toBe(true)
}

test('scans populated Top, History, and Final Review surfaces @workflow @a11y', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await gotoMockedShellWithScenario(page, context, optimizationSuccessScenario)
  await loadAndOptimize(page)

  await page.locator('[data-qa-action="shell.resultsTab.top"]').click()
  await expect(page.locator('.topTableVue tbody tr').first()).toBeVisible()
  await expectAccessible(page, 'populated Top results')

  await page.locator('[data-qa-action="shell.workspace.library"]').click()
  await expect(page.locator('#results_panel_history')).toBeVisible()
  await expectAccessible(page, 'populated History')

  await page.locator('[data-qa-action="shell.workspace.workflow"]').click()
  await page.locator('[data-qa-action="shell.workflowStep.finalize"]').click()
  await expect(page.locator('#results_panel_final')).toBeVisible()
  await expectAccessible(page, 'populated Final Review')
  await guard.assertClean()
})

test('scans populated Aggregate and Portfolio surfaces @workflow @a11y', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await gotoMockedShellWithScenario(page, context, aggregateMetaReadyScenario)

  await page.locator('[data-qa-action="shell.workspace.aggregate"]').click()
  await expect(page.locator('#results_panel_meta')).toBeVisible()
  await expectAccessible(page, 'populated Aggregate')

  await page.locator('[data-qa-action="shell.workspace.portfolio"]').click()
  await expect(page.locator('#portfolioShellGrid')).toBeVisible()
  await expectAccessible(page, 'populated Portfolio')
  await guard.assertClean()
})

test('scans Options stages with a saved artifact @workflow @a11y', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await gotoMockedShellWithScenario(page, context, optionsValidationReadyScenario)

  await page.locator('[data-qa-action="shell.workspace.library"]').click()
  await page.locator('[data-qa-action="library.savedRun.refresh"]').click()
  await page.getByRole('button', { name: /DoubleMa/i }).first().click()
  await expect(page.locator('[aria-label="Options expression follow-up"]')).toBeVisible()
  await page.locator('[data-qa-action="options.artifacts.refresh"]').click()
  await expect(page.locator('[data-qa-action="options.artifacts.load"]')).toBeEnabled()
  await page.locator('[data-qa-action="options.artifacts.load"]').click()
  await expect(page.getByText(/Loaded saved options expression artifact/i).first()).toBeVisible()
  await page.locator('[data-qa-action="shell.workspace.workflow"]').click()
  await page.locator('[data-qa-action="shell.mode.options"]').click()

  for (const step of ['data', 'optimize', 'validate', 'analyze', 'finalize']) {
    await page.locator(`[data-qa-action="shell.workflowStep.${step}"]`).click()
    await expect(page.locator(`[data-options-stage="${step}"]`)).toBeVisible()
    await expectAccessible(page, `Options ${step} stage`)
  }
  await guard.assertClean()
})

test('scans the compact shell and inspector drawer @workflow @a11y', async ({ page, context }) => {
  const guard = installQaGuards(page)
  await page.setViewportSize({ width: 390, height: 844 })
  await gotoMockedShellWithScenario(page, context, emptyAppScenario)
  await expectAccessible(page, 'compact workflow shell')
  await page.locator('[data-qa-action="workbench.inspector.compactOpen"]').click()
  await expect(page.locator('[aria-label="Workbench inspector"]')).toBeVisible()
  await expectAccessible(page, 'compact inspector drawer')
  await guard.assertClean()
})
