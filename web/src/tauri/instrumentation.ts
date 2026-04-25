import {
  applyDoubleMaProgress,
  applyResearchCampaignProgress,
  beginTrackedCommand,
  completeTrackedCommand,
  failTrackedCommand,
  recordPolygonLiveBarEvent,
  recordLiveExecutionRunnerEvent,
  recordPolygonLiveStatusEvent,
  TRACKED_TAURI_COMMANDS,
  type DoubleMaProgressPayload,
  type LiveExecutionRunnerEventPayload,
  type PolygonLiveBarEventPayload,
  type PolygonLiveStatusEventPayload,
  type ResearchCampaignProgressPayload,
} from '../stores/taskStore'
import { isTauriCommand, type TauriCommand } from './commands'
import {
  resetInvokeLifecycleHooksForTests,
  setInvokeLifecycleHooks,
  type InvokeArgs,
} from './invoke'
import {
  hasTauriEventApi,
  listenEvent,
  resetSharedTauriEventListenersForTests,
} from './events'

export const INSTRUMENTED_EVENT_CHANNELS = [
  'double_ma_progress',
  'research_campaign_progress',
  'live_execution_runner_event',
  'polygon_live_status',
  'polygon_live_bar',
] as const

let invokeHooksInstalled = false
let disposeRegistered = false
let eventRetryTimer: ReturnType<typeof setInterval> | null = null
let eventSubscribePromise: Promise<void> | null = null
let eventUnsubscribers: Array<() => void> = []

function isTrackedLifecycleCommand(command: string): command is TauriCommand {
  return isTauriCommand(command) && TRACKED_TAURI_COMMANDS.has(command)
}

function asRecord(payload: unknown): Record<string, unknown> | null {
  if (!payload || typeof payload !== 'object') return null
  return payload as Record<string, unknown>
}

function asNumber(v: unknown): number {
  const n = Number(v)
  return Number.isFinite(n) ? n : 0
}

function asString(v: unknown): string {
  return String(v ?? '').trim()
}

function normalizeDoubleMa(payload: unknown): DoubleMaProgressPayload | null {
  const rec = asRecord(payload)
  if (!rec) return null
  return {
    processed_pairs: asNumber(rec.processed_pairs),
    total_pairs: asNumber(rec.total_pairs),
    phase: asString(rec.phase),
  }
}

function normalizeResearchCampaign(
  payload: unknown,
): ResearchCampaignProgressPayload | null {
  const rec = asRecord(payload)
  if (!rec) return null
  return {
    stage: asString(rec.stage),
    datasetIndex: asNumber(rec.datasetIndex ?? rec.dataset_index),
    datasetTotal: asNumber(rec.datasetTotal ?? rec.dataset_total),
    label: asString(rec.label),
    message: asString(rec.message),
    optimized: asNumber(rec.optimized),
    validated: asNumber(rec.validated),
    skipped: asNumber(rec.skipped),
    failed: asNumber(rec.failed),
  }
}

function normalizeRunnerEvent(payload: unknown): LiveExecutionRunnerEventPayload | null {
  const rec = asRecord(payload)
  if (!rec) return null
  return {
    status: asString(rec.status),
    message: asString(rec.message),
    symbol: asString(rec.symbol),
    at_unix_ms: asNumber(rec.at_unix_ms),
    bar_open_unix_ms: asNumber(rec.bar_open_unix_ms),
  }
}

function normalizePolygonStatus(payload: unknown): PolygonLiveStatusEventPayload | null {
  const rec = asRecord(payload)
  if (!rec) return null
  return {
    status: asString(rec.status),
    message: asString(rec.message),
    market: asString(rec.market),
    symbol: asString(rec.symbol),
    delayed: Boolean(rec.delayed),
    channel: asString(rec.channel),
    at_unix_ms: asNumber(rec.at_unix_ms),
  }
}

function normalizePolygonBar(payload: unknown): PolygonLiveBarEventPayload | null {
  const rec = asRecord(payload)
  if (!rec) return null
  return {
    symbol: asString(rec.symbol || rec.ticker || rec.pair),
    at_unix_ms: asNumber(rec.at_unix_ms || rec.time_unix_ms || rec.t || rec.ts),
    close: asNumber(rec.close ?? rec.c),
  }
}

