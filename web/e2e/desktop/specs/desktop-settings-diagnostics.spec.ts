import { $, expect } from '@wdio/globals'
import { shellSelectors } from '../support/selectors.js'
import { openWorkspace, waitForSettingsWorkspace, waitForShellReady } from '../support/waits.js'

describe('desktop settings diagnostics', () => {
  it('shows desktop diagnostics and task-center controls in the desktop runtime', async () => {
    await waitForShellReady()

    await openWorkspace('settings')
    await waitForSettingsWorkspace()
    const settingsGrid = await $(shellSelectors.settingsGrid)
    await expect(settingsGrid).toBeDisplayed()
    await expect($(shellSelectors.settingsDiagnosticsSection)).toBeDisplayed()
    await expect($(shellSelectors.settingsDiagnosticsRuntimeCard)).toBeDisplayed()
    await expect($(shellSelectors.settingsDiagnosticsNativeDesktopCard)).toBeDisplayed()
    await expect($(shellSelectors.settingsDiagnosticsDesktopPresenceCard)).toBeDisplayed()
    await expect($(shellSelectors.settingsDiagnosticsTaskPostureCard)).toBeDisplayed()
    await expect($(shellSelectors.settingsDiagnosticsDesktopLogsCard)).toBeDisplayed()
    await expect($(shellSelectors.settingsOpenTaskCenterButton)).toBeDisplayed()
    await expect($(shellSelectors.settingsCopyDiagnosticsButton)).toBeDisplayed()
    await expect($(shellSelectors.settingsDesktopNotificationPermission)).toBeDisplayed()
    await expect($(shellSelectors.settingsDesktopUpdaterStatus)).toBeDisplayed()

    const nativeDesktopDiagnosticsText = await (await $(
      shellSelectors.settingsDiagnosticsNativeDesktopCard,
    )).getText()
    expect(nativeDesktopDiagnosticsText).toContain('Updates: Updater idle')
    expect(nativeDesktopDiagnosticsText).toContain(
      'Update checks are installed but disabled until signed release metadata is configured.',
    )

    const windowStateText = await (await $(shellSelectors.settingsDesktopWindowStateSummary)).getText()
    expect(windowStateText).toMatch(/Window (geometry persists|state persistence)/)

    const presenceSummary = await (await $(shellSelectors.settingsDesktopPresenceSummary)).getText()
    expect(presenceSummary).toContain('Foreground-first workstation')
  })
})
