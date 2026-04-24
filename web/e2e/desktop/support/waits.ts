import { $, browser } from '@wdio/globals'
import { desktopSmokeAppUrl, desktopSmokeDisablePreview, desktopSmokePackagedUrl } from './appPaths.js'
import { commandPaletteSelectors, shellSelectors, taskCenterSelectors, type WorkspaceKey, workspaceTabSelector } from './selectors.js'

async function waitForShellMarkerOrUrl(urlPrefix: string, timeout: number, timeoutMsg: string): Promise<void> {
  await browser.waitUntil(
    async () => {
      const titlebarVisible = await (await $(shellSelectors.titlebar))
        .isDisplayed()
        .catch(() => false)
      if (titlebarVisible) {
        return true
      }
      const nextUrl = await browser.getUrl().catch(() => '')
      return Boolean(nextUrl && nextUrl !== 'about:blank' && nextUrl.startsWith(urlPrefix))
    },
    {
      timeout,
      interval: 250,
      timeoutMsg,
    },
  )
}

export async function waitForShellReady(timeout = 30000): Promise<void> {
  const currentUrl = await browser.getUrl().catch(() => '')
  if (!currentUrl || currentUrl === 'about:blank') {
    if (desktopSmokeDisablePreview) {
      try {
        await waitForShellMarkerOrUrl(
          desktopSmokePackagedUrl,
          Math.min(timeout, 4_000),
          'Desktop smoke stayed on the blank WebDriver URL long enough to require packaged-shell bootstrap.',
        )
      } catch {
        await browser.url(desktopSmokePackagedUrl)
        await waitForShellMarkerOrUrl(
          desktopSmokePackagedUrl,
          Math.min(timeout, 12_000),
          `Desktop smoke never attached to the packaged shell at ${desktopSmokePackagedUrl}.`,
        )
      }
    } else {
      await browser.url(desktopSmokeAppUrl)
      await waitForShellMarkerOrUrl(
        desktopSmokeAppUrl,
        10_000,
        `Desktop smoke never navigated away from the blank shell to ${desktopSmokeAppUrl}.`,
      )
    }
  }

  await (await $(shellSelectors.titlebar)).waitForDisplayed({ timeout })
  await (await $(shellSelectors.shellHeader)).waitForDisplayed({ timeout })
  await browser.waitUntil(
    async () => {
      const workspaceBarVisible = await (await $(shellSelectors.workspaceBar))
        .isDisplayed()
        .catch(() => false)
      const advancedModeVisible = await (await $('#uiModeBar [data-ui-mode="advanced"]'))
        .isDisplayed()
        .catch(() => false)
      return workspaceBarVisible || advancedModeVisible
    },
    {
      timeout,
      timeoutMsg: 'Shell did not expose either Advanced workspace chrome or the UI mode switcher.',
    },
  )
  if (!(await (await $(shellSelectors.workspaceBar)).isDisplayed().catch(() => false))) {
    const advancedModeButton = await $('#uiModeBar [data-ui-mode="advanced"]')
    await advancedModeButton.waitForClickable({ timeout })
    await advancedModeButton.click()
  }
  await (await $(shellSelectors.workspaceBar)).waitForDisplayed({ timeout })
  await (await $(shellSelectors.runFooter)).waitForDisplayed({ timeout })

  await browser.waitUntil(
      async () => {
        const workspaceBar = await $(shellSelectors.workspaceBar)
        const text = await workspaceBar.getText()
        return ['Research', 'Library', 'Aggregate', 'Diagnostics', 'Settings'].every((label) =>
          text.includes(label),
        )
      },
    {
      timeout,
      timeoutMsg: 'Workspace bar did not expose the expected top-level sections.',
    },
  )
}

export async function openWorkspace(key: WorkspaceKey, timeout = 10000): Promise<void> {
  const tab = await $(workspaceTabSelector(key))
  await tab.waitForDisplayed({ timeout })
  await tab.click()
  await browser.waitUntil(
    async () => (await tab.getAttribute('aria-selected')) === 'true',
    {
      timeout,
      timeoutMsg: `Workspace ${key} never became selected.`,
    },
  )
}

export async function openCommandPalette(timeout = 10000): Promise<void> {
  const button = await $(shellSelectors.commandPaletteButton)
  await button.waitForClickable({ timeout })
  await button.click()
  const dialog = await $(commandPaletteSelectors.dialog)
  await dialog.waitForDisplayed({ timeout })
  const input = await $(commandPaletteSelectors.searchInput)
  await input.waitForDisplayed({ timeout })
  const inputFocused = await browser.execute((selector) => {
    const doc = (globalThis as { document?: { querySelector: (value: string) => unknown; activeElement: unknown } }).document
    const inputEl = doc?.querySelector(selector)
    return Boolean(inputEl && doc?.activeElement === inputEl)
  }, commandPaletteSelectors.searchInput)
  if (!inputFocused) {
    await input.click()
  }
}

export async function closeCommandPalette(timeout = 10000): Promise<void> {
  await browser.keys('Escape')
  await browser.waitUntil(
    async () => !(await (await $(commandPaletteSelectors.dialog)).isDisplayed().catch(() => false)),
    {
      timeout,
      timeoutMsg: 'Command palette did not close.',
    },
  )
}

export async function openTaskCenter(timeout = 10000): Promise<void> {
  const launcher = await $(taskCenterSelectors.launcher)
  await launcher.waitForDisplayed({ timeout })
  try {
    await launcher.waitForClickable({ timeout })
    await launcher.click()
  } catch {
    await browser.execute((selector) => {
      const doc = (globalThis as { document?: { querySelector: (value: string) => { click?: () => void } | null } }).document
      const element = doc?.querySelector(selector)
      element?.click?.()
    }, taskCenterSelectors.launcher)
  }
  await (await $(taskCenterSelectors.dialog)).waitForDisplayed({ timeout })
}

export async function closeTaskCenter(timeout = 10000): Promise<void> {
  const dialog = await $(taskCenterSelectors.dialog)
  await dialog.waitForDisplayed({ timeout })
  await browser.keys('Escape')
  await browser.waitUntil(
    async () => !(await (await $(taskCenterSelectors.dialog)).isDisplayed().catch(() => false)),
    {
      timeout,
      timeoutMsg: 'Task center did not close.',
    },
  )
}

export async function waitForSettingsWorkspace(timeout = 10000): Promise<void> {
  await (await $(shellSelectors.settingsGrid)).waitForDisplayed({ timeout })
  await (await $(shellSelectors.settingsDiagnosticsSection)).waitForDisplayed({ timeout })
}
