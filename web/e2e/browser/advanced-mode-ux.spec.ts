import { expect, test } from '@playwright/test'
import { shellSelectors } from '../desktop/support/selectors'
import { dismissStartupOverlay, installMockedTauri } from './support/mockTauri'

test.describe('browser advanced-only mode ux @workflow', () => {
  test('defaults first-run users into the full Advanced workbench', async ({ context, page }) => {
    await installMockedTauri(context, { holdStartupOverlay: true })

    await page.goto('/', { waitUntil: 'networkidle' })
    await dismissStartupOverlay(page)

    await expect(page.locator(shellSelectors.titlebar)).toBeVisible()
    await expect(page.locator('#uiModeBar')).toHaveCount(0)
    await expect(page.locator('#guidedShellGrid')).toHaveCount(0)
    await expect(page.locator('#workspaceBar')).toBeVisible()
    await expect(page.locator('#runFooterShell')).toBeVisible()
    await expect(page.locator('#dataVueShellMount')).toBeVisible()
  })

  test('ignores a legacy persisted Simple preference during launch', async ({ context, page }) => {
    await context.addInitScript(() => {
      localStorage.setItem('vta_ui_mode_v1', 'guided')
    })
    await installMockedTauri(context, { holdStartupOverlay: true })

    await page.goto('/', { waitUntil: 'networkidle' })
    await dismissStartupOverlay(page)

    await expect(page.locator('#uiModeBar')).toHaveCount(0)
    await expect(page.locator('#guidedShellGrid')).toHaveCount(0)
    await expect(page.locator('#workspaceBar')).toBeVisible()
    await expect(page.locator('#runFooterShell')).toBeVisible()
    await expect(page.locator(shellSelectors.settingsWorkspaceButton)).toBeVisible()
  })
})
