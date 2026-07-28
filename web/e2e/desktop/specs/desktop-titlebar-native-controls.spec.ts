import { $, browser, expect } from '@wdio/globals'
import { desktopSmokeDisablePreview } from '../support/appPaths.js'
import { shellSelectors } from '../support/selectors.js'
import {
  getNativeAppWindowGeometry,
  nativeRectWithinBounds,
  type NativeWindowRect,
} from '../support/nativeWindowGeometry.js'
import {
  isReleaseProofRun,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { waitForShellReady } from '../support/waits.js'

const GEOMETRY_TOLERANCE = 24
const MAXIMIZE_ROLLBACK_NOTICE = 'Maximize was reverted because the window exceeded the current monitor.'

function rectCloseTo(actual: NativeWindowRect, expected: NativeWindowRect): boolean {
  return Math.abs(actual.left - expected.left) <= GEOMETRY_TOLERANCE
    && Math.abs(actual.top - expected.top) <= GEOMETRY_TOLERANCE
    && Math.abs(actual.width - expected.width) <= GEOMETRY_TOLERANCE
    && Math.abs(actual.height - expected.height) <= GEOMETRY_TOLERANCE
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

async function observeNativeMaximizeTransition(): Promise<{
  finalWindow: NativeWindowRect
  sawUnsafeBounds: boolean
}> {
  const deadline = Date.now() + 1_200
  let sawUnsafeBounds = false
  let latest = await getNativeAppWindowGeometry()

  while (Date.now() < deadline) {
    latest = await getNativeAppWindowGeometry()
    const withinMonitor = nativeRectWithinBounds(latest.window, latest.monitor, GEOMETRY_TOLERANCE)
    sawUnsafeBounds ||= !withinMonitor
    await delay(25)
  }
  return { finalWindow: latest.window, sawUnsafeBounds }
}

describe('desktop titlebar native controls', () => {
  it('keeps maximize inside the active monitor or automatically restores safe bounds', async () => {
    await waitForShellReady()

    await expect($(shellSelectors.minimizeWindowButton)).toBeEnabled()
    await expect($(shellSelectors.closeWindowButton)).toBeEnabled()

    // The managed preview is a remote HTTP origin by design. Tauri correctly
    // withholds native window IPC from it, so only packaged-shell runs can
    // exercise maximize without weakening the application's capability boundary.
    if (!desktopSmokeDisablePreview) {
      await expect($(shellSelectors.maximizeWindowButton)).toBeEnabled()
      await expect($(shellSelectors.maximizeWindowButton)).toHaveAttribute('title', 'Maximize')
      return
    }

    const initiallyRestorable = await $(shellSelectors.restoreWindowButton).isDisplayed().catch(() => false)
    if (initiallyRestorable) {
      await (await $(shellSelectors.restoreWindowButton)).click()
      await (await $(shellSelectors.maximizeWindowButton)).waitForDisplayed({ timeout: 5_000 })
    }

    await expect($(shellSelectors.maximizeWindowButton)).toBeEnabled()
    const originalGeometry = await getNativeAppWindowGeometry()
    const maximizeButton = await $(shellSelectors.maximizeWindowButton)
    await maximizeButton.click()
    const maximizeOutcome = await observeNativeMaximizeTransition()
    const maximizedGeometry = await getNativeAppWindowGeometry()
    const maximizeTitle = await maximizeButton.getAttribute('title')
    expect(
      nativeRectWithinBounds(
        maximizedGeometry.window,
        maximizedGeometry.monitor,
        GEOMETRY_TOLERANCE,
      ),
    ).toBe(true)

    if (maximizeTitle === 'Restore') {
      const restoreButton = await $(shellSelectors.restoreWindowButton)
      await restoreButton.click()
      await browser.waitUntil(
        async () => (await $(shellSelectors.maximizeWindowButton)).isDisplayed().catch(() => false),
        {
          timeout: 5_000,
          timeoutMsg: 'Maximize button never reappeared after clicking restore.',
        },
      )
    } else if (maximizeTitle === MAXIMIZE_ROLLBACK_NOTICE) {
      await expect($(shellSelectors.maximizeWindowButton)).toHaveAttribute('title', MAXIMIZE_ROLLBACK_NOTICE)
      await browser.waitUntil(
        async () => rectCloseTo((await getNativeAppWindowGeometry()).window, originalGeometry.window),
        {
          timeout: 5_000,
          interval: 100,
          timeoutMsg: 'Guarded maximize did not restore the original native window bounds.',
        },
      )
      await expect($(shellSelectors.titlebar)).toBeDisplayed()
      await expect($(shellSelectors.workspaceBar)).toBeDisplayed()
    } else {
      throw new Error(
        `Unexpected maximize outcome '${maximizeTitle || '(empty)'}'; native transition: ${JSON.stringify(maximizeOutcome)}.`,
      )
    }

    if (isReleaseProofRun()) {
      const finalGeometry = await getNativeAppWindowGeometry()
      writeReleaseProofEvidence('titlebar-native-controls-proof.json', {
        contract: 'vectorgrid-titlebar-native-controls-v1',
        original_geometry: originalGeometry,
        maximized_geometry: maximizedGeometry,
        maximize_outcome: {
          title: maximizeTitle,
          saw_unsafe_bounds: maximizeOutcome.sawUnsafeBounds,
          transition_final_window: maximizeOutcome.finalWindow,
        },
        final_geometry: finalGeometry,
      })
    }
  })
})
