import { expect } from '@playwright/test'
import { emptyAppScenario } from '../fixtures'
import { defineWorkflow, type WorkflowContext } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow } from '../support/qaAssertions'

async function expectCompactWorkbenchParity({ page, action }: WorkflowContext): Promise<void> {
  await expectNoHorizontalDocumentOverflow(page)
  await expect(page.locator('[data-qa-shell="workbench"]')).toBeVisible()
  await expect(page.locator('[data-qa-panel="workbench.inspector"]')).toBeHidden()

  await action('shell.workflowStep.optimize').click()
  await expect(action('shell.workflowStep.optimize')).toHaveAttribute('aria-selected', 'true')
  await action('workbench.cockpit.step.configure').click()
  await expect(action('shell.workflowStep.optimize')).toHaveAttribute('aria-selected', 'true')
  await action('shell.workflowStep.validate').click()
  await expect(action('shell.workflowStep.validate')).toHaveAttribute('aria-selected', 'true')
  await action('shell.workspace.library').click()
  await expect(action('shell.workspace.library')).toHaveAttribute('aria-selected', 'true')

  await action('shell.commandPalette.open').click()
  await expect(page.locator('.cmdPaletteDialog')).toBeVisible()
  await page.getByLabel('Command search').fill('validate')
  await expect(action('commandPalette.command.goStepValidate')).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(page.locator('.cmdPaletteDialog')).toBeHidden()

  await action('shell.workspace.workflow').click()
  await expect(page.locator('#workflowStepBar')).toBeVisible()
  await expectNoHorizontalDocumentOverflow(page)
}

defineWorkflow({
  id: 'responsive.mobile.commandPalette',
  title: 'keeps command palette usable at compact width',
  tags: ['@workflow', '@responsive'],
  viewport: { width: 390, height: 844 },
  scenario: emptyAppScenario,
  run: async ({ page, action }) => {
    await action('shell.commandPalette.open').click()
    const dialog = page.locator('.cmdPaletteDialog')
    await expect(dialog).toBeVisible()
    await page.getByLabel('Command search').fill('settings')
    await expect(page.getByText('Go to Settings')).toBeVisible()
    await expectNoHorizontalDocumentOverflow(page)
  },
})

defineWorkflow({
  id: 'responsive.mobile.workbenchParity',
  title: 'keeps compact workbench shell core workflow parity',
  tags: ['@workflow', '@responsive'],
  viewport: { width: 390, height: 844 },
  scenario: emptyAppScenario,
  run: expectCompactWorkbenchParity,
})
