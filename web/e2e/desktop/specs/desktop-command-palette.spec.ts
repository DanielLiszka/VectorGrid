import { $, browser, expect } from '@wdio/globals'
import { describe, it } from 'mocha'
import { commandPaletteSelectors, shellSelectors, workspaceTabSelector } from '../support/selectors.js'
import { closeCommandPalette, openCommandPalette, waitForShellReady, waitForSettingsWorkspace } from '../support/waits.js'

describe('desktop command palette', () => {
  it('opens from the titlebar, navigates to settings, and closes cleanly', async () => {
    await waitForShellReady()

    await openCommandPalette()
    await expect($(commandPaletteSelectors.dialog)).toBeDisplayed()
    await expect($(commandPaletteSelectors.searchInput)).toBeFocused()
    await expect($(commandPaletteSelectors.listbox)).toBeDisplayed()

    const input = await $(commandPaletteSelectors.searchInput)
    await input.setValue('settings')
    await browser.waitUntil(
      async () => (await (await $(commandPaletteSelectors.listbox)).getText()).includes('Go to Settings'),
      {
        timeout: 10000,
        timeoutMsg: 'Command palette never surfaced Go to Settings.',
      },
    )

    await browser.keys('Enter')
    await browser.waitUntil(
      async () => !(await (await $(commandPaletteSelectors.dialog)).isDisplayed().catch(() => false)),
      {
        timeout: 10000,
        timeoutMsg: 'Command palette did not close after command execution.',
      },
    )
    await expect($(workspaceTabSelector('settings'))).toHaveAttr('aria-selected', 'true')
    await waitForSettingsWorkspace()

    await (await $(shellSelectors.commandPaletteButton)).click()
    await expect($(commandPaletteSelectors.dialog)).toBeDisplayed()
    await closeCommandPalette()
  })
})
