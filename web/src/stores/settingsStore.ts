import { defineStore } from 'pinia'
import {
  CONTROLS_COLLAPSED_KEY,
  DESKTOP_NOTIFICATION_MODE_KEY,
  DENSITY_MODE_KEY,
  HELP_MODE_KEY,
  THEME_PREF_KEY,
  WINDOW_EFFECT_KEY,
  WORKSTATION_PERSPECTIVES_KEY,
  WORKFLOW_ADVANCED_KEY,
} from '../utils/core/storageKeys'
import {
  openReportsDir as openReportsDirService,
  openRunsDir as openRunsDirService,
  setWindowEffect,
  type WindowEffect,
} from '../services/settingsService'
import { useDataStore } from './dataStore'
import { resolveRuntimePinia } from './runtimePinia'
import { useWorkflowStore } from './workflowStore'
import { readStorageString, removeStorageKey, writeStorageString } from '../utils/storage'

const THEME_KEYS = new Set(['light', 'dark', 'system'])
const EFFECT_KEYS = new Set(['none', 'mica', 'acrylic', 'vibrancy'])
const HELP_MODE_KEYS = new Set(['full', 'concise', 'off'])
const DENSITY_MODE_KEYS = new Set(['comfortable', 'standard', 'dense'])
const DESKTOP_NOTIFICATION_MODE_KEYS = new Set(['off', 'failures', 'important'])

export type HelpMode = 'full' | 'concise' | 'off'
export type DensityMode = 'comfortable' | 'standard' | 'dense'
export type DesktopNotificationMode = 'off' | 'failures' | 'important'

export interface WorkstationPerspectiveSnapshot {
  mode: string
  workspace: string
  workflow: string
  workflowStep: string
  controlsTab: string
  resultsTab: string
  aggregateTab: string
  splitFraction: number
  densityMode: DensityMode
}

export interface WorkstationPerspective {
  id: string
  name: string
  builtIn: boolean
  snapshot: WorkstationPerspectiveSnapshot
}

function normalizeTheme(value: unknown): string {
  const next = String(value ?? '').trim().toLowerCase()
  return THEME_KEYS.has(next) ? next : 'system'
}

function normalizeEffect(value: unknown): string {
  const next = String(value ?? '').trim().toLowerCase()
  return EFFECT_KEYS.has(next) ? next : 'none'
}

function normalizeHelpMode(value: unknown): HelpMode {
  const next = String(value ?? '').trim().toLowerCase()
  if (HELP_MODE_KEYS.has(next)) return next as HelpMode
  return 'full'
}

function normalizeDensityMode(value: unknown): DensityMode {
  const next = String(value ?? '').trim().toLowerCase()
  if (DENSITY_MODE_KEYS.has(next)) return next as DensityMode
  return 'standard'
}

function normalizeDesktopNotificationMode(value: unknown): DesktopNotificationMode {
  const next = String(value ?? '').trim().toLowerCase()
  if (DESKTOP_NOTIFICATION_MODE_KEYS.has(next)) return next as DesktopNotificationMode
  return 'off'
}

function normalizeSplitFraction(value: unknown): number {
  const next = Number(value)
  if (!Number.isFinite(next)) return 0.34
  return Math.min(0.9, Math.max(0.1, next))
}

