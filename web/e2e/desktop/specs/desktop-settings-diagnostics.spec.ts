import { $, expect } from '@wdio/globals'
import { describe, it } from 'mocha'
import { shellSelectors } from '../support/selectors.js'
import { openWorkspace, waitForSettingsWorkspace, waitForShellReady } from '../support/waits.js'

describe('desktop settings diagnostics', () => {
  it('shows desktop diagnostics and task-center controls in the desktop runtime', async () => {
    await waitForShellReady()

    await openWorkspace('settings')
    await waitForSettingsWorkspace()
    const settingsGrid = await $(shellSelectors.settingsGrid)
    await expect(settingsGrid).toBeDisplayed()

    const text = await settingsGrid.getText()
    expect(text).toContain('Desktop notifications')
    expect(text).toContain('Runtime and task center')
    expect(text).toContain('NATIVE DESKTOP')
    expect(text).toContain('DESKTOP LOGS')
    expect(text).toContain('Open task center')
    expect(text).toContain('Copy diagnostics snapshot')
    expect(text).toMatch(/Window (geometry persists|state persistence)/)
  })
})
