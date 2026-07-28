import type { BrowserContext, Page } from '@playwright/test'
import type { MockTauriScheduledEvent } from '../fixtures/types'

export interface MockTauriOptions {
  holdStartupOverlay?: boolean
  strictUnknownInvoke?: boolean
  commandResults?: Record<string, unknown>
  commandDelays?: Record<string, number>
  commandEvents?: Record<string, MockTauriScheduledEvent[]>
}

export interface MockTauriSequence {
  __mockSequence: unknown[]
}

export function mockTauriSequence(...responses: unknown[]): MockTauriSequence {
  return { __mockSequence: responses }
}

const HOLD_STORAGE_KEY = 'VTA_STARTUP_OVERLAY_HOLD'
const DEFAULT_COMMAND_RESULTS: Record<string, unknown> = {
  load_latest_run_artifact_hydration: null,
  load_latest_run_artifact_summary: null,
  list_live_session_artifacts: [],
  load_live_session_artifact_detail: null,
}

export async function installMockedTauri(context: BrowserContext, options: MockTauriOptions = {}): Promise<void> {
  const holdStartupOverlay = options.holdStartupOverlay ?? false
  const strictUnknownInvoke = options.strictUnknownInvoke ?? true
  const commandResults = {
    ...DEFAULT_COMMAND_RESULTS,
    ...(options.commandResults ?? {}),
  }
  await context.addInitScript(({ hold, holdKey, strictInvoke, results, delays, scheduledEvents }) => {
    const handlers: Record<string, number[]> = {}
    const callbacks: Record<number, unknown> = {}
    const invokeLog: string[] = []
    const invokeLogEntries: Array<{ command: string; args: Record<string, unknown> }> = []
    const invokeCountsStorageKey = '__VTA_TEST_INVOKE_COUNTS_V1'
    const invokeCounts = (() => {
      try {
        const parsed = JSON.parse(sessionStorage.getItem(invokeCountsStorageKey) || '{}') as unknown
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
          return parsed as Record<string, number>
        }
      } catch {
        // Fall back to an empty counter map when storage is unavailable or malformed.
      }
      return {} as Record<string, number>
    })()
    let nextCallbackId = 1

    const recordInvokeCount = (command: string): number => {
      const nextCount = Math.max(0, Number(invokeCounts[command] || 0)) + 1
      invokeCounts[command] = nextCount
      try {
        sessionStorage.setItem(invokeCountsStorageKey, JSON.stringify(invokeCounts))
      } catch {
        // The in-memory counters still cover the current page.
      }
      return nextCount
    }

    const addListener = (event: string, callbackId: number) => {
      if (!handlers[event]) handlers[event] = []
      handlers[event].push(callbackId)
      return callbackId
    }
    const removeListener = (event: string, callbackId: number) => {
      handlers[event] = (handlers[event] || []).filter((id) => id !== callbackId)
      unregisterCallback(callbackId)
    }
    const emitEvent = (event: string, payload: unknown) => {
      const listeners = handlers[event] || []
      for (const callbackId of listeners) {
        const cb = callbacks[callbackId]
        if (typeof cb !== 'function') continue
        try {
          cb({ event, id: callbackId, payload })
        } catch {
          // ignore callback failures in browser UX smoke
        }
      }
    }
    const scheduleCommandEvents = (command: string) => {
      const rows = (scheduledEvents as Record<string, Array<{ event: string; delayMs?: number; payload: unknown }>>)[command] || []
      for (const row of rows) {
        const event = String(row?.event || '').trim()
        if (!event) continue
        const delayMs = Math.max(0, Number(row.delayMs || 0))
        globalThis.setTimeout(() => emitEvent(event, row.payload), delayMs)
      }
    }
    const waitForCommandDelay = async (command: string) => {
      const delayMs = Math.max(0, Number((delays as Record<string, unknown>)[command] || 0))
      if (delayMs <= 0) return
      await new Promise<void>((resolve) => globalThis.setTimeout(resolve, delayMs))
    }
    const listen = async (event: string, cb: (payload: { event: string; payload: unknown }) => void) => {
      const callbackId = transformCallback(cb)
      addListener(event, callbackId)
      return () => {
        removeListener(event, callbackId)
      }
    }

    const invoke = async (command: string, args: Record<string, unknown> = {}) => {
      const key = String(command ?? '')
      invokeLog.push(key)
      const invokeCount = recordInvokeCount(key)
      try {
        invokeLogEntries.push({
          command: key,
          args: JSON.parse(JSON.stringify(args ?? {})) as Record<string, unknown>,
        })
      } catch {
        invokeLogEntries.push({ command: key, args })
      }
      if (key === 'plugin:event|listen') {
        return addListener(String(args.event || ''), Number(args.handler))
      }
      if (key === 'plugin:event|unlisten') {
        removeListener(String(args.event || ''), Number(args.eventId))
        return null
      }
      if (key === 'plugin:event|emit') {
        emitEvent(String(args.event || ''), args.payload)
        return null
      }
      if (Object.prototype.hasOwnProperty.call(results, key)) {
        scheduleCommandEvents(key)
        await waitForCommandDelay(key)
        const configuredResult = (results as Record<string, unknown>)[key]
        const sequence =
          configuredResult
          && typeof configuredResult === 'object'
          && Array.isArray((configuredResult as { __mockSequence?: unknown }).__mockSequence)
            ? (configuredResult as { __mockSequence: unknown[] }).__mockSequence
            : null
        const result = sequence
          ? sequence[Math.min(invokeCount - 1, Math.max(0, sequence.length - 1))]
          : configuredResult
        if (
          result
          && typeof result === 'object'
          && Object.prototype.hasOwnProperty.call(result, '__mockFailure')
        ) {
          throw new Error(String((result as { __mockFailure?: unknown }).__mockFailure || `Mocked ${key} failure`))
        }
        return result
      }
      if (strictInvoke) {
        throw new Error(`Unexpected mocked Tauri invoke: ${key}`)
      }
      return null
    }
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
        sessionStorage.setItem(holdKey, '1')
      } catch {
        // ignore storage failures in mocked browser harness
      }
    } else {
      try {
        sessionStorage.removeItem(holdKey)
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
    ;(
      window as typeof window & {
        __TAURI_EVENT_PLUGIN_INTERNALS__?: { unregisterListener: (event: string, eventId: number) => void }
      }
    ).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: removeListener,
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
      emitEvent(event, payload)
    }
    ;(window as typeof window & { __VTA_TEST_INVOKES?: string[] }).__VTA_TEST_INVOKES = invokeLog
    ;(
      window as typeof window & {
        __VTA_TEST_INVOKE_COUNTS?: Record<string, number>
      }
    ).__VTA_TEST_INVOKE_COUNTS = invokeCounts
    ;(
      window as typeof window & {
        __VTA_TEST_INVOKE_LOG?: Array<{ command: string; args: Record<string, unknown> }>
      }
    ).__VTA_TEST_INVOKE_LOG = invokeLogEntries
  }, {
    hold: holdStartupOverlay,
    holdKey: HOLD_STORAGE_KEY,
    strictInvoke: strictUnknownInvoke,
    results: commandResults,
    delays: options.commandDelays ?? {},
    scheduledEvents: options.commandEvents ?? {},
  })
}

export async function dismissStartupOverlay(page: Page): Promise<void> {
  const overlay = page.locator('#startupAnimationOverlay')
  if (await overlay.count() === 0) return
  try {
    await overlay.waitFor({ state: 'visible', timeout: 2_000 })
  } catch {
    return
  }
  await page.waitForTimeout(460)
  const dismissButton = page.locator('.startupSkipButton')
  await dismissButton.click()
  await overlay.waitFor({ state: 'detached', timeout: 10_000 })
}
