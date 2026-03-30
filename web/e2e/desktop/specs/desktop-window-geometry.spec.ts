import { $, browser, expect } from '@wdio/globals'
import { describe, it } from 'mocha'
import { shellSelectors } from '../support/selectors.js'
import { openWorkspace, waitForSettingsWorkspace, waitForShellReady } from '../support/waits.js'

const TARGET_WIDTH = 1180
const TARGET_HEIGHT = 700
const SIZE_TOLERANCE = 24

function closeTo(actual: number, expected: number): boolean {
  return Math.abs(actual - expected) <= SIZE_TOLERANCE
}

describe('desktop window geometry', () => {
  it('keeps the shell usable after a deterministic resize', async () => {
    await waitForShellReady()
    const originalRect = await browser.getWindowRect()

    try {
      await browser.setWindowSize(TARGET_WIDTH, TARGET_HEIGHT)
      await browser.waitUntil(
        async () => {
          const rect = await browser.getWindowRect()
          return closeTo(rect.width, TARGET_WIDTH) && closeTo(rect.height, TARGET_HEIGHT)
        },
        {
          timeout: 10_000,
          timeoutMsg: `Desktop window never resized to approximately ${String(TARGET_WIDTH)}x${String(TARGET_HEIGHT)}.`,
        },
      )

      await expect($(shellSelectors.titlebar)).toBeDisplayed()
      await expect($(shellSelectors.workspaceBar)).toBeDisplayed()
      await expect($(shellSelectors.runFooter)).toBeDisplayed()

      await openWorkspace('settings')
      await waitForSettingsWorkspace()
    } finally {
      await browser.setWindowRect(originalRect.x, originalRect.y, originalRect.width, originalRect.height)
      await browser.waitUntil(
        async () => {
          const rect = await browser.getWindowRect()
          return closeTo(rect.width, originalRect.width) && closeTo(rect.height, originalRect.height)
        },
        {
          timeout: 10_000,
          timeoutMsg: 'Desktop window did not restore its original geometry.',
        },
      )
    }
  })
})
