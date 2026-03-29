import { beforeEach, describe, expect, it } from 'vitest'
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
  THEME_PREF_KEY,
  WINDOW_EFFECT_KEY,
  WORKFLOW_ADVANCED_KEY,
  WORKFLOW_STATE_KEY,
  WORKFLOW_STEP_KEY,
  WORKFLOW_TAB_KEY,
  WORKSPACE_TAB_KEY,
} from './core/storageKeys'
import {
  LEGACY_STORAGE_KEYS,
  STORAGE_SCHEMA_VERSION_KEY,
  readStorageJson,
  readStorageString,
  runStorageMigrations,
  writeStorageJson,
} from './storage'

class MemoryStorage implements Storage {
  private readonly values = new Map<string, string>()

  get length(): number {
    return this.values.size
  }

  clear(): void {
    this.values.clear()
  }

  getItem(key: string): string | null {
    return this.values.has(key) ? this.values.get(key) ?? null : null
  }

  key(index: number): string | null {
    const keys = Array.from(this.values.keys())
    return index >= 0 && index < keys.length ? (keys[index] ?? null) : null
  }

  removeItem(key: string): void {
    this.values.delete(key)
  }

  setItem(key: string, value: string): void {
    this.values.set(key, String(value))
  }
}

describe('storage migrations and compatibility', () => {
  beforeEach(() => {
    const storage = new MemoryStorage()
    ;(globalThis as Record<string, unknown>).window = { localStorage: storage }
  })

  it('tracks all legacy storage keys for backward compatibility', () => {
    const expected = new Set([
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
    ])
    expect(new Set(LEGACY_STORAGE_KEYS)).toEqual(expected)
  })

  it('runs schema migration and normalizes bool-like key values', () => {
    const storage = (window as { localStorage: Storage }).localStorage
    storage.setItem(WORKFLOW_ADVANCED_KEY, 'true')
    storage.setItem(CONTROLS_COLLAPSED_KEY, 'false')
    expect(storage.getItem(STORAGE_SCHEMA_VERSION_KEY)).toBeNull()

    const version = runStorageMigrations()
    expect(version).toBe(2)
    expect(storage.getItem(STORAGE_SCHEMA_VERSION_KEY)).toBe('2')
    expect(storage.getItem(WORKFLOW_ADVANCED_KEY)).toBe('1')
    expect(storage.getItem(CONTROLS_COLLAPSED_KEY)).toBeNull()
  })

  it('migrates legacy vectorbt storage keys to vectorgrid keys', () => {
    const storage = (window as { localStorage: Storage }).localStorage
    storage.setItem(LEGACY_SETTINGS_KEY, '{"backend":"GpuKernel"}')
    storage.setItem(LEGACY_HISTORY_KEY, '[{"ts":"2026-03-13T00:00:00Z"}]')
    storage.setItem(LEGACY_CONTROLS_TAB_KEY, 'strategy')
    storage.setItem(LEGACY_RESULTS_TAB_KEY, 'history')
    storage.setItem(LEGACY_RECENT_CSV_KEY, 'C:/data/AAPL.csv')
    storage.setItem(LEGACY_LAST_BATCH_SUMMARY_KEY, 'C:/runs/batch.json')
    storage.setItem(LEGACY_EXEC_KEYS_STORAGE_KEY, '{"api_key":"k","api_secret":"s"}')

    const version = runStorageMigrations()
    expect(version).toBe(2)
    expect(readStorageString(SETTINGS_KEY, '')).toBe('{"backend":"GpuKernel"}')
    expect(readStorageString(HISTORY_KEY, '')).toBe('[{"ts":"2026-03-13T00:00:00Z"}]')
    expect(readStorageString(CONTROLS_TAB_KEY, '')).toBe('strategy')
    expect(readStorageString(RESULTS_TAB_KEY, '')).toBe('history')
    expect(readStorageString(RECENT_CSV_KEY, '')).toBe('C:/data/AAPL.csv')
    expect(readStorageString(LAST_BATCH_SUMMARY_KEY, '')).toBe('C:/runs/batch.json')
    expect(readStorageString(EXEC_KEYS_STORAGE_KEY, '')).toBe('{"api_key":"k","api_secret":"s"}')
    expect(storage.getItem(SETTINGS_KEY)).toBe('{"backend":"GpuKernel"}')
    expect(storage.getItem(HISTORY_KEY)).toBe('[{"ts":"2026-03-13T00:00:00Z"}]')
  })

  it('reads and writes JSON payloads safely', () => {
    const ok = writeStorageJson('vta_test_json', { alpha: 1, beta: 'ok' })
    expect(ok).toBe(true)
    expect(readStorageJson<{ alpha: number; beta: string }>('vta_test_json')).toEqual({ alpha: 1, beta: 'ok' })
  })
})
