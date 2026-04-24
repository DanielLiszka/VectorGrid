import { $, expect } from '@wdio/globals'
import { describe, it } from 'mocha'
import { shellSelectors } from '../support/selectors.js'
import { waitForShellReady } from '../support/waits.js'

describe('desktop shell readiness', () => {
  it('launches the Tauri shell and exposes the main chrome', async () => {
    await waitForShellReady()

    await expect($(shellSelectors.titlebar)).toBeDisplayed()
    await expect($(shellSelectors.shellHeader)).toBeDisplayed()
    await expect($(shellSelectors.workspaceBar)).toBeDisplayed()
    await expect($(shellSelectors.runFooter)).toBeDisplayed()

    const workspaceText = await (await $(shellSelectors.workspaceBar)).getText()
    expect(workspaceText).toContain('Research')
    expect(workspaceText).toContain('Library')
    expect(workspaceText).toContain('Aggregate')
    expect(workspaceText).toContain('Diagnostics')
    expect(workspaceText).toContain('Settings')
  })
})
