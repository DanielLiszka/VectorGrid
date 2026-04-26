import { expect, test } from '@playwright/test'
import { gotoMockedShell } from './support/appHarness'

test.describe('artifact history bootstrap perf bridge', () => {
  test('exposes startup and latest-run timing entries through the browser debug bridge', async ({ context, page }) => {
    await gotoMockedShell(page, context)

    const entries = await page.evaluate(() => {
      const bridge = (
        window as Window & {
          __VTA_ARTIFACT_HISTORY_PERF__?: {
            readEntries: () => Array<{ name: string; durationMs: number; atUnixMs: number }>
          }
        }
      ).__VTA_ARTIFACT_HISTORY_PERF__
      return bridge ? bridge.readEntries() : []
    })

    const names = entries.map((entry) => entry.name)
    expect(names).toEqual(
      expect.arrayContaining([
        'main.firstPaintWaitMs',
        'main.bootToReadyMs',
        'workflowBootstrap.startupDataHydrationMs',
        'workflowBootstrap.latestRunSyncMs',
        'workflowArtifactsSync.latestRunFastPathMs',
      ]),
    )
    for (const entry of entries) {
      expect(entry.durationMs).toBeGreaterThanOrEqual(0)
      expect(entry.atUnixMs).toBeGreaterThan(0)
    }
  })
})
