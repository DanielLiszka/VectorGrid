import {
  AGGREGATE_TAB_KEY,
  CONTROLS_COLLAPSED_KEY,
  CONTROLS_TAB_KEY,
  CONTROLS_WIDTH_KEY,
  DESKTOP_NOTIFICATION_MODE_KEY,
  DIST_SETTINGS_KEY,
  EXEC_KEYS_STORAGE_KEY,
  HISTORY_KEY,
  LEGACY_CONTROLS_TAB_KEY,
  LEGACY_EXEC_KEYS_STORAGE_KEY,
  LEGACY_HISTORY_KEY,
  LEGACY_LAST_BATCH_SUMMARY_KEY,
  LEGACY_RECENT_CSV_KEY,
  LEGACY_RESULTS_TAB_KEY,
  LEGACY_SETTINGS_KEY,
  LAST_BATCH_SUMMARY_KEY,
  RECENT_CSV_KEY,
  RESULTS_TAB_KEY,
  SETTINGS_KEY,
  STORAGE_KEY_ALIASES,
  THEME_PREF_KEY,
  WINDOW_EFFECT_KEY,
  WORKFLOW_ADVANCED_KEY,
  WORKFLOW_STATE_KEY,
  WORKFLOW_STEP_KEY,
  WORKFLOW_TAB_KEY,
  WORKSPACE_TAB_KEY,
} from './core/storageKeys'

export const LEGACY_STORAGE_KEYS = [
  CONTROLS_TAB_KEY,
  LEGACY_CONTROLS_TAB_KEY,
  RESULTS_TAB_KEY,
  LEGACY_RESULTS_TAB_KEY,
  SETTINGS_KEY,
  LEGACY_SETTINGS_KEY,
  HISTORY_KEY,
  LEGACY_HISTORY_KEY,
  RECENT_CSV_KEY,
  LEGACY_RECENT_CSV_KEY,
  LAST_BATCH_SUMMARY_KEY,
  LEGACY_LAST_BATCH_SUMMARY_KEY,
  EXEC_KEYS_STORAGE_KEY,
  LEGACY_EXEC_KEYS_STORAGE_KEY,
  WORKFLOW_TAB_KEY,
  WORKFLOW_STATE_KEY,
  WORKSPACE_TAB_KEY,
  WORKFLOW_STEP_KEY,
  WORKFLOW_ADVANCED_KEY,
  AGGREGATE_TAB_KEY,
  CONTROLS_WIDTH_KEY,
  CONTROLS_COLLAPSED_KEY,
  DESKTOP_NOTIFICATION_MODE_KEY,
  WINDOW_EFFECT_KEY,
  THEME_PREF_KEY,
  DIST_SETTINGS_KEY,
] as const

export type LegacyStorageKey = (typeof LEGACY_STORAGE_KEYS)[number]

export const STORAGE_SCHEMA_VERSION_KEY = 'vta_storage_schema_version_v1'
export const STORAGE_SCHEMA_VERSION = 2
export const TASK_TIMELINE_STORAGE_KEY = 'vta_task_timeline_v1'

function localStorageSafe(): Storage | null {
  try {
    const globalStorage = (globalThis as { localStorage?: Storage | null }).localStorage
    if (globalStorage) return globalStorage
    if (typeof window === 'undefined') return null
    return window.localStorage ?? null
  } catch {
    return null
  }
}

function storageKeyCandidates(key: string): string[] {
  const aliases = Array.isArray(STORAGE_KEY_ALIASES[key]) ? STORAGE_KEY_ALIASES[key] : []
  return [key, ...aliases]
}

export function readStorageString(key: string, fallback = ''): string {
  const storage = localStorageSafe()
  if (!storage) return fallback
  try {
    for (const candidate of storageKeyCandidates(key)) {
      const value = storage.getItem(candidate)
      if (value !== null) return String(value)
    }
    return fallback
  } catch {
    return fallback
  }
}

export function writeStorageString(key: string, value: string): boolean {
  const storage = localStorageSafe()
  if (!storage) return false
  try {
    storage.setItem(key, value)
    for (const alias of STORAGE_KEY_ALIASES[key] || []) {
      storage.removeItem(alias)
    }
    return true
  } catch {
    return false
  }
}

export function removeStorageKey(key: string): boolean {
  const storage = localStorageSafe()
  if (!storage) return false
  try {
    for (const candidate of storageKeyCandidates(key)) {
      storage.removeItem(candidate)
    }
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

function migrateToSchemaV2(): void {
  const storage = localStorageSafe()
  if (!storage) return
  for (const [primary, aliases] of Object.entries(STORAGE_KEY_ALIASES)) {
    const primaryValue = storage.getItem(primary)
    if (primaryValue !== null) {
      for (const alias of aliases) storage.removeItem(alias)
      continue
    }
    for (const alias of aliases) {
      const legacyValue = storage.getItem(alias)
      if (legacyValue === null) continue
      storage.setItem(primary, legacyValue)
      break
    }
  }
}

export function runStorageMigrations(): number {
  const rawVersion = readStorageString(STORAGE_SCHEMA_VERSION_KEY, '0').trim()
  let version = Number(rawVersion)
  if (!Number.isFinite(version) || version < 0) version = 0
  version = Math.floor(version)

  if (version < 1) {
    migrateToSchemaV1()
    version = 1
  }

  if (version < 2) {
    migrateToSchemaV2()
    version = 2
  }

  writeStorageString(STORAGE_SCHEMA_VERSION_KEY, String(version))

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
