import { $, expect } from '@wdio/globals'
import { describe, it } from 'mocha'
import { shellSelectors, workspaceTabSelector } from '../support/selectors.js'
import { openWorkspace, waitForSettingsWorkspace, waitForShellReady } from '../support/waits.js'

describe('desktop workspace navigation', () => {
  it('moves between run library, aggregate analysis, and settings', async () => {
    await waitForShellReady()

    await openWorkspace('library')
    await expect($(workspaceTabSelector('library'))).toHaveAttr('aria-selected', 'true')
    const libraryText = await $('body').getText()
    expect(libraryText).toContain('Session history runs')

    await openWorkspace('aggregate')
    await expect($(workspaceTabSelector('aggregate'))).toHaveAttr('aria-selected', 'true')
    const aggregateText = await $('body').getText()
    expect(aggregateText).toContain('Meta analysis sections')

    await openWorkspace('settings')
    await expect($(workspaceTabSelector('settings'))).toHaveAttr('aria-selected', 'true')
    await waitForSettingsWorkspace()
    await expect($(shellSelectors.settingsGrid)).toBeDisplayed()
  })
})
