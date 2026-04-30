import { expect, test } from '@playwright/test'
import { shellSelectors } from '../desktop/support/selectors'
import { gotoMockedShell } from './support/appHarness'
import { dismissStartupOverlay, installMockedTauri } from './support/mockTauri'

test.describe('browser guided mode ux @workflow', () => {
  test('defaults first-run users into Simple Mode', async ({ context, page }) => {
    await installMockedTauri(context, { holdStartupOverlay: true })

    await page.goto('/', { waitUntil: 'networkidle' })
    await dismissStartupOverlay(page)

    await expect(page.locator(shellSelectors.titlebar)).toBeVisible()
    await expect(page.locator('#guidedShellGrid')).toBeVisible()
    await expect(page.locator('#uiModeBar [data-ui-mode="guided"]')).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#workspaceBar')).toHaveCount(0)
    await expect(page.locator('#runFooterShell')).toHaveCount(0)
    await expect(page.getByRole('heading', { name: 'Optimize backtests. Validate when needed.' })).toBeVisible()
    await expect(page.locator('.guidedNextAction').getByRole('button', { name: 'Choose data' })).toBeVisible()
  })

  test('keeps the first-run path focused while preserving the advanced handoff', async ({ context, page }) => {
    await gotoMockedShell(page, context)

    await page.locator('#uiModeBar [data-ui-mode="guided"]').click()

    await expect(page.locator('#guidedShellGrid')).toBeVisible()
    await expect(page.locator('#workspaceBar')).toHaveCount(0)
    await expect(page.locator('#runFooterShell')).toHaveCount(0)
    await expect(page.getByRole('heading', { name: 'Optimize backtests. Validate when needed.' })).toBeVisible()
    await expect(page.getByRole('navigation', { name: 'Simple workflow steps' }).getByRole('button')).toHaveCount(4)
    await expect(page.getByRole('region', { name: 'Simple checklist' }).getByRole('listitem')).toHaveCount(4)

    const titlebarPrimary = page.locator('.titlebarPrimary')
    await expect(titlebarPrimary).toHaveAttribute('aria-label', 'Choose data')
    await expect(titlebarPrimary).toBeDisabled()
    await expect(page.getByRole('button', { name: 'Open full data tools' })).toBeVisible()
    await expect(page.getByText('Need the full workstation?')).toBeVisible()

    await page.getByRole('navigation', { name: 'Simple workflow steps' }).getByRole('button', { name: /Optimize/i }).click()
    await expect(titlebarPrimary).toHaveAttribute('aria-label', 'Run optimization')
    await expect(titlebarPrimary).toBeDisabled()
    await expect(page.getByText('Load data before optimization.').first()).toBeVisible()

    await page.getByRole('navigation', { name: 'Simple workflow steps' }).getByRole('button', { name: /Validate/i }).click()
    await expect(titlebarPrimary).toHaveAttribute('aria-label', 'Run validation')
    await expect(titlebarPrimary).toBeDisabled()
    await expect(page.getByText('Run optimization before validation.').first()).toBeVisible()

    await page.locator(shellSelectors.settingsWorkspaceButton).click()

    await expect(page.locator('#uiModeBar [data-ui-mode="advanced"]')).toHaveAttribute('aria-selected', 'true')
    await expect(page.locator('#workspaceBar')).toBeVisible()
    await expect(page.locator('#settingsShellGrid')).toBeVisible()
    await expect(page.getByText('Default interface')).toBeVisible()
  })
})
