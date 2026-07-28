import { browser, expect } from '@wdio/globals'
import {
  getNativeAppWindowGeometry,
  nativeRectWithinBounds,
} from '../support/nativeWindowGeometry.js'
import {
  isReleaseProofRun,
  writeReleaseProofEvidence,
} from '../support/releaseProofEvidence.js'
import { waitForShellReady } from '../support/waits.js'

const GEOMETRY_TOLERANCE = 24

describe('desktop persisted window bounds', () => {
  it('relaunches inside one monitor after the preceding native window checks', async () => {
    const coldRelaunched = isReleaseProofRun()
    if (coldRelaunched) await browser.reloadSession()
    await waitForShellReady()

    const geometry = await getNativeAppWindowGeometry()
    expect(nativeRectWithinBounds(geometry.window, geometry.monitor, GEOMETRY_TOLERANCE)).toBe(true)
    expect(geometry.window.width).toBeLessThanOrEqual(
      geometry.monitor.right - geometry.monitor.left + GEOMETRY_TOLERANCE,
    )
    expect(geometry.window.height).toBeLessThanOrEqual(
      geometry.monitor.bottom - geometry.monitor.top + GEOMETRY_TOLERANCE,
    )

    if (coldRelaunched) {
      writeReleaseProofEvidence('window-launch-bounds-proof.json', {
        contract: 'vectorgrid-window-launch-bounds-v1',
        cold_relaunched: true,
        geometry,
        tolerance_px: GEOMETRY_TOLERANCE,
      })
    }
  })
})