function clearEventRetryTimer(): void {
  if (!eventRetryTimer) return
  globalThis.clearInterval(eventRetryTimer)
  eventRetryTimer = null
}

function clearEventSubscriptions(): void {
  clearEventRetryTimer()
  for (const off of eventUnsubscribers) {
    try {
      off()
    } catch {
          }
  }
  eventUnsubscribers = []
  eventSubscribePromise = null
}

async function attachListener(
  event: string,
  handler: (payload: unknown) => void,
): Promise<void> {
  try {
    const off = await listenEvent(event, (payload) => handler(payload))
    if (typeof off === 'function') {
      eventUnsubscribers.push(off)
    }
  } catch {
      }
}

async function subscribeProgressEvents(): Promise<void> {
  if (eventUnsubscribers.length > 0 || eventSubscribePromise) {
    await eventSubscribePromise
    return
  }

  eventSubscribePromise = (async () => {
    const nextUnsubscribers: Array<() => void> = []
    const priorUnsubscribers = eventUnsubscribers
    eventUnsubscribers = nextUnsubscribers
    try {
      await attachListener('double_ma_progress', (payload) => {
        const p = normalizeDoubleMa(payload)
        if (p) applyDoubleMaProgress(p)
      })
      await attachListener('research_campaign_progress', (payload) => {
        const p = normalizeResearchCampaign(payload)
        if (p) applyResearchCampaignProgress(p)
      })
      await attachListener('live_execution_runner_event', (payload) => {
        const p = normalizeRunnerEvent(payload)
        if (p) recordLiveExecutionRunnerEvent(p)
      })
      await attachListener('polygon_live_status', (payload) => {
        const p = normalizePolygonStatus(payload)
        if (p) recordPolygonLiveStatusEvent(p)
      })
      await attachListener('polygon_live_bar', (payload) => {
        const p = normalizePolygonBar(payload)
        if (p) recordPolygonLiveBarEvent(p)
      })

      for (const off of priorUnsubscribers) {
        try {
          off()
        } catch {
                  }
      }
    } finally {
      eventSubscribePromise = null
    }
  })()

  await eventSubscribePromise
}

function subscribeWithRetry(maxTries = 30, everyMs = 100): void {
  if (eventUnsubscribers.length > 0 || eventSubscribePromise || eventRetryTimer) return

  let tries = 0
  const attempt = async (): Promise<boolean> => {
    if (!hasTauriEventApi()) return false
    await subscribeProgressEvents()
    return eventUnsubscribers.length > 0
  }

  void attempt().then((ok) => {
    if (ok) return
    eventRetryTimer = globalThis.setInterval(() => {
      tries += 1
      void attempt().then((done) => {
        if (done || tries >= maxTries) {
          clearEventRetryTimer()
        }
      })
    }, everyMs)
  })
}

function installInvokeHooks(): void {
  if (invokeHooksInstalled) return
  invokeHooksInstalled = true
  setInvokeLifecycleHooks({
    beforeInvoke(command: TauriCommand, _args?: InvokeArgs) {
      if (!isTrackedLifecycleCommand(command)) return null
      return beginTrackedCommand(command, { queued: true })
    },
    afterSuccess(token: unknown) {
      if (typeof token !== 'string' || !token) return
      completeTrackedCommand(token)
    },
    afterError(token: unknown, _command: TauriCommand, _args: InvokeArgs, error: unknown) {
      if (typeof token !== 'string' || !token) return
      failTrackedCommand(token, error)
    },
  })
}

export function installTauriInstrumentation(): void {
  installInvokeHooks()
  subscribeWithRetry()
  if (import.meta.hot && !disposeRegistered) {
    disposeRegistered = true
    import.meta.hot.dispose(() => {
      clearEventSubscriptions()
      setInvokeLifecycleHooks(null)
      invokeHooksInstalled = false
      disposeRegistered = false
    })
  }
}

export function resetTauriInstrumentationForTests(): void {
  clearEventSubscriptions()
  setInvokeLifecycleHooks(null)
  resetInvokeLifecycleHooksForTests()
  resetSharedTauriEventListenersForTests()
  invokeHooksInstalled = false
  disposeRegistered = false
}