function resolveTheme(pref: string): string {
  if (pref === 'dark') return 'dark'
  if (pref === 'light') return 'light'
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return 'light'
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function setRootDataAttr(name: string, value: string): void {
  if (typeof document === 'undefined') return
  if (value) document.documentElement.dataset[name as keyof DOMStringMap] = value
  else delete document.documentElement.dataset[name as keyof DOMStringMap]
}

function persistTheme(pref: string): void {
  writeStorageString(THEME_PREF_KEY, pref)
}

function persistEffect(effect: string): void {
  if (effect === 'none') {
    removeStorageKey(WINDOW_EFFECT_KEY)
    return
  }
  writeStorageString(WINDOW_EFFECT_KEY, effect)
}

function persistHelpMode(mode: HelpMode): void {
  if (mode === 'full') {
    removeStorageKey(HELP_MODE_KEY)
    return
  }
  writeStorageString(HELP_MODE_KEY, mode)
}

function persistBoolFlag(key: string, enabled: boolean): void {
  if (enabled) writeStorageString(key, '1')
  else removeStorageKey(key)
}

function persistDensityMode(mode: DensityMode): void {
  if (mode === 'standard') {
    removeStorageKey(DENSITY_MODE_KEY)
    return
  }
  writeStorageString(DENSITY_MODE_KEY, mode)
}

function persistDesktopNotificationMode(mode: DesktopNotificationMode): void {
  if (mode === 'off') {
    removeStorageKey(DESKTOP_NOTIFICATION_MODE_KEY)
    return
  }
  writeStorageString(DESKTOP_NOTIFICATION_MODE_KEY, mode)
}

function persistPerspectives(perspectives: WorkstationPerspective[]): void {
  writeStorageString(WORKSTATION_PERSPECTIVES_KEY, JSON.stringify(perspectives))
}

function applyDensityModeLocally(mode: DensityMode): void {
  const normalized = normalizeDensityMode(mode)
  setRootDataAttr('density', normalized)
  persistDensityMode(normalized)
}

function applyThemeLocally(pref: string): void {
  const normalized = normalizeTheme(pref)
  setRootDataAttr('themePref', normalized)
  setRootDataAttr('theme', resolveTheme(normalized))
  persistTheme(normalized)
}

function applyEffectLocally(effect: string): void {
  const normalized = normalizeEffect(effect) as WindowEffect
  setRootDataAttr('windowEffect', normalized)
  persistEffect(normalized)
  if (typeof document === 'undefined') return
  const dark = String(document.documentElement.dataset.theme || 'light').trim() === 'dark'
  void setWindowEffect(normalized, dark).catch(() => undefined)
}

function applyWorkflowAdvancedLocally(enabled: boolean): void {
  setRootDataAttr('workflowAdvanced', enabled ? '1' : '')
  persistBoolFlag(WORKFLOW_ADVANCED_KEY, enabled)
}

function applyControlsCollapsedLocally(enabled: boolean): void {
  setRootDataAttr('controlsCollapsed', enabled ? '1' : '')
  persistBoolFlag(CONTROLS_COLLAPSED_KEY, enabled)
}

function applyHelpModeLocally(mode: HelpMode): void {
  const normalized = normalizeHelpMode(mode)
  setRootDataAttr('helpMode', normalized)
  persistHelpMode(normalized)
}

const BUILTIN_PERSPECTIVES: WorkstationPerspective[] = [
  {
    id: 'builtin_optimize',
    name: 'Optimize',
    builtIn: true,
    snapshot: {
      mode: 'conventional',
      workspace: 'workflow',
      workflow: 'optimize',
      workflowStep: 'optimize',
      controlsTab: 'opt',
      resultsTab: 'overview',
      aggregateTab: 'meta',
      splitFraction: 0.34,
      densityMode: 'standard',
    },
  },
  {
    id: 'builtin_validate',
    name: 'Validate',
    builtIn: true,
    snapshot: {
      mode: 'conventional',
      workspace: 'workflow',
      workflow: 'validate',
      workflowStep: 'validate',
      controlsTab: 'opt',
      resultsTab: 'validate',
      aggregateTab: 'meta',
      splitFraction: 0.34,
      densityMode: 'standard',
    },
  },
  {
    id: 'builtin_top_results_triage',
    name: 'Top Results Triage',
    builtIn: true,
    snapshot: {
      mode: 'conventional',
      workspace: 'workflow',
      workflow: 'analyze',
      workflowStep: 'analyze',
      controlsTab: 'opt',
      resultsTab: 'top',
      aggregateTab: 'meta',
      splitFraction: 0.34,
      densityMode: 'dense',
    },
  },
  {
    id: 'builtin_execution_review',
    name: 'Execution Review',
    builtIn: true,
    snapshot: {
      mode: 'conventional',
      workspace: 'settings',
      workflow: 'data',
      workflowStep: 'data',
      controlsTab: 'data',
      resultsTab: 'overview',
      aggregateTab: 'meta',
      splitFraction: 0.34,
      densityMode: 'standard',
    },
  },
  {
    id: 'builtin_run_library',
    name: 'Run Library',
    builtIn: true,
    snapshot: {
      mode: 'conventional',
      workspace: 'library',
      workflow: 'history',
      workflowStep: 'data',
      controlsTab: 'data',
      resultsTab: 'history',
      aggregateTab: 'meta',
      splitFraction: 0.34,
      densityMode: 'dense',
    },
  },
]

function normalizePerspectiveSnapshot(value: unknown): WorkstationPerspectiveSnapshot | null {
  if (!value || typeof value !== 'object') return null
  const row = value as Record<string, unknown>
  return {
    mode: String(row.mode || '').trim() || 'conventional',
    workspace: String(row.workspace || '').trim() || 'workflow',
    workflow: String(row.workflow || '').trim() || 'data',
    workflowStep: String(row.workflowStep || '').trim() || 'data',
    controlsTab: String(row.controlsTab || '').trim() || 'data',
    resultsTab: String(row.resultsTab || '').trim() || 'overview',
    aggregateTab: String(row.aggregateTab || '').trim() || 'meta',
    splitFraction: normalizeSplitFraction(row.splitFraction),
    densityMode: normalizeDensityMode(row.densityMode),
  }
}

function readStoredPerspectives(): WorkstationPerspective[] {
  const byId = new Map<string, WorkstationPerspective>()
  for (const entry of BUILTIN_PERSPECTIVES) byId.set(entry.id, { ...entry, snapshot: { ...entry.snapshot } })
  try {
    const raw = readStorageString(WORKSTATION_PERSPECTIVES_KEY, '')
    if (!raw) return Array.from(byId.values())
    const parsed = JSON.parse(raw) as unknown
    if (!Array.isArray(parsed)) return Array.from(byId.values())
    for (const item of parsed) {
      if (!item || typeof item !== 'object') continue
      const row = item as Record<string, unknown>
      const id = String(row.id || '').trim()
      const name = String(row.name || '').trim()
      const snapshot = normalizePerspectiveSnapshot(row.snapshot)
      if (!id || !name || !snapshot) continue
      const builtin = Boolean(row.builtIn)
      if (byId.has(id)) {
        const base = byId.get(id)
        if (!base) continue
        byId.set(id, {
          id,
          name: builtin ? base.name : name,
          builtIn: true,
          snapshot,
        })
        continue
      }
      byId.set(id, {
        id,
        name,
        builtIn: false,
        snapshot,
      })
    }
  } catch {}
  return Array.from(byId.values())
}

function runtimeCanMutateSettings(): boolean {
  if (typeof document === 'undefined') return true
  const root = document.documentElement as HTMLElement | undefined
  if (!root || !root.dataset) return true
  return String(root.dataset.running || '').trim() !== '1'
}

interface SettingsState {
  theme: string
  effect: string
  helpMode: HelpMode
  densityMode: DensityMode
  desktopNotificationMode: DesktopNotificationMode
  workflowAdvanced: boolean
  controlsCollapsed: boolean
  perspectives: WorkstationPerspective[]
  selectedPerspectiveId: string
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
  source: 'vue' | 'unknown'
  updatedAtUnixMs: number
}

export const useSettingsStore = defineStore('settings', {
  state: (): SettingsState => ({
    theme: 'system',
    effect: 'none',
    helpMode: 'full',
    densityMode: normalizeDensityMode(readStorageString(DENSITY_MODE_KEY, '')),
    desktopNotificationMode: normalizeDesktopNotificationMode(readStorageString(DESKTOP_NOTIFICATION_MODE_KEY, '')),
    workflowAdvanced: false,
    controlsCollapsed: false,
    perspectives: readStoredPerspectives(),
    selectedPerspectiveId: 'builtin_optimize',
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
      this.helpMode = normalizeHelpMode(snapshot.helpMode ?? this.helpMode ?? 'full')
      this.densityMode = normalizeDensityMode(snapshot.densityMode ?? this.densityMode ?? 'standard')
      this.desktopNotificationMode = normalizeDesktopNotificationMode(
        snapshot.desktopNotificationMode ?? this.desktopNotificationMode ?? 'off',
      )
      this.workflowAdvanced = Boolean(snapshot.workflowAdvanced ?? this.workflowAdvanced)
      this.controlsCollapsed = Boolean(snapshot.controlsCollapsed ?? this.controlsCollapsed)
      this.perspectives = Array.isArray(snapshot.perspectives) ? snapshot.perspectives : this.perspectives
      this.selectedPerspectiveId = String(snapshot.selectedPerspectiveId ?? this.selectedPerspectiveId ?? 'builtin_optimize').trim() || 'builtin_optimize'
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
      applyDensityModeLocally(this.densityMode)
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (key === 'settingsMarketDataDir') {
        const normalized = next.trim()
        if (this.marketDataDir !== normalized) this.marketDataDir = normalized
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
      } else if (key === 'settingsHelpMode') {
        const normalized = normalizeHelpMode(next)
        this.helpMode = normalized
        applyHelpModeLocally(normalized)
        this.markVueUpdate()
      } else if (key === 'settingsDensityMode') {
        const normalized = normalizeDensityMode(next)
        this.densityMode = normalized
        applyDensityModeLocally(normalized)
        this.markVueUpdate()
      } else if (key === 'settingsDesktopNotifications') {
        const normalized = normalizeDesktopNotificationMode(next)
        this.desktopNotificationMode = normalized
        persistDesktopNotificationMode(normalized)
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
        this.markVueUpdate()
      }
    },

    selectPerspective(id: string): void {
      const next = String(id || '').trim()
      if (!next) return
      if (!this.perspectives.some((entry) => entry.id === next)) return
      this.selectedPerspectiveId = next
      this.markVueUpdate()
    },

    createPerspective(name: string): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      const workflowStore = useWorkflowStore(pinia)
      const trimmedName = String(name || '').trim() || `Perspective ${this.perspectives.length + 1}`
      const id = `perspective_${Date.now().toString(36)}`
      const entry: WorkstationPerspective = {
        id,
        name: trimmedName,
        builtIn: false,
        snapshot: {
          mode: workflowStore.mode,
          workspace: workflowStore.workspace,
          workflow: workflowStore.workflow,
          workflowStep: workflowStore.workflowStep,
          controlsTab: workflowStore.controlsTab,
          resultsTab: workflowStore.resultsTab,
          aggregateTab: workflowStore.aggregateTab,
          splitFraction: normalizeSplitFraction(workflowStore.splitFraction),
          densityMode: this.densityMode,
        },
      }
      this.perspectives = [...this.perspectives, entry]
      this.selectedPerspectiveId = id
      persistPerspectives(this.perspectives)
      this.statusLine = `Saved perspective: ${trimmedName}.`
      this.markVueUpdate()
    },

    overwritePerspective(id: string): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      const workflowStore = useWorkflowStore(pinia)
      const targetId = String(id || '').trim()
      if (!targetId) return
      const existing = this.perspectives.find((entry) => entry.id === targetId)
      if (!existing) return
      this.perspectives = this.perspectives.map((entry) =>
        entry.id !== targetId
          ? entry
          : {
              ...entry,
              snapshot: {
                mode: workflowStore.mode,
                workspace: workflowStore.workspace,
                workflow: workflowStore.workflow,
                workflowStep: workflowStore.workflowStep,
                controlsTab: workflowStore.controlsTab,
                resultsTab: workflowStore.resultsTab,
                aggregateTab: workflowStore.aggregateTab,
                splitFraction: normalizeSplitFraction(workflowStore.splitFraction),
                densityMode: this.densityMode,
              },
            },
      )
      persistPerspectives(this.perspectives)
      this.selectedPerspectiveId = targetId
      this.statusLine = `Updated perspective: ${existing.name}.`
      this.markVueUpdate()
    },

    applyPerspective(id: string): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      const targetId = String(id || '').trim()
      const entry = this.perspectives.find((item) => item.id === targetId)
      if (!entry) return
      const workflowStore = useWorkflowStore(pinia)
      this.selectedPerspectiveId = targetId
      this.densityMode = normalizeDensityMode(entry.snapshot.densityMode)
      applyDensityModeLocally(this.densityMode)
      workflowStore.applyLegacySnapshot({
        mode: entry.snapshot.mode === 'ml-signal' ? 'ml-signal' : 'conventional',
        workspace: entry.snapshot.workspace,
        workflow: entry.snapshot.workflow,
        workflowStep: entry.snapshot.workflowStep,
        controlsTab: entry.snapshot.controlsTab,
        resultsTab: entry.snapshot.resultsTab,
        aggregateTab: entry.snapshot.aggregateTab,
        splitFraction: entry.snapshot.splitFraction,
      })
      workflowStore.commitState('vue', true)
      this.statusLine = `Restored perspective: ${entry.name}.`
      this.markVueUpdate()
    },

    renamePerspective(id: string, name: string): void {
      const targetId = String(id || '').trim()
      const nextName = String(name || '').trim()
      if (!targetId || !nextName) return
      const target = this.perspectives.find((entry) => entry.id === targetId)
      if (!target || target.builtIn) return
      this.perspectives = this.perspectives.map((entry) =>
        entry.id === targetId ? { ...entry, name: nextName } : entry,
      )
      persistPerspectives(this.perspectives)
      this.selectedPerspectiveId = targetId
      this.statusLine = `Renamed perspective to ${nextName}.`
      this.markVueUpdate()
    },

    duplicatePerspective(id: string): void {
      const targetId = String(id || '').trim()
      const existing = this.perspectives.find((entry) => entry.id === targetId)
      if (!existing) return
      const copyName = `${existing.name} Copy`
      const duplicate: WorkstationPerspective = {
        id: `perspective_${Date.now().toString(36)}`,
        name: copyName,
        builtIn: false,
        snapshot: {
          ...existing.snapshot,
        },
      }
      this.perspectives = [...this.perspectives, duplicate]
      persistPerspectives(this.perspectives)
      this.selectedPerspectiveId = duplicate.id
      this.statusLine = `Duplicated perspective: ${existing.name}.`
      this.markVueUpdate()
    },

    deletePerspective(id: string): void {
      const targetId = String(id || '').trim()
      const existing = this.perspectives.find((entry) => entry.id === targetId)
      if (!existing || existing.builtIn) return
      this.perspectives = this.perspectives.filter((entry) => entry.id !== targetId)
      persistPerspectives(this.perspectives)
      this.selectedPerspectiveId = this.perspectives[0]?.id || 'builtin_optimize'
      this.statusLine = `Deleted perspective: ${existing.name}.`
      this.markVueUpdate()
    },

    syncFromDataStore(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      const dataStore = useDataStore(pinia)
      const canMutate = runtimeCanMutateSettings()

      this.marketDataDir = String(dataStore.marketDataDir || '').trim()
      this.moveExisting = !!dataStore.marketDataMoveExisting
      this.marketDataDirHint = String(dataStore.marketDataDirHint || this.marketDataDirHint || '—').trim() || '—'

      this.canBrowseMarketDataDir = canMutate && !!dataStore.canBrowseMarketDataDir
      this.canSetMarketDataDir = canMutate && !!dataStore.canSetMarketDataDir
      this.canResetMarketDataDir = canMutate && !!dataStore.canResetMarketDataDir
      this.canOpenMarketDataDir = canMutate && !!dataStore.canOpenMarketDataDir
      this.canOpenWorkflowData = canMutate
      this.canOpenRunsDir = canMutate
      this.canOpenReportsDir = canMutate
      this.canOpenPolygonModal = canMutate
      this.canGotoLibrary = true
      this.canGotoAggregate = true

      this.markVueUpdate()
    },

    browseMarketDataDir(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useDataStore(pinia).browseMarketDataDir()
    },

    setMarketDataDir(dir: string, moveExisting: boolean): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useDataStore(pinia).setMarketDataDir(String(dir || '').trim(), !!moveExisting)
    },

    resetMarketDataDir(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useDataStore(pinia).resetMarketDataDir()
    },

    openMarketDataDir(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useDataStore(pinia).openMarketDataDir()
    },

    openWorkflowData(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useWorkflowStore(pinia).selectWorkspace('workflow', 'data')
    },

    openRunsDir(): void {
      if (!this.canOpenRunsDir) return
      void openRunsDirService().catch(() => undefined)
    },

    openReportsDir(): void {
      if (!this.canOpenReportsDir) return
      void openReportsDirService().catch(() => undefined)
    },

    openPolygonModal(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useDataStore(pinia).openPolygon()
    },

    gotoLibrary(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useWorkflowStore(pinia).selectWorkspace('library')
    },

    gotoAggregate(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useWorkflowStore(pinia).selectWorkspace('aggregate')
    },
  },
})

