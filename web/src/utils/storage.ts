import {
  AGGREGATE_TAB_KEY,
  CONTROLS_COLLAPSED_KEY,
  CONTROLS_TAB_KEY,
  CONTROLS_WIDTH_KEY,
  DIST_SETTINGS_KEY,
  EXEC_KEYS_STORAGE_KEY,
  HISTORY_KEY,
  LAST_BATCH_SUMMARY_KEY,
  RECENT_CSV_KEY,
  RESULTS_TAB_KEY,
  SETTINGS_KEY,
  THEME_PREF_KEY,
  WINDOW_EFFECT_KEY,
  WORKFLOW_ADVANCED_KEY,
  WORKFLOW_STATE_KEY,
  WORKFLOW_STEP_KEY,
  WORKFLOW_TAB_KEY,
  WORKSPACE_TAB_KEY,
} from '../legacy/core/storageKeys.js'

export const LEGACY_STORAGE_KEYS = [
  CONTROLS_TAB_KEY,
  RESULTS_TAB_KEY,
  SETTINGS_KEY,
  HISTORY_KEY,
  RECENT_CSV_KEY,
  LAST_BATCH_SUMMARY_KEY,
  EXEC_KEYS_STORAGE_KEY,
  WORKFLOW_TAB_KEY,
  WORKFLOW_STATE_KEY,
  WORKSPACE_TAB_KEY,
  WORKFLOW_STEP_KEY,
  WORKFLOW_ADVANCED_KEY,
  AGGREGATE_TAB_KEY,
  CONTROLS_WIDTH_KEY,
  CONTROLS_COLLAPSED_KEY,
  WINDOW_EFFECT_KEY,
  THEME_PREF_KEY,
  DIST_SETTINGS_KEY,
] as const

export type LegacyStorageKey = (typeof LEGACY_STORAGE_KEYS)[number]

export const STORAGE_SCHEMA_VERSION_KEY = 'vta_storage_schema_version_v1'
export const STORAGE_SCHEMA_VERSION = 1
export const TASK_TIMELINE_STORAGE_KEY = 'vta_task_timeline_v1'

function localStorageSafe(): Storage | null {
  try {
    if (typeof window === 'undefined') return null
    return window.localStorage ?? null
  } catch {
    return null
  }
}

export function readStorageString(key: string, fallback = ''): string {
  const storage = localStorageSafe()
  if (!storage) return fallback
  try {
    const value = storage.getItem(key)
    return value === null ? fallback : String(value)
  } catch {
    return fallback
  }
}

export function writeStorageString(key: string, value: string): boolean {
  const storage = localStorageSafe()
  if (!storage) return false
  try {
    storage.setItem(key, value)
    return true
  } catch {
    return false
  }
}

export function removeStorageKey(key: string): boolean {
  const storage = localStorageSafe()
  if (!storage) return false
  try {
    storage.removeItem(key)
    return true
  } catch {
    return false
  }
}

export function readStorageJson<T>(key: string): T | null {
  const raw = readStorageString(key, '')
  if (!raw) return null
  try {
    return JSON.parse(raw) as T
  } catch {
    return null
  }
}

export function writeStorageJson(key: string, value: unknown): boolean {
  try {
    return writeStorageString(key, JSON.stringify(value))
  } catch {
    return false
  }
}

export function readLegacyStorageKey(key: LegacyStorageKey, fallback = ''): string {
  return readStorageString(key, fallback)
}

export function writeLegacyStorageKey(key: LegacyStorageKey, value: string): boolean {
  return writeStorageString(key, value)
}

function normalizeBoolFlag(key: LegacyStorageKey): void {
  const raw = readStorageString(key, '').trim().toLowerCase()
  if (!raw) return
  if (raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on') {
    writeStorageString(key, '1')
    return
  }
  if (raw === '0' || raw === 'false' || raw === 'no' || raw === 'off') {
    removeStorageKey(key)
    return
  }
  if (raw === 'null' || raw === 'undefined') {
    removeStorageKey(key)
  }
}

function migrateToSchemaV1(): void {
  // Keep existing legacy keys; only normalize permissive bool-like values.
  normalizeBoolFlag(WORKFLOW_ADVANCED_KEY)
  normalizeBoolFlag(CONTROLS_COLLAPSED_KEY)
}

export function runStorageMigrations(): number {
  const rawVersion = readStorageString(STORAGE_SCHEMA_VERSION_KEY, '0').trim()
  let version = Number(rawVersion)
  if (!Number.isFinite(version) || version < 0) version = 0
  version = Math.floor(version)

  if (version < 1) {
    migrateToSchemaV1()
    version = 1
    writeStorageString(STORAGE_SCHEMA_VERSION_KEY, String(version))
  }

  return version
}

export function readLegacyStorageSnapshot(): Partial<Record<LegacyStorageKey, string>> {
  const snapshot: Partial<Record<LegacyStorageKey, string>> = {}
  for (const key of LEGACY_STORAGE_KEYS) {
    const value = readStorageString(key, '')
    if (value) snapshot[key] = value
  }
  return snapshot
}
