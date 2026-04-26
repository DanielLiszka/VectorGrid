import { $, browser, expect } from '@wdio/globals'
import { shellSelectors } from '../support/selectors.js'
import { getNativeAppWindowRect } from '../support/nativeWindowGeometry.js'
import { openWorkspace, waitForSettingsWorkspace, waitForShellReady } from '../support/waits.js'

const TARGET_WIDTH = 1180
const TARGET_HEIGHT = 700
const SIZE_TOLERANCE = 24
const POSITION_TOLERANCE = 24

function closeTo(actual: number, expected: number, tolerance = SIZE_TOLERANCE): boolean {
  return Math.abs(actual - expected) <= tolerance
}

describe('desktop window geometry', () => {
  it('keeps the shell usable after a deterministic resize', async () => {
    await waitForShellReady()
    const originalRect = await browser.getWindowRect()
    const originalNativeRect = await getNativeAppWindowRect()

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
      await browser.setWindowRect(
        originalNativeRect.left,
        originalNativeRect.top,
        originalNativeRect.width,
        originalNativeRect.height,
      )
      await browser.waitUntil(
        async () => {
          const rect = await browser.getWindowRect()
          return closeTo(rect.width, originalRect.width) && closeTo(rect.height, originalRect.height)
        },
        {
          timeout: 10_000,
          timeoutMsg: 'Desktop window did not restore its original size.',
        },
      )
      await browser.waitUntil(
        async () => {
          const rect = await getNativeAppWindowRect()
          return (
            closeTo(rect.left, originalNativeRect.left, POSITION_TOLERANCE)
            && closeTo(rect.top, originalNativeRect.top, POSITION_TOLERANCE)
          )
        },
        {
          timeout: 10_000,
          timeoutMsg: 'Desktop window did not restore its original native position.',
        },
      )
    }
  })
})
