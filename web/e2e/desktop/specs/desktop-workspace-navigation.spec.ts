import { $, expect } from '@wdio/globals'
import { shellSelectors, workspaceTabSelector } from '../support/selectors.js'
import { openWorkspace, waitForSettingsWorkspace, waitForShellReady } from '../support/waits.js'

describe('desktop workspace navigation', () => {
  it('moves between workflow-critical workspaces in the real shell', async () => {
    await waitForShellReady()

    await openWorkspace('workflow')
    await expect($(workspaceTabSelector('workflow'))).toHaveAttr('aria-selected', 'true')
    await expect($(shellSelectors.workflowStepBar)).toBeDisplayed()

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

    await openWorkspace('live')
    await expect($(workspaceTabSelector('live'))).toHaveAttr('aria-selected', 'true')
    await expect($(shellSelectors.liveGrid)).toBeDisplayed()

    await openWorkspace('settings')
    await expect($(workspaceTabSelector('settings'))).toHaveAttr('aria-selected', 'true')
    await waitForSettingsWorkspace()
    await expect($(shellSelectors.settingsGrid)).toBeDisplayed()
  })
})
