import { $, expect } from '@wdio/globals'
import { shellSelectors } from '../support/selectors.js'
import { expectedWorkspaceLabels, waitForShellReady } from '../support/waits.js'

describe('desktop shell readiness', () => {
  it('launches the Tauri shell and exposes the main chrome', async () => {
    await waitForShellReady()

    await expect($(shellSelectors.titlebar)).toBeDisplayed()
    await expect($(shellSelectors.shellHeader)).toBeDisplayed()
    await expect($(shellSelectors.workspaceBar)).toBeDisplayed()
    await expect($(shellSelectors.runFooter)).toBeDisplayed()

    const workspaceText = await (await $(shellSelectors.workspaceBar)).getText()
    for (const label of await expectedWorkspaceLabels()) {
      expect(workspaceText).toContain(label)
    }
  })
})
