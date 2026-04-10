import { $, browser } from '@wdio/globals'
import { desktopSmokeAppUrl } from './appPaths.js'
import { commandPaletteSelectors, shellSelectors, taskCenterSelectors, type WorkspaceKey, workspaceTabSelector } from './selectors.js'

export async function waitForShellReady(timeout = 30000): Promise<void> {
  const currentUrl = await browser.getUrl().catch(() => '')
  if (!currentUrl || currentUrl === 'about:blank') {
    await browser.url(desktopSmokeAppUrl)
    await browser.waitUntil(
      async () => {
        const nextUrl = await browser.getUrl().catch(() => '')
        return nextUrl.startsWith(desktopSmokeAppUrl)
      },
      {
        timeout: 10_000,
        timeoutMsg: `Desktop smoke never navigated away from the blank shell to ${desktopSmokeAppUrl}.`,
      },
    )
  }

  await (await $(shellSelectors.titlebar)).waitForDisplayed({ timeout })
  await (await $(shellSelectors.shellHeader)).waitForDisplayed({ timeout })
  await (await $(shellSelectors.workspaceBar)).waitForDisplayed({ timeout })
  await (await $(shellSelectors.runFooter)).waitForDisplayed({ timeout })

  await browser.waitUntil(
      async () => {
        const workspaceBar = await $(shellSelectors.workspaceBar)
        const text = await workspaceBar.getText()
        return ['Workflow', 'Run Library', 'Aggregate Analysis', 'Pro Tools', 'Settings'].every((label) =>
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
