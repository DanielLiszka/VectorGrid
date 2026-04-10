import { $, expect } from '@wdio/globals'
import { describe, it } from 'mocha'
import { shellSelectors, workspaceTabSelector } from '../support/selectors.js'
import { openWorkspace, waitForSettingsWorkspace, waitForShellReady } from '../support/waits.js'

describe('desktop workspace navigation', () => {
  it('moves between run library, aggregate analysis, pro tools, and settings', async () => {
    await waitForShellReady()

    await openWorkspace('library')
    await expect($(workspaceTabSelector('library'))).toHaveAttr('aria-selected', 'true')
    await expect($(shellSelectors.libraryPanelTitle)).toBeDisplayed()
    await expect($(shellSelectors.libraryPanelTitle)).toHaveText('Run History')

    await openWorkspace('aggregate')
    await expect($(workspaceTabSelector('aggregate'))).toHaveAttr('aria-selected', 'true')
    await expect($(shellSelectors.aggregatePanelTitle)).toBeDisplayed()
    await expect($(shellSelectors.aggregatePanelTitle)).toHaveText('Meta Analysis')

    await openWorkspace('pro')
    await expect($(workspaceTabSelector('pro'))).toHaveAttr('aria-selected', 'true')

    await openWorkspace('settings')
    await expect($(workspaceTabSelector('settings'))).toHaveAttr('aria-selected', 'true')
    await waitForSettingsWorkspace()
    await expect($(shellSelectors.settingsGrid)).toBeDisplayed()
  })
})
