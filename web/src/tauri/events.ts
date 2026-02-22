import { normalizeTauriRuntimeError } from './runtimeError';

type ListenHandler = (event: { event: string; id?: number; payload: unknown }) => void;
type ListenFn = (
  event: string,
  handler: ListenHandler,
) => Promise<() => void> | (() => void);

function globalListenFn(): ListenFn | null {
  const tauri = window.__TAURI__;
  if (!tauri?.event) return null;
  if (typeof tauri.event.listen !== "function") return null;
  return tauri.event.listen;
}

export function hasTauriEventApi(): boolean {
  return !!globalListenFn();
}

export async function listenEvent<TPayload = unknown>(
  event: string,
  handler: (payload: TPayload, meta: { event: string; id?: number }) => void,
): Promise<(() => void) | null> {
  const listen = globalListenFn();
  if (!listen) return null;
  try {
    const maybeOff = listen(event, (evt) => {
      const meta = typeof evt.id === 'number' ? { event: evt.event, id: evt.id } : { event: evt.event };
      handler(evt.payload as TPayload, meta);
    });
    const off = await Promise.resolve(maybeOff);
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
