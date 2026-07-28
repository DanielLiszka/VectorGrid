import { $, browser } from '@wdio/globals'
import { desktopSmokeAppUrl, desktopSmokeDisablePreview, desktopSmokePackagedUrl } from './appPaths.js'
import {
  commandPaletteSelectors,
  shellSelectors,
  taskCenterSelectors,
  workbenchSelectors,
  type WorkspaceKey,
  workspaceTabSelector,
} from './selectors.js'

export async function expectedWorkspaceLabels(): Promise<string[]> {
  const workbenchVisible = await (await $(workbenchSelectors.root)).isDisplayed().catch(() => false)
  return workbenchVisible
    ? ['Research', 'Library', 'Runs', 'Live', 'Settings', 'Diag']
    : ['Research', 'Library', 'Aggregate', 'Diagnostics', 'Live', 'Settings']
}

async function waitForShellMarkerOrUrl(urlPrefix: string, timeout: number, timeoutMsg: string): Promise<void> {
  await browser.waitUntil(
    async () => {
      const nextUrl = await browser.getUrl().catch(() => '')
      if (nextUrl && nextUrl !== 'about:blank' && nextUrl.startsWith(urlPrefix)) {
        return true
      }
      const titlebarVisible = await (await $(shellSelectors.titlebar))
        .isDisplayed()
        .catch(() => false)
      return titlebarVisible
    },
    {
      timeout,
      interval: 250,
      timeoutMsg,
    },
  )
}

export async function waitForShellReady(timeout = 30000): Promise<void> {
  const currentUrl = await browser.getUrl()
  if (!currentUrl || currentUrl === 'about:blank') {
    if (desktopSmokeDisablePreview) {
      // Packaged Tauri sessions consistently attach WebDriver to about:blank
      // first. Bootstrap immediately; waiting here can let that renderer detach
      // before the tauri.localhost navigation is issued.
      await browser.url(desktopSmokePackagedUrl)
      await waitForShellMarkerOrUrl(
        desktopSmokePackagedUrl,
        Math.min(timeout, 12_000),
        `Desktop smoke never attached to the packaged shell at ${desktopSmokePackagedUrl}.`,
      )
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
      return workspaceBarVisible
    },
    {
      timeout,
      timeoutMsg: 'Shell did not expose the Advanced workspace chrome.',
    },
  )
  await (await $(shellSelectors.workspaceBar)).waitForDisplayed({ timeout })
  await (await $(shellSelectors.runFooter)).waitForDisplayed({ timeout })

  await browser.waitUntil(
      async () => {
        const workspaceBar = await $(shellSelectors.workspaceBar)
        const text = await workspaceBar.getText()
        const labels = await expectedWorkspaceLabels()
        return labels.every((label) => text.includes(label))
      },
    {
      timeout,
      timeoutMsg: 'Workspace bar did not expose the expected top-level sections.',
    },
  )
}

export async function openWorkspace(key: WorkspaceKey, timeout = 10000): Promise<void> {
  const selector = workspaceTabSelector(key)
  const tab = await $(selector)
  await tab.waitForDisplayed({ timeout })
  await tab.waitForClickable({ timeout })
  await browser.waitUntil(
    async () => {
      const currentTab = await $(selector)
      if ((await currentTab.getAttribute('aria-selected').catch(() => 'false')) === 'true') return true
      if (!(await currentTab.isClickable().catch(() => false))) return false
      await currentTab.click().catch(() => undefined)
      return (await currentTab.getAttribute('aria-selected').catch(() => 'false')) === 'true'
    },
    {
      timeout,
      interval: 200,
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
