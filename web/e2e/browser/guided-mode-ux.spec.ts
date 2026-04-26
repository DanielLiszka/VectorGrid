import { expect, test } from '@playwright/test'
import { shellSelectors } from '../desktop/support/selectors'
import { gotoMockedShell } from './support/appHarness'
import { dismissStartupOverlay, installMockedTauri } from './support/mockTauri'

test.describe('browser guided mode ux', () => {
  test('defaults first-run users into Guided Mode', async ({ context, page }) => {
    await installMockedTauri(context, { holdStartupOverlay: true })

    await page.goto('/', { waitUntil: 'networkidle' })
    await dismissStartupOverlay(page)

    await expect(page.locator(shellSelectors.titlebar)).toBeVisible()
    await expect(page.locator('#guidedShellGrid')).toBeVisible()
    await expect(page.locator('#uiModeBar [data-ui-mode="guided"]')).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#workspaceBar')).toHaveCount(0)
    await expect(page.locator('#runFooterShell')).toHaveCount(0)
    await expect(page.getByRole('heading', { name: 'Optimize, validate, then dry-run.' })).toBeVisible()
    await expect(page.locator('.guidedNextAction').getByRole('button', { name: 'Choose data' })).toBeVisible()
  })

  test('keeps the first-run path focused while preserving the advanced handoff', async ({ context, page }) => {
    await gotoMockedShell(page, context)

    await page.locator('#uiModeBar [data-ui-mode="guided"]').click()

    await expect(page.locator('#guidedShellGrid')).toBeVisible()
    await expect(page.locator('#workspaceBar')).toHaveCount(0)
    await expect(page.locator('#runFooterShell')).toHaveCount(0)
    await expect(page.getByRole('heading', { name: 'Optimize, validate, then dry-run.' })).toBeVisible()
    await expect(page.getByRole('navigation', { name: 'Guided workflow steps' }).getByRole('button')).toHaveCount(5)
    await expect(page.getByRole('complementary', { name: 'Guided checklist' }).getByRole('listitem')).toHaveCount(5)

    const titlebarPrimary = page.locator('.titlebarPrimary')
    await expect(titlebarPrimary).toHaveAttribute('aria-label', 'Choose data')
    await expect(titlebarPrimary).toBeDisabled()
    await expect(page.getByRole('button', { name: 'Advanced Mode' })).toBeVisible()
    await expect(page.getByText('Advanced Mode stays complete.')).toBeVisible()

    await page.locator(shellSelectors.settingsWorkspaceButton).click()

    await expect(page.locator('#uiModeBar [data-ui-mode="advanced"]')).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#workspaceBar')).toBeVisible()
    await expect(page.locator('#settingsShellGrid')).toBeVisible()
    await expect(page.getByText('Default interface')).toBeVisible()
  })
})
