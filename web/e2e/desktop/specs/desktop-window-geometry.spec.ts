import { $, browser, expect } from '@wdio/globals'
import { shellSelectors } from '../support/selectors.js'
import { getNativeAppWindowRect } from '../support/nativeWindowGeometry.js'
import {
  isReleaseProofRun,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
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
    let resizedRect = originalRect
    let resizedNativeRect = originalNativeRect

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
      resizedRect = await browser.getWindowRect()
      resizedNativeRect = await getNativeAppWindowRect()
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

    if (isReleaseProofRun()) {
      writeReleaseProofEvidence('window-geometry-proof.json', {
        contract: 'vectorgrid-window-geometry-v1',
        target: {
          width: TARGET_WIDTH,
          height: TARGET_HEIGHT,
        },
        original_webdriver_rect: originalRect,
        original_native_rect: originalNativeRect,
        resized_webdriver_rect: resizedRect,
        resized_native_rect: resizedNativeRect,
        restored_webdriver_rect: await browser.getWindowRect(),
        restored_native_rect: await getNativeAppWindowRect(),
      })
    }
  })
})
