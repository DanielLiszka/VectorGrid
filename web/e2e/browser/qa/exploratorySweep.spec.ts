import { expect } from '@playwright/test'
import { emptyAppScenario } from '../fixtures/scenarios'
import { defineWorkflow } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow, expectNoStuckBusyState } from '../support/qaAssertions'

defineWorkflow({
  id: 'shell.commandPalette.navigation',
  title: 'exercises safe navigation and overlay actions through the real browser',
  tags: ['@explore'],
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.taskCenter.open').click()
    await expect(page.locator('[aria-label="Task center"]')).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(page.locator('[aria-label="Task center"]')).toBeHidden()

    await action('shell.commandPalette.open').click()
    await expect(page.locator('.cmdPaletteDialog')).toBeVisible()
    await page.getByLabel('Command search').fill('settings')
    await page.keyboard.press('Enter')
    await expect(page.locator('#settingsShellGrid')).toBeVisible()

    await action('shell.commandPalette.open').click()
    await page.getByLabel('Command search').fill('live')
    await page.keyboard.press('Enter')
    await expect(page.locator('#liveShellGrid')).toBeVisible()

    await action('shell.commandPalette.open').click()
    await page.getByLabel('Command search').fill('portfolio')
    await page.keyboard.press('Enter')
    await expect(page.locator('#portfolioShellGrid')).toBeVisible()

    await action('shell.workspace.workflow').click()
    await expect(page.locator('#workflowStepBar')).toBeVisible()

    for (const step of ['data', 'optimize', 'validate', 'analyze', 'finalize']) {
      await action(`shell.workflowStep.${step}`).click()
      await expect(action(`shell.workflowStep.${step}`)).toHaveAttribute('aria-selected', 'true')
      await expectNoStuckBusyState(page)
    }

    await action('shell.workflowStep.analyze').click()
    for (const tab of ['overview', 'top', 'drill', 'heatmap', 'pareto']) {
      await action(`shell.resultsTab.${tab}`).click()
      await expect(action(`shell.resultsTab.${tab}`)).toHaveAttribute('aria-selected', 'true')
    }

    for (const workspace of ['library', 'aggregate', 'portfolio', 'diagnostics', 'live', 'settings', 'workflow']) {
      await action(`shell.workspace.${workspace}`).click()
      await expect(action(`shell.workspace.${workspace}`)).toHaveAttribute('aria-selected', 'true')
      await expectNoStuckBusyState(page)
    }

    await action('shell.uiMode.guided').click()
    await expect(page.locator('#guidedShellGrid')).toBeVisible()
    for (const stepButton of await action('guided.step.select').all()) {
      await stepButton.click()
      await expectNoStuckBusyState(page)
    }
    await action('shell.uiMode.advanced').click()
    await expect(action('shell.uiMode.advanced')).toHaveAttribute('aria-selected', 'true')

    await expectNoHorizontalDocumentOverflow(page)
  },
})
