import { $, browser, expect } from '@wdio/globals'
import { shellSelectors } from '../support/selectors.js'
import { waitForShellReady } from '../support/waits.js'

function isSessionClosedError(error: unknown): boolean {
  const message = String(error instanceof Error ? error.message : error || '').toLowerCase()
  return message.includes('invalid session id')
    || message.includes('no such window')
    || message.includes('webview not found')
    || message.includes('target window already closed')
  }

describe('desktop app close', () => {
  it('closes the real desktop session from the custom titlebar close button', async () => {
    await waitForShellReady()

    const closeButton = await $(shellSelectors.closeWindowButton)
    await expect(closeButton).toBeEnabled()
    await closeButton.click()

    await browser.waitUntil(
      async () => {
        try {
          const handles = await browser.getWindowHandles()
          return handles.length === 0
        } catch (error) {
          return isSessionClosedError(error)
        }
      },
      {
        timeout: 10_000,
        timeoutMsg: 'Desktop app did not close after clicking the custom titlebar close button.',
      },
    )
  })
})
