import { $, expect } from '@wdio/globals'
import { describe, it } from 'mocha'
import { taskCenterSelectors } from '../support/selectors.js'
import { closeTaskCenter, openTaskCenter, waitForShellReady } from '../support/waits.js'

describe('desktop task center', () => {
  it('opens, exposes the core sections, and closes from the real shell', async () => {
    await waitForShellReady()

    await openTaskCenter()
    const dialog = await $(taskCenterSelectors.dialog)
    await expect(dialog).toBeDisplayed()
    const text = await dialog.getText()
    expect(text).toContain('ACTIVE POSTURE')
    expect(text).toContain('RECENT OUTCOMES')
    expect(text).toContain('EVENTS')
    expect(text).toContain('TIMELINE')

    await closeTaskCenter()
    await expect($(taskCenterSelectors.launcher)).toHaveAttr('aria-expanded', 'false')
  })
})
