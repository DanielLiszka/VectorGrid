import { expect } from '@playwright/test'
import { emptyAppScenario } from '../fixtures/scenarios'
import { defineWorkflow } from '../support/workflowRunner'
import { expectNoHorizontalDocumentOverflow } from '../support/qaAssertions'

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
