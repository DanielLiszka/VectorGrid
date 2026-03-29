import { $, expect } from '@wdio/globals'
import { describe, it } from 'mocha'
import { shellSelectors } from '../support/selectors.js'
import { waitForShellReady } from '../support/waits.js'

describe('desktop titlebar native controls', () => {
  it('exposes enabled window controls and can maximize then restore', async () => {
    await waitForShellReady()

    await expect($(shellSelectors.minimizeWindowButton)).toBeEnabled()
    await expect($(shellSelectors.maximizeWindowButton)).toBeEnabled()
    await expect($(shellSelectors.closeWindowButton)).toBeEnabled()

    const maximizeButton = await $(shellSelectors.maximizeWindowButton)
    await maximizeButton.click()
    await expect($(shellSelectors.restoreWindowButton)).toBeDisplayed()

    const restoreButton = await $(shellSelectors.restoreWindowButton)
    await restoreButton.click()
    await expect($(shellSelectors.maximizeWindowButton)).toBeDisplayed()
  })
})
