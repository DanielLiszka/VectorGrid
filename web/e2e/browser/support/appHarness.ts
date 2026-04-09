import { expect, type BrowserContext, type Locator, type Page } from '@playwright/test'
import { commandPaletteSelectors, shellSelectors, taskCenterSelectors, workspaceTabSelector } from '../../desktop/support/selectors'
import { dismissStartupOverlay, installMockedTauri } from './mockTauri'

export async function gotoMockedShell(page: Page, context: BrowserContext): Promise<void> {
  await installMockedTauri(context, { holdStartupOverlay: true })
  await page.goto('/', { waitUntil: 'networkidle' })
  await dismissStartupOverlay(page)
  await waitForShellReady(page)
}

export async function waitForShellReady(page: Page): Promise<void> {
  await expect(page.locator(shellSelectors.titlebar)).toBeVisible()
  await expect(page.locator(shellSelectors.shellHeader)).toBeVisible()
  await expect(page.locator(shellSelectors.workspaceBar)).toBeVisible()
  await expect(page.locator(shellSelectors.runFooter)).toBeVisible()
  await expect(page.locator(workspaceTabSelector('workflow'))).toHaveAttribute('aria-selected', 'true')
}

export async function openCommandPalette(page: Page): Promise<Locator> {
  const opener = page.locator(shellSelectors.commandPaletteButton)
  await opener.click()
  const dialog = page.locator(commandPaletteSelectors.dialog)
  await expect(dialog).toBeVisible()
  const input = page.locator(commandPaletteSelectors.searchInput)
  await expect(input).toBeVisible()
  await expect(input).toBeFocused()
  return input
}

export async function closeCommandPaletteWithEscape(page: Page): Promise<void> {
  await page.keyboard.press('Escape')
  await expect(page.locator(commandPaletteSelectors.dialog)).toBeHidden()
}

export async function openTaskCenter(page: Page): Promise<Locator> {
  const launcher = page.locator('button.taskCenterLauncher')
  await launcher.click()
  const dialog = page.locator(taskCenterSelectors.dialog)
  await expect(dialog).toBeVisible()
  return launcher
}

export async function closeTaskCenterWithEscape(page: Page): Promise<void> {
  await page.keyboard.press('Escape')
  await expect(page.locator(taskCenterSelectors.dialog)).toBeHidden()
}
