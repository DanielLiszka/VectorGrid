import { expect, type Page } from '@playwright/test'

export interface QaGuardOptions {
  allowedConsoleErrors?: RegExp[]
  failOnRequestFailure?: boolean
}

export interface QaGuard {
  consoleErrors: string[]
  pageErrors: string[]
  requestFailures: string[]
  assertClean: () => Promise<void>
}

export function installQaGuards(page: Page, options: QaGuardOptions = {}): QaGuard {
  const consoleErrors: string[] = []
  const pageErrors: string[] = []
  const requestFailures: string[] = []

  page.on('console', (msg) => {
    if (msg.type() !== 'error') return
    const text = msg.text()
    if ((options.allowedConsoleErrors ?? []).some((pattern) => pattern.test(text))) return
    consoleErrors.push(text)
  })

  page.on('pageerror', (error) => {
    pageErrors.push(error.stack || error.message)
  })

  if (options.failOnRequestFailure ?? true) {
    page.on('requestfailed', (request) => {
      const url = request.url()
      if (url.startsWith('data:') || url.startsWith('blob:')) return
      requestFailures.push(`${request.failure()?.errorText || 'failed'} ${url}`)
    })
  }

  return {
    consoleErrors,
    pageErrors,
    requestFailures,
    async assertClean() {
      expect(pageErrors, 'uncaught page errors').toEqual([])
      expect(consoleErrors, 'unexpected console errors').toEqual([])
      expect(requestFailures, 'failed app requests').toEqual([])
    },
  }
}

export async function expectNoHorizontalDocumentOverflow(page: Page): Promise<void> {
  const overflow = await page.evaluate(() => Math.max(
    document.documentElement.scrollWidth - document.documentElement.clientWidth,
    document.body.scrollWidth - window.innerWidth,
  ))
  expect(overflow, 'document horizontal overflow px').toBeLessThanOrEqual(2)
}

export async function expectNoStuckBusyState(page: Page, timeoutMs = 5000): Promise<void> {
  await expect
    .poll(async () => {
      return page.locator('[aria-busy="true"], button:has-text("Running"), button:has-text("Cancelling")').count()
    }, { timeout: timeoutMs })
    .toBe(0)
}
