import { $, browser, expect } from '@wdio/globals'
import { shellSelectors } from '../support/selectors.js'
import { waitForShellReady } from '../support/waits.js'

describe('desktop titlebar native controls', () => {
  it('exposes enabled window controls and keeps the shell responsive after maximize interaction', async () => {
    await waitForShellReady()

    await expect($(shellSelectors.minimizeWindowButton)).toBeEnabled()
    await expect($(shellSelectors.maximizeWindowButton)).toBeEnabled()
    await expect($(shellSelectors.closeWindowButton)).toBeEnabled()

    const maximizeButton = await $(shellSelectors.maximizeWindowButton)
    await maximizeButton.click()
    await browser.pause(250)

    const restoreVisible = await $(shellSelectors.restoreWindowButton).isDisplayed().catch(() => false)
    if (restoreVisible) {
      const restoreButton = await $(shellSelectors.restoreWindowButton)
      await restoreButton.click()
      await browser.waitUntil(
        async () => (await $(shellSelectors.maximizeWindowButton)).isDisplayed().catch(() => false),
        {
          timeout: 5_000,
          timeoutMsg: 'Maximize button never reappeared after clicking restore.',
        },
      )
    } else {
      await expect($(shellSelectors.titlebar)).toBeDisplayed()
      await expect($(shellSelectors.workspaceBar)).toBeDisplayed()
    }
  })
})
