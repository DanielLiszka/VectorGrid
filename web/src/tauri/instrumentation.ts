import {
  applyDistributedProgress,
  applyDoubleMaProgress,
  beginTrackedCommand,
  completeTrackedCommand,
  failTrackedCommand,
  recordPolygonLiveBarEvent,
  recordLiveExecutionRunnerEvent,
  recordPolygonLiveStatusEvent,
  type DistributedProgressPayload,
  type DoubleMaProgressPayload,
  type LiveExecutionRunnerEventPayload,
  type PolygonLiveBarEventPayload,
  type PolygonLiveStatusEventPayload,
} from '../stores/taskStore'
import { hasTauriEventApi, listenEvent } from './events'

type InvokeFn = (command: string, args?: Record<string, unknown>) => Promise<unknown>

export const INSTRUMENTED_EVENT_CHANNELS = [
  'double_ma_progress',
  'distributed_progress',
  'live_execution_runner_event',
  'polygon_live_status',
  'polygon_live_bar',
] as const

function isInvokePropertyWritable(owner: Record<string, unknown>, key: 'invoke'): boolean {
  const descriptor = Object.getOwnPropertyDescriptor(owner, key)
  if (!descriptor) return true
  if ('writable' in descriptor) return descriptor.writable === true
  return typeof descriptor.set === 'function'
}

function globalInvokeTarget(): { owner: Record<string, unknown>; key: 'invoke'; fn: InvokeFn } | null {
  const tauri = window.__TAURI__
  if (!tauri) return null

  if (tauri.core && typeof tauri.core.invoke === 'function') {
    return { owner: tauri.core as unknown as Record<string, unknown>, key: 'invoke', fn: tauri.core.invoke }
  }
  if (typeof tauri.invoke === 'function') {
    return { owner: tauri as unknown as Record<string, unknown>, key: 'invoke', fn: tauri.invoke }
  }
  return null
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

function normalizeDistributed(payload: unknown): DistributedProgressPayload | null {
  const rec = asRecord(payload)
  if (!rec) return null
  return {
    mode: asString(rec.mode),
    completed: asNumber(rec.completed),
    total: asNumber(rec.total),
    successes: asNumber(rec.successes),
    failures: asNumber(rec.failures),
    last: asString(rec.last),
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

function isCancelledErrorMessage(err: unknown): boolean {
  const msg = asString(err instanceof Error ? err.message : err)
  const lower = msg.toLowerCase()
  return lower.includes('cancel') || lower.includes('aborted')
}

function patchInvokeOnce(): boolean {
  if (window.__VTA_INVOKE_PATCHED__) return true
  if (window.__VTA_INVOKE_PATCH_BLOCKED__) return true
  const target = globalInvokeTarget()
  if (!target) return false

  if (!isInvokePropertyWritable(target.owner, target.key)) {
    window.__VTA_INVOKE_PATCH_BLOCKED__ = true
    return true
  }

  const original = target.fn.bind(target.owner)
  const wrapped: InvokeFn = async (command: string, args?: Record<string, unknown>) => {
    const commandName = asString(command) || 'unknown_command'
    const taskId = beginTrackedCommand(commandName, { queued: true })
    try {
      const result = await original(commandName, args)
      completeTrackedCommand(taskId)
      return result
    } catch (err) {
      failTrackedCommand(taskId, err)
      if (isCancelledErrorMessage(err)) {
        // Preserve original rejection while still classifying cancellation in the store.
      }
      throw err
    }
  }

  try {
    target.owner[target.key] = wrapped
  } catch {
    window.__VTA_INVOKE_PATCH_BLOCKED__ = true
    return true
  }
  if (target.owner[target.key] !== wrapped) {
    window.__VTA_INVOKE_PATCH_BLOCKED__ = true
    return true
  }

  window.__VTA_INVOKE_PATCHED__ = true
  return true
}

function clearEventSubscriptions(): void {
  const subs = window.__VTA_EVENT_UNSUBSCRIBERS__
  if (!subs || subs.length === 0) return
  for (const off of subs) {
    try {
      off()
    } catch {
      // no-op
    }
  }
  window.__VTA_EVENT_UNSUBSCRIBERS__ = []
}

async function attachListener(event: string, handler: (payload: unknown) => void): Promise<void> {
  try {
    const off = await listenEvent(event, (payload) => handler(payload))
    if (typeof off === 'function') {
      if (!window.__VTA_EVENT_UNSUBSCRIBERS__) window.__VTA_EVENT_UNSUBSCRIBERS__ = []
      window.__VTA_EVENT_UNSUBSCRIBERS__.push(off)
    }
  } catch {
    // no-op: events are optional depending on runtime context.
  }
}

async function subscribeProgressEvents(): Promise<void> {
  clearEventSubscriptions()

  await attachListener('double_ma_progress', (payload) => {
    const p = normalizeDoubleMa(payload)
    if (p) applyDoubleMaProgress(p)
  })
  await attachListener('distributed_progress', (payload) => {
    const p = normalizeDistributed(payload)
    if (p) applyDistributedProgress(p)
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
}

function subscribeWithRetry(maxTries = 30, everyMs = 100): void {
  let tries = 0
  const run = async (): Promise<boolean> => {
    if (!hasTauriEventApi()) return false
    await subscribeProgressEvents()
    return true
  }

  void run().then((ok) => {
    if (ok) return
    const timer = window.setInterval(() => {
      tries += 1
      void run().then((done) => {
        if (done || tries >= maxTries) {
          window.clearInterval(timer)
        }
      })
    }, everyMs)
  })
}

function patchInvokeWithRetry(maxTries = 30, everyMs = 100): void {
  if (patchInvokeOnce()) return
  let tries = 0
  const timer = window.setInterval(() => {
    tries += 1
    if (patchInvokeOnce() || tries >= maxTries) {
      window.clearInterval(timer)
    }
  }, everyMs)
}

export function installTauriInstrumentation(): void {
  patchInvokeWithRetry()
  subscribeWithRetry()
  if (import.meta.hot) {
    import.meta.hot.dispose(() => {
      clearEventSubscriptions()
    })
  }
}
