import { expect, test } from '@playwright/test'
import { commandPaletteSelectors, shellSelectors, taskCenterSelectors, workspaceTabSelector } from '../desktop/support/selectors'
import { optimizationSuccessScenario } from './fixtures'
import {
  closeCommandPaletteWithEscape,
  closeTaskCenterWithEscape,
  gotoMockedShell,
  openCommandPalette,
} from './support/appHarness'
import {
  gotoMockedShellWithScenario,
  readTauriInvokeLog,
} from './support/workflowRunner'

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

    const taskCenterInput = await openCommandPalette(page)
    await taskCenterInput.fill('task center')
    await expect(page.locator(commandPaletteSelectors.listbox)).toContainText(
      'Open Task Center',
    )
    await taskCenterInput.press('Enter')
    await expect(page.locator(commandPaletteSelectors.dialog)).toBeHidden()
    await expect(page.locator(taskCenterSelectors.dialog)).toContainText('Active posture')
    await expect(page.locator(taskCenterSelectors.dialog)).toContainText('Recent outcomes')
    await closeTaskCenterWithEscape(page)
    await expect(searchButton).toBeFocused()
  })

  test('supports palette close via Escape with focus restored to the opener', async ({ context, page }) => {
    await gotoMockedShell(page, context)

    const searchButton = page.locator(shellSelectors.commandPaletteButton)
    await openCommandPalette(page)
    await page.locator(commandPaletteSelectors.searchInput).fill('aggregate')
    await closeCommandPaletteWithEscape(page)
    await expect(searchButton).toBeFocused()
  })

  test('runs a ready optimization successfully from both alternate action surfaces', async ({
    context,
    page,
  }) => {
    await gotoMockedShellWithScenario(
      page,
      context,
      optimizationSuccessScenario,
    )

    await page.locator('[data-qa-action="shell.workflowStep.data"]').click()
    await page.locator('[data-qa-action="data.sample.load"]').first().click()
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === 'load_price_data',
        ).length,
      )
      .toBe(1)

    await page.locator('[data-qa-action="shell.workflowStep.optimize"]').click()
    const footerPrimary = page.locator(
      '[data-qa-action="footer.primaryAction.run"]',
    )
    await expect(footerPrimary).toBeEnabled()

    const input = await openCommandPalette(page)
    await input.fill('run optimization')
    const paletteRun = page.locator(
      '[data-qa-action="commandPalette.command.run"]',
    )
    await expect(paletteRun).toBeVisible()
    await expect(paletteRun).toHaveAttribute('aria-disabled', 'false')
    await input.press('Enter')

    await expect(page.locator(commandPaletteSelectors.dialog)).toBeHidden()
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === 'run_double_ma_optimization',
        ).length,
      )
      .toBe(1)
    await expect(footerPrimary).toHaveText('Run optimization')
    await expect(footerPrimary).toBeEnabled()

    await footerPrimary.click()
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === 'run_double_ma_optimization',
        ).length,
      )
      .toBe(2)
    await expect(footerPrimary).toHaveText('Run optimization')
    await expect(footerPrimary).toBeEnabled()
  })
})
