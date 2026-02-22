import { beforeEach, describe, expect, it } from 'vitest'
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
import {
  LEGACY_STORAGE_KEYS,
  STORAGE_SCHEMA_VERSION_KEY,
  readStorageJson,
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
    ])
    expect(new Set(LEGACY_STORAGE_KEYS)).toEqual(expected)
  })

  it('runs schema migration and normalizes bool-like key values', () => {
    const storage = (window as { localStorage: Storage }).localStorage
    storage.setItem(WORKFLOW_ADVANCED_KEY, 'true')
    storage.setItem(CONTROLS_COLLAPSED_KEY, 'false')
    expect(storage.getItem(STORAGE_SCHEMA_VERSION_KEY)).toBeNull()

    const version = runStorageMigrations()
    expect(version).toBe(1)
    expect(storage.getItem(STORAGE_SCHEMA_VERSION_KEY)).toBe('1')
    expect(storage.getItem(WORKFLOW_ADVANCED_KEY)).toBe('1')
    expect(storage.getItem(CONTROLS_COLLAPSED_KEY)).toBeNull()
  })

  it('reads and writes JSON payloads safely', () => {
    const ok = writeStorageJson('vta_test_json', { alpha: 1, beta: 'ok' })
    expect(ok).toBe(true)
    expect(readStorageJson<{ alpha: number; beta: string }>('vta_test_json')).toEqual({ alpha: 1, beta: 'ok' })
  })
})
