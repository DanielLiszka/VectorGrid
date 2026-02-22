import { defineStore } from 'pinia'
import {
  CONTROLS_COLLAPSED_KEY,
  THEME_PREF_KEY,
  WINDOW_EFFECT_KEY,
  WORKFLOW_ADVANCED_KEY,
} from '../legacy/core/storageKeys.js'
import { invokeCommand } from '../tauri/invoke'
import { useDataStore } from './dataStore'
import { useWorkflowStore } from './workflowStore'

const THEME_KEYS = new Set(['light', 'dark', 'system'])
const EFFECT_KEYS = new Set(['none', 'mica', 'acrylic', 'vibrancy'])

function normalizeTheme(value: unknown): string {
  const next = String(value ?? '').trim().toLowerCase()
  return THEME_KEYS.has(next) ? next : 'system'
}

function normalizeEffect(value: unknown): string {
  const next = String(value ?? '').trim().toLowerCase()
  return EFFECT_KEYS.has(next) ? next : 'none'
}

function resolveTheme(pref: string): string {
  if (pref === 'dark') return 'dark'
  if (pref === 'light') return 'light'
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return 'light'
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function setRootDataAttr(name: string, value: string): void {
  if (typeof document === 'undefined') return
  document.documentElement.dataset[name as keyof DOMStringMap] = value
}

function setLegacyFieldValue(id: string, value: string): void {
  if (typeof document === 'undefined') return
  const el = document.getElementById(id) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
  if (!el) return
  if (String(el.value || '') !== value) el.value = value
}

function setLegacyCheckboxValue(id: string, checked: boolean): void {
  if (typeof document === 'undefined') return
  const el = document.getElementById(id) as HTMLInputElement | null
  if (!el || String(el.type || '').toLowerCase() !== 'checkbox') return
  if (el.checked !== checked) el.checked = checked
}

function persistTheme(pref: string): void {
  try {
    localStorage.setItem(THEME_PREF_KEY, pref)
  } catch {}
}

function persistEffect(effect: string): void {
  try {
    if (effect === 'none') {
      localStorage.removeItem(WINDOW_EFFECT_KEY)
      return
    }
    localStorage.setItem(WINDOW_EFFECT_KEY, effect)
  } catch {}
}

function persistBoolFlag(key: string, enabled: boolean): void {
  try {
    if (enabled) localStorage.setItem(key, '1')
    else localStorage.removeItem(key)
  } catch {}
}

function applyThemeLocally(pref: string): void {
  const normalized = normalizeTheme(pref)
  setRootDataAttr('themePref', normalized)
  setRootDataAttr('theme', resolveTheme(normalized))
  persistTheme(normalized)
}

function applyEffectLocally(effect: string): void {
  const normalized = normalizeEffect(effect)
  setRootDataAttr('windowEffect', normalized)
  persistEffect(normalized)
  if (typeof document === 'undefined') return
  const dark = String(document.documentElement.dataset.theme || 'light').trim() === 'dark'
  void invokeCommand('set_window_effect', { effect: normalized === 'none' ? null : normalized, dark }).catch(() => undefined)
}

function applyWorkflowAdvancedLocally(enabled: boolean): void {
  setRootDataAttr('workflowAdvanced', enabled ? '1' : '')
  persistBoolFlag(WORKFLOW_ADVANCED_KEY, enabled)
}

function applyControlsCollapsedLocally(enabled: boolean): void {
  setRootDataAttr('controlsCollapsed', enabled ? '1' : '')
  persistBoolFlag(CONTROLS_COLLAPSED_KEY, enabled)
}

interface SettingsState {
  theme: string
  effect: string
  workflowAdvanced: boolean
  controlsCollapsed: boolean
  marketDataDir: string
  moveExisting: boolean
  marketDataDirHint: string
  canBrowseMarketDataDir: boolean
  canSetMarketDataDir: boolean
  canResetMarketDataDir: boolean
  canOpenMarketDataDir: boolean
  canOpenWorkflowData: boolean
  canOpenRunsDir: boolean
  canOpenReportsDir: boolean
  canOpenPolygonModal: boolean
  canGotoLibrary: boolean
  canGotoAggregate: boolean
  statusLine: string
  source: 'legacy-dom' | 'vue' | 'unknown'
  updatedAtUnixMs: number
}

export const useSettingsStore = defineStore('settings', {
  state: (): SettingsState => ({
    theme: 'system',
    effect: 'none',
    workflowAdvanced: false,
    controlsCollapsed: false,
    marketDataDir: '',
    moveExisting: true,
    marketDataDirHint: '—',
    canBrowseMarketDataDir: true,
    canSetMarketDataDir: true,
    canResetMarketDataDir: true,
    canOpenMarketDataDir: true,
    canOpenWorkflowData: true,
    canOpenRunsDir: true,
    canOpenReportsDir: true,
    canOpenPolygonModal: true,
    canGotoLibrary: true,
    canGotoAggregate: true,
    statusLine: 'Ready.',
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
    markVueUpdate(): void {
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    applyLegacySnapshot(snapshot: Partial<SettingsState>): void {
      this.theme = String(snapshot.theme ?? this.theme ?? 'system').trim() || 'system'
      this.effect = String(snapshot.effect ?? this.effect ?? 'none').trim() || 'none'
      this.workflowAdvanced = Boolean(snapshot.workflowAdvanced ?? this.workflowAdvanced)
      this.controlsCollapsed = Boolean(snapshot.controlsCollapsed ?? this.controlsCollapsed)
      this.marketDataDir = String(snapshot.marketDataDir ?? this.marketDataDir ?? '').trim()
      this.moveExisting = Boolean(snapshot.moveExisting ?? this.moveExisting)
      this.marketDataDirHint = String(snapshot.marketDataDirHint ?? this.marketDataDirHint ?? '—').trim() || '—'
      this.canBrowseMarketDataDir = Boolean(snapshot.canBrowseMarketDataDir ?? this.canBrowseMarketDataDir)
      this.canSetMarketDataDir = Boolean(snapshot.canSetMarketDataDir ?? this.canSetMarketDataDir)
      this.canResetMarketDataDir = Boolean(snapshot.canResetMarketDataDir ?? this.canResetMarketDataDir)
      this.canOpenMarketDataDir = Boolean(snapshot.canOpenMarketDataDir ?? this.canOpenMarketDataDir)
      this.canOpenWorkflowData = Boolean(snapshot.canOpenWorkflowData ?? this.canOpenWorkflowData)
      this.canOpenRunsDir = Boolean(snapshot.canOpenRunsDir ?? this.canOpenRunsDir)
      this.canOpenReportsDir = Boolean(snapshot.canOpenReportsDir ?? this.canOpenReportsDir)
      this.canOpenPolygonModal = Boolean(snapshot.canOpenPolygonModal ?? this.canOpenPolygonModal)
      this.canGotoLibrary = Boolean(snapshot.canGotoLibrary ?? this.canGotoLibrary)
      this.canGotoAggregate = Boolean(snapshot.canGotoAggregate ?? this.canGotoAggregate)
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.source = 'legacy-dom'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (key === 'settingsMarketDataDir') {
        const normalized = next.trim()
        if (this.marketDataDir !== normalized) this.marketDataDir = normalized
        setLegacyFieldValue('marketDataDir', normalized)
        this.markVueUpdate()
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (key === 'settingsThemeSelect') {
        const normalized = normalizeTheme(next)
        this.theme = normalized
        applyThemeLocally(normalized)
        // Theme affects the dark/light payload for native window effects.
        applyEffectLocally(this.effect)
        this.markVueUpdate()
      } else if (key === 'settingsEffectSelect') {
        const normalized = normalizeEffect(next)
        this.effect = normalized
        applyEffectLocally(normalized)
        this.markVueUpdate()
      }
    },

    setCheckboxField(field: string, checked: boolean): void {
      const key = String(field || '').trim()
      const next = !!checked
      if (key === 'settingsWorkflowAdvanced') {
        this.workflowAdvanced = next
        applyWorkflowAdvancedLocally(next)
        this.markVueUpdate()
      } else if (key === 'settingsControlsCollapsed') {
        this.controlsCollapsed = next
        applyControlsCollapsedLocally(next)
        this.markVueUpdate()
      } else if (key === 'settingsMoveExisting') {
        this.moveExisting = next
        setLegacyCheckboxValue('marketDataMoveExisting', next)
        this.markVueUpdate()
      }
    },

    browseMarketDataDir(): void {
      useDataStore().browseMarketDataDir()
    },

    setMarketDataDir(dir: string, moveExisting: boolean): void {
      useDataStore().setMarketDataDir(String(dir || '').trim(), !!moveExisting)
    },

    resetMarketDataDir(): void {
      useDataStore().resetMarketDataDir()
    },

    openMarketDataDir(): void {
      useDataStore().openMarketDataDir()
    },

    openWorkflowData(): void {
      useWorkflowStore().selectWorkspace('workflow', 'data')
    },

    openRunsDir(): void {
      if (!this.canOpenRunsDir) return
      void invokeCommand('open_runs_dir').catch(() => undefined)
    },

    openReportsDir(): void {
      if (!this.canOpenReportsDir) return
      void invokeCommand('open_reports_dir').catch(() => undefined)
    },

    openPolygonModal(): void {
      useDataStore().openPolygon()
    },

    gotoLibrary(): void {
      useWorkflowStore().selectWorkspace('library')
    },

    gotoAggregate(): void {
      useWorkflowStore().selectWorkspace('aggregate')
    },
  },
})
