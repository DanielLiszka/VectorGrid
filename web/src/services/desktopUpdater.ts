import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { tauriRuntimeAvailable } from '../tauri/invoke'

export type DesktopUpdaterStatus =
  | 'unavailable'
  | 'idle'
  | 'checking'
  | 'up-to-date'
  | 'update-available'
  | 'error'

export interface DesktopUpdateInfo {
  available: boolean
  version: string
  currentVersion: string
  date: string
  body: string
}

export interface DesktopUpdaterCheckResult {
  status: DesktopUpdaterStatus
  update: DesktopUpdateInfo | null
  error?: string
}

let pendingUpdate: Update | null = null

function errorMessage(error: unknown): string {
  return String(error instanceof Error ? error.message : error)
}

function normalizeUpdate(update: Update): DesktopUpdateInfo {
  return {
    available: true,
    version: String(update.version ?? ''),
    currentVersion: String(update.currentVersion ?? ''),
    date: String(update.date ?? ''),
    body: String(update.body ?? ''),
  }
}

export function resetDesktopUpdaterForTests(): void {
  pendingUpdate = null
}

export async function checkForDesktopUpdate(): Promise<DesktopUpdaterCheckResult> {
  if (!tauriRuntimeAvailable()) {
    pendingUpdate = null
    return { status: 'unavailable', update: null }
  }

  try {
    const update = await check()
    pendingUpdate = update ?? null
    if (!update) return { status: 'up-to-date', update: null }
    return {
      status: 'update-available',
      update: normalizeUpdate(update),
    }
  } catch (error) {
    pendingUpdate = null
    return {
      status: 'error',
      update: null,
      error: errorMessage(error),
    }
  }
}

export async function downloadAndInstallDesktopUpdate(): Promise<DesktopUpdaterCheckResult> {
  if (!tauriRuntimeAvailable()) {
    return { status: 'unavailable', update: null }
  }

  const update = pendingUpdate ?? (await check())
  pendingUpdate = update ?? null
  if (!update) return { status: 'up-to-date', update: null }

  try {
    await update.downloadAndInstall()
    return {
      status: 'update-available',
      update: normalizeUpdate(update),
    }
  } catch (error) {
    return {
      status: 'error',
      update: normalizeUpdate(update),
      error: errorMessage(error),
    }
  }
}

export async function relaunchAfterDesktopUpdate(): Promise<boolean> {
  if (!tauriRuntimeAvailable()) return false
  await relaunch()
  return true
}
