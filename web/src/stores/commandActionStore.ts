import { defineStore } from 'pinia'
import { useAnalysisStore } from './analysisStore'
import { useDataStore } from './dataStore'
import { useOptimizeStore } from './optimizeStore'
import { resolveRuntimePinia } from './runtimePinia'
import { useSettingsStore } from './settingsStore'
import { useValidationStore } from './validationStore'
import { openPathWithRevealFallback } from '../utils/openPath'
import { useWorkflowArtifactsStore } from './workflowArtifactsStore'

function normalizedPath(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizedOptionalBoolean(value: unknown): boolean | null {
  if (typeof value === 'boolean') return value
  const text = normalizedPath(value).toLowerCase()
  if (!text) return null
  if (text === '1' || text === 'true' || text === 'yes' || text === 'on') return true
  if (text === '0' || text === 'false' || text === 'no' || text === 'off') return false
  return null
}

function rootRuntimePath(key: 'latestReportPath' | 'latestMetaReportPath'): string {
  if (typeof document === 'undefined') return ''
  return normalizedPath(document.documentElement.dataset[key])
}

function rootRuntimeOptionalBoolean(key: 'latestMetaReportExists'): boolean | null {
  if (typeof document === 'undefined') return null
  return normalizedOptionalBoolean(document.documentElement.dataset[key])
}

function runtimeRunning(): boolean {
  if (typeof document === 'undefined') return false
  return String(document.documentElement.dataset.running || '').trim() === '1'
}

function workflowArtifactsReportPath(): string {
  const pinia = resolveRuntimePinia()
  if (!pinia) return ''
  const artifactsStore = useWorkflowArtifactsStore(pinia)
  const reportPath = normalizedPath(artifactsStore.currentRun?.reportXlsxPath)
  const reportExists = normalizedOptionalBoolean(artifactsStore.currentRun?.reportXlsxExists)
  if (!reportPath) return ''
  return reportExists === false ? '' : reportPath
}

function resolvedLatestReportPath(state: { latestReportPath: string }): string {
  return normalizedPath(state.latestReportPath)
    || workflowArtifactsReportPath()
    || rootRuntimePath('latestReportPath')
}

function resolvedLatestMetaReportPath(state: {
  latestMetaReportPath: string
  latestMetaReportExists: boolean | null
}): string {
  const runtimePath = rootRuntimePath('latestMetaReportPath')
  if (runtimePath && rootRuntimeOptionalBoolean('latestMetaReportExists') !== false) {
    return runtimePath
  }
  const storePath = normalizedPath(state.latestMetaReportPath)
  if (!storePath) return ''
  return state.latestMetaReportExists === false ? '' : storePath
}

export const useCommandActionStore = defineStore('commandActions', {
  state: () => ({
    latestReportPath: rootRuntimePath('latestReportPath'),
    latestMetaReportPath: rootRuntimePath('latestMetaReportPath'),
    latestMetaReportExists: rootRuntimeOptionalBoolean('latestMetaReportExists'),
    source: 'unknown' as 'legacy-event' | 'unknown' | 'vue-artifacts',
    updatedAtUnixMs: 0,
  }),

  getters: {
    canOpenLatestReport: (state): boolean => resolvedLatestReportPath(state).length > 0,
    canOpenLatestMetaReport: (state): boolean => resolvedLatestMetaReportPath(state).length > 0,
  },

  actions: {
    applyLegacyQuickActionsSnapshot(snapshot: {
      latestReportPath?: unknown
      latestMetaReportPath?: unknown
      latestMetaReportExists?: unknown
    }): void {
      if (Object.prototype.hasOwnProperty.call(snapshot, 'latestReportPath')) {
        this.latestReportPath = normalizedPath(snapshot.latestReportPath)
      }
      if (Object.prototype.hasOwnProperty.call(snapshot, 'latestMetaReportPath')) {
        this.latestMetaReportPath = normalizedPath(snapshot.latestMetaReportPath)
      }
      if (Object.prototype.hasOwnProperty.call(snapshot, 'latestMetaReportExists')) {
        this.latestMetaReportExists = normalizedOptionalBoolean(snapshot.latestMetaReportExists)
      }
      this.source = 'legacy-event'
      this.updatedAtUnixMs = Date.now()
    },

    applyLatestMetaReportState(path: unknown, exists?: unknown): void {
      this.latestMetaReportPath = normalizedPath(path)
      this.latestMetaReportExists = normalizedOptionalBoolean(exists)
      this.source = 'vue-artifacts'
      this.updatedAtUnixMs = Date.now()
    },

    runOptimization(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useOptimizeStore(pinia).runOptimization()
    },

    cancelOptimization(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useOptimizeStore(pinia).cancelOptimization()
    },

    validateOptimization(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useOptimizeStore(pinia).runValidation()
    },

    browseCsv(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useDataStore(pinia).browseCsv()
    },

    loadData(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useDataStore(pinia).loadCsv()
    },

    openPolygon(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useDataStore(pinia).openPolygon()
    },

    exportTopCsv(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useAnalysisStore(pinia).exportTopCsv()
    },

    exportValidationShortlistCsv(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useValidationStore(pinia).exportShortlistCsv()
    },

    exportTopJson(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useAnalysisStore(pinia).exportTopJson()
    },

    openLatestReport(): void {
      if (runtimeRunning()) return
      const path = resolvedLatestReportPath(this)
      if (path === workflowArtifactsReportPath()) this.source = 'vue-artifacts'
      void openPathWithRevealFallback(path).catch(() => undefined)
    },

    openLatestMetaReport(): void {
      if (runtimeRunning()) return
      void openPathWithRevealFallback(resolvedLatestMetaReportPath(this)).catch(() => undefined)
    },

    toggleControls(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      const settingsStore = useSettingsStore(pinia)
      settingsStore.setCheckboxField('settingsControlsCollapsed', !settingsStore.controlsCollapsed)
    },

    clearDrillTradeSelection(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useAnalysisStore(pinia).drillClearTradeSelection()
    },

    drillPrevTrade(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useAnalysisStore(pinia).drillPrevTrade()
    },

    drillNextTrade(): void {
      const pinia = resolveRuntimePinia()
      if (!pinia) return
      useAnalysisStore(pinia).drillNextTrade()
    },
  },
})
