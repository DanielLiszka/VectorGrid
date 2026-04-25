import { mockWindows } from '@tauri-apps/api/mocks'

export function mockDesktopWindows(current: string, ...others: string[]): void {
  ;(globalThis as { isTauri?: boolean }).isTauri = true
  mockWindows(current, ...others)
}

export function resetMockWindows(): void {
  if (typeof window === 'undefined') return
  const tauriInternals = (window as Window & { __TAURI_INTERNALS__?: { metadata?: unknown } }).__TAURI_INTERNALS__
  if (tauriInternals && typeof tauriInternals === 'object') {
    delete tauriInternals.metadata
  }
}
