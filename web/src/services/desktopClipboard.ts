import { writeText as writeNativeClipboardText } from '@tauri-apps/plugin-clipboard-manager'
import { tauriRuntimeAvailable } from '../tauri/invoke'

export type DesktopClipboardWriteMethod =
  | 'tauri-clipboard'
  | 'navigator-clipboard'
  | 'exec-command'

export interface DesktopClipboardWriteResult {
  ok: boolean
  method: DesktopClipboardWriteMethod | 'none'
  error?: string
}

function normalizeClipboardText(text: unknown): string {
  return String(text ?? '')
}

function errorMessage(error: unknown): string {
  return String(error instanceof Error ? error.message : error)
}

async function writeWithNavigatorClipboard(text: string): Promise<boolean> {
  const clipboard =
    typeof navigator === 'undefined' ? null : navigator.clipboard
  if (!clipboard?.writeText) return false
  await clipboard.writeText(text)
  return true
}

function writeWithExecCommand(text: string): boolean {
  if (typeof document === 'undefined' || typeof document.execCommand !== 'function') return false
  const textarea = document.createElement('textarea')
  textarea.value = text
  textarea.setAttribute('readonly', 'true')
  textarea.style.position = 'fixed'
  textarea.style.left = '-9999px'
  textarea.style.top = '-9999px'
  document.body.appendChild(textarea)
  textarea.select()
  try {
    return document.execCommand('copy')
  } finally {
    textarea.remove()
  }
}

export async function copyTextToDesktopClipboard(
  text: unknown,
): Promise<DesktopClipboardWriteResult> {
  const normalized = normalizeClipboardText(text)
  let lastError = ''

  if (tauriRuntimeAvailable()) {
    try {
      await writeNativeClipboardText(normalized)
      return { ok: true, method: 'tauri-clipboard' }
    } catch (error) {
      lastError = errorMessage(error)
    }
  }

  try {
    if (await writeWithNavigatorClipboard(normalized)) {
      return { ok: true, method: 'navigator-clipboard' }
    }
  } catch (error) {
    lastError = errorMessage(error)
  }

  try {
    if (writeWithExecCommand(normalized)) {
      return { ok: true, method: 'exec-command' }
    }
  } catch (error) {
    lastError = errorMessage(error)
  }

  return {
    ok: false,
    method: 'none',
    ...(lastError ? { error: lastError } : {}),
  }
}
