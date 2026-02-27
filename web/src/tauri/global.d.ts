export {}

type TauriInvokeFn = (command: string, args?: Record<string, unknown>) => Promise<unknown>
type TauriListenFn = (
  event: string,
  handler: (event: { event: string; id?: number; payload: unknown }) => void,
) => Promise<() => void> | (() => void)

declare global {
  interface Window {
    __TAURI__?: {
      core?: {
        invoke?: TauriInvokeFn
      }
      invoke?: TauriInvokeFn
      event?: {
        listen?: TauriListenFn
      }
    }
    __VTA_INVOKE_PATCHED__?: boolean
    __VTA_INVOKE_PATCH_BLOCKED__?: boolean
    __VTA_EVENT_UNSUBSCRIBERS__?: Array<() => void>
    __VTA_LEGACY_BOOTED__?: boolean
  }
}
