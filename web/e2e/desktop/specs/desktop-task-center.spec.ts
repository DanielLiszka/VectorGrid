import { $, expect } from '@wdio/globals'
import { taskCenterSelectors } from '../support/selectors.js'
import { closeTaskCenter, openTaskCenter, waitForShellReady } from '../support/waits.js'

describe('desktop task center', () => {
  it('opens, exposes the core sections, and closes from the real shell', async () => {
    await waitForShellReady()

    await openTaskCenter()
    const dialog = await $(taskCenterSelectors.dialog)
    await expect(dialog).toBeDisplayed()
    await expect($(taskCenterSelectors.activePostureSection)).toBeDisplayed()
    await expect($(taskCenterSelectors.recentOutcomesSection)).toBeDisplayed()
    await expect($(taskCenterSelectors.eventsSection)).toBeDisplayed()
    await expect($(taskCenterSelectors.timelineSection)).toBeDisplayed()

    await closeTaskCenter()
    await expect($(taskCenterSelectors.launcher)).toHaveAttr('aria-expanded', 'false')
  })
})
