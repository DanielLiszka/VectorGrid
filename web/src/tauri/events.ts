import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { normalizeTauriRuntimeError } from './runtimeError'

type SharedEventHandler = (
  payload: unknown,
  meta: { event: string; id?: number },
) => void

interface SharedChannelListener {
  nextHandlerId: number
  handlers: Map<number, SharedEventHandler>
  nativeOff: (() => void | Promise<void>) | null
  subscribePromise: Promise<void> | null
}

const sharedChannels = new Map<string, SharedChannelListener>()

function sharedChannel(event: string): SharedChannelListener {
  let channel = sharedChannels.get(event)
  if (!channel) {
    channel = {
      nextHandlerId: 0,
      handlers: new Map(),
      nativeOff: null,
      subscribePromise: null,
    }
    sharedChannels.set(event, channel)
  }
  return channel
}

function clearSharedChannel(event: string, channel: SharedChannelListener): void {
  try {
    const offResult = channel.nativeOff?.()
    if (offResult && typeof (offResult as Promise<void>).catch === 'function') {
      void (offResult as Promise<void>).catch(() => {
        // best-effort teardown only
      })
    }
  } catch {
    // best-effort teardown only
  }
  channel.nativeOff = null
  channel.subscribePromise = null
  sharedChannels.delete(event)
}

async function ensureNativeListener(
  event: string,
  channel: SharedChannelListener,
): Promise<void> {
  if (channel.nativeOff || channel.subscribePromise) {
    await channel.subscribePromise
    return
  }

  channel.subscribePromise = (async () => {
    try {
      const off = await listen(event, (evt) => {
        const meta =
          typeof evt.id === 'number'
            ? { event: evt.event, id: evt.id }
            : { event: evt.event }
        const handlers = Array.from(channel.handlers.values())
        for (const handler of handlers) {
          try {
            handler(evt.payload, meta)
          } catch {
            // keep one misbehaving consumer from breaking the others
          }
        }
      })
      channel.nativeOff = typeof off === 'function' ? off : null
    } catch (cause) {
      channel.handlers.clear()
      clearSharedChannel(event, channel)
      throw normalizeTauriRuntimeError({
        code: 'TAURI_EVENT_LISTEN_FAILED',
        operation: `listen:${event}`,
        fallbackMessage: `Failed to subscribe to Tauri event "${event}".`,
        causeValue: cause,
      })
    } finally {
      channel.subscribePromise = null
    }
  })()

  await channel.subscribePromise
}

function detachSharedHandler(
  event: string,
  channel: SharedChannelListener,
  handlerId: number,
): void {
  channel.handlers.delete(handlerId)
  if (channel.handlers.size > 0) return
  if (channel.subscribePromise) {
    void channel.subscribePromise.finally(() => {
      const latest = sharedChannels.get(event)
      if (latest !== channel || latest.handlers.size > 0) return
      clearSharedChannel(event, channel)
    })
    return
  }
  clearSharedChannel(event, channel)
}

export function hasTauriEventApi(): boolean {
  try {
    return isTauri()
  } catch {
    return false
  }
}

export async function listenEvent<TPayload = unknown>(
  event: string,
  handler: (payload: TPayload, meta: { event: string; id?: number }) => void,
): Promise<(() => void) | null> {
  if (!hasTauriEventApi()) return null

  const channel = sharedChannel(event)
  channel.nextHandlerId += 1
  const handlerId = channel.nextHandlerId
  channel.handlers.set(handlerId, handler as SharedEventHandler)

  try {
    await ensureNativeListener(event, channel)
  } catch (error) {
    channel.handlers.delete(handlerId)
    if (channel.handlers.size === 0 && sharedChannels.get(event) === channel) {
      clearSharedChannel(event, channel)
    }
    throw error
  }

  return () => {
    detachSharedHandler(event, channel, handlerId)
  }
}

export function resetSharedTauriEventListenersForTests(): void {
  for (const [event, channel] of sharedChannels.entries()) {
    channel.handlers.clear()
    clearSharedChannel(event, channel)
  }
}
