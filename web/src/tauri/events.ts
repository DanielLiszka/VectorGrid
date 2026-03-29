import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { normalizeTauriRuntimeError } from './runtimeError';

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
  if (!hasTauriEventApi()) return null;
  try {
    const off = await listen(event, (evt) => {
      const meta = typeof evt.id === 'number' ? { event: evt.event, id: evt.id } : { event: evt.event };
      handler(evt.payload as TPayload, meta);
    });
    return typeof off === "function" ? off : null;
  } catch (cause) {
    throw normalizeTauriRuntimeError({
      code: 'TAURI_EVENT_LISTEN_FAILED',
      operation: `listen:${event}`,
      fallbackMessage: `Failed to subscribe to Tauri event "${event}".`,
      causeValue: cause,
    });
  }
}
