import { expect, test, type BrowserContext, type Page } from '@playwright/test'
import { gotoMockedShell } from './support/appHarness'

async function openDataStepWithHelpMode(page: Page, context: BrowserContext, helpMode: 'full' | 'concise' | 'off'): Promise<void> {
  await context.addInitScript((mode) => {
    try {
      localStorage.setItem('vta_help_mode_v1', mode)
      localStorage.setItem('vta_ui_mode_v1', 'advanced')
      localStorage.setItem('vta_workflow_step_v1', 'data')
    } catch {
      // Ignore storage failures in the browser harness.
    }
  }, helpMode)

  await gotoMockedShell(page, context)
  await page.locator('#workflowStepBar [data-workflow-step="data"]').click()
}

test.describe('browser help tooltip behavior', () => {
  test('opens full help on hover and closes it after pointer leave', async ({ page, context }) => {
    await openDataStepWithHelpMode(page, context, 'full')

    const trigger = page.getByRole('button', { name: 'Storage controls help' })
    await expect(trigger).toBeVisible()
    await trigger.scrollIntoViewIfNeeded()
    await trigger.hover()

    const card = page.locator('.contextHelpCard').filter({ hasText: 'Review the active market-data folder' })
    await page.waitForTimeout(900)
    await expect(card).toBeHidden()

    await expect(card).toBeVisible({ timeout: 1_600 })
    await expect(card).toContainText('Storage controls')
    await expect(card).toContainText('Directory-level storage management')

    await page.mouse.move(8, 8)
    await expect(card).toBeHidden({ timeout: 1_000 })
  })

  test('opens concise help on hover without rendering the long detail body', async ({ page, context }) => {
    await openDataStepWithHelpMode(page, context, 'concise')

    const trigger = page.getByRole('button', { name: 'Storage controls help' })
    await expect(trigger).toBeVisible()
    await trigger.scrollIntoViewIfNeeded()
    await trigger.hover()

    const tooltip = page.locator('.contextHelpTooltip').filter({ hasText: 'Review the active market-data folder' })
    await page.waitForTimeout(400)
    await expect(tooltip).toBeHidden()

    await expect(tooltip).toBeVisible({ timeout: 1_000 })
    await expect(tooltip).toContainText('Review the active market-data folder')
    await expect(tooltip).not.toContainText('Directory-level storage management')

    await page.mouse.move(8, 8)
    await expect(tooltip).toBeHidden({ timeout: 1_000 })
  })

  test('hides optional help triggers when help mode is off', async ({ page, context }) => {
    await openDataStepWithHelpMode(page, context, 'off')

    await expect(page.getByRole('button', { name: 'Storage controls help' })).toHaveCount(0)
  })
})
