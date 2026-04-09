import type { BrowserContext, Page } from '@playwright/test'

export interface MockTauriOptions {
  holdStartupOverlay?: boolean
}

const HOLD_STORAGE_KEY = 'VTA_STARTUP_OVERLAY_HOLD'

export async function installMockedTauri(context: BrowserContext, options: MockTauriOptions = {}): Promise<void> {
  const holdStartupOverlay = options.holdStartupOverlay ?? false
  await context.addInitScript(({ hold, holdKey }) => {
    const handlers: Record<string, Array<(event: { event: string; payload: unknown }) => void>> = {}
    const callbacks: Record<number, unknown> = {}
    let nextCallbackId = 1

    const listen = async (event: string, cb: (payload: { event: string; payload: unknown }) => void) => {
      if (!handlers[event]) handlers[event] = []
      handlers[event].push(cb)
      return () => {
        handlers[event] = (handlers[event] || []).filter((fn) => fn !== cb)
      }
    }

    const invoke = async () => null
    const transformCallback = (cb: unknown) => {
      const id = nextCallbackId++
      callbacks[id] = cb
      return id
    }
    const unregisterCallback = (id: number) => {
      delete callbacks[id]
    }

    if (hold) {
      try {
        localStorage.setItem(holdKey, '1')
      } catch {
        // ignore storage failures in mocked browser harness
      }
    } else {
      try {
        localStorage.removeItem(holdKey)
      } catch {
        // ignore storage failures in mocked browser harness
      }
    }

    ;(window as typeof window & { isTauri?: boolean }).isTauri = true
    ;(
      window as typeof window & {
        __TAURI_INTERNALS__?: {
          invoke: typeof invoke
          transformCallback: typeof transformCallback
          unregisterCallback: typeof unregisterCallback
          convertFileSrc: (value: string) => string
          metadata: {
            currentWindow: { label: string }
            currentWebview: { label: string }
          }
        }
      }
    ).__TAURI_INTERNALS__ = {
      invoke,
      transformCallback,
      unregisterCallback,
      convertFileSrc: (value: string) => value,
      metadata: {
        currentWindow: { label: 'main' },
        currentWebview: { label: 'main' },
      },
    }
    ;(window as typeof window & { __TAURI_EVENT_PLUGIN_INTERNALS__?: { unregisterListener: () => void } }).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: () => {},
    }

    const tauri = (window as typeof window & { __TAURI__?: Record<string, unknown> }).__TAURI__ || {}
    tauri.event = { listen }
    tauri.core = { invoke }
    tauri.invoke = invoke
    ;(window as typeof window & { __TAURI__?: Record<string, unknown> }).__TAURI__ = tauri

    ;(window as typeof window & { __VTA_TEST_EMIT?: (event: string, payload: unknown) => void }).__VTA_TEST_EMIT = (
      event: string,
      payload: unknown,
    ) => {
      const listeners = handlers[event] || []
      for (const cb of listeners) {
        try {
          cb({ event, payload })
        } catch {
          // ignore callback failures in browser UX smoke
        }
      }
    }
  }, { hold: holdStartupOverlay, holdKey: HOLD_STORAGE_KEY })
}

export async function dismissStartupOverlay(page: Page): Promise<void> {
  const overlay = page.locator('#startupAnimationOverlay')
  await overlay.waitFor({ state: 'visible', timeout: 15_000 })
  await page.waitForTimeout(460)
  const dismissButton = page.locator('.startupSkipButton')
  await dismissButton.click()
  await overlay.waitFor({ state: 'detached', timeout: 10_000 })
}
