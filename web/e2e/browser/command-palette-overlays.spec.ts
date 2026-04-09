import { expect, test } from '@playwright/test'
import { commandPaletteSelectors, shellSelectors, taskCenterSelectors, workspaceTabSelector } from '../desktop/support/selectors'
import {
  closeCommandPaletteWithEscape,
  closeTaskCenterWithEscape,
  gotoMockedShell,
  openCommandPalette,
  openTaskCenter,
} from './support/appHarness'

test.describe('browser command palette and overlays', () => {
  test('supports command palette execution plus task-center focus return', async ({ context, page }) => {
    await gotoMockedShell(page, context)

    const searchButton = page.locator(shellSelectors.commandPaletteButton)
    const input = await openCommandPalette(page)

    await input.fill('go to')
    await expect(page.locator(commandPaletteSelectors.listbox)).toContainText('Go to Settings')
    const initialActiveDescendant = await input.getAttribute('aria-activedescendant')
    await input.press('ArrowDown')
    await expect.poll(async () => input.getAttribute('aria-activedescendant')).not.toBe(initialActiveDescendant)
    await input.press('ArrowUp')
    await expect(input).toHaveAttribute('aria-activedescendant', initialActiveDescendant || '')

    await input.fill('settings')
    await expect(page.locator(commandPaletteSelectors.listbox)).toContainText('Go to Settings')
    await input.press('Enter')

    await expect(page.locator(commandPaletteSelectors.dialog)).toBeHidden()
    await expect(page.locator(workspaceTabSelector('settings'))).toHaveAttribute('aria-selected', 'true')
    await expect(searchButton).toBeFocused()

    const launcher = await openTaskCenter(page)
    await expect(page.locator(taskCenterSelectors.dialog)).toContainText('Active posture')
    await expect(page.locator(taskCenterSelectors.dialog)).toContainText('Recent outcomes')
    await closeTaskCenterWithEscape(page)
    await expect(launcher).toBeFocused()
    await expect(launcher).toHaveAttribute('aria-expanded', 'false')
  })

  test('supports palette close via Escape with focus restored to the opener', async ({ context, page }) => {
    await gotoMockedShell(page, context)

    const searchButton = page.locator(shellSelectors.commandPaletteButton)
    await openCommandPalette(page)
    await page.locator(commandPaletteSelectors.searchInput).fill('aggregate')
    await closeCommandPaletteWithEscape(page)
    await expect(searchButton).toBeFocused()
  })
})
