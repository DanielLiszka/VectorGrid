import { defineStore } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { useAnalysisStore } from './analysisStore'
import { useDataStore } from './dataStore'
import { useOptimizeStore } from './optimizeStore'
import { resolveRuntimePinia } from './runtimePinia'
import { useSettingsStore } from './settingsStore'
import { useValidationStore } from './validationStore'
import { useWorkflowArtifactsStore } from './workflowArtifactsStore'

function normalizedPath(value: unknown): string {
  return String(value ?? '').trim()
}

function rootRuntimePath(key: 'latestReportPath' | 'latestMetaReportPath'): string {
  if (typeof document === 'undefined') return ''
  return normalizedPath(document.documentElement.dataset[key])
}

function runtimeRunning(): boolean {
  if (typeof document === 'undefined') return false
  return String(document.documentElement.dataset.running || '').trim() === '1'
}

function workflowArtifactsReportPath(): string {
  const pinia = resolveRuntimePinia()
  if (!pinia) return ''
  const artifactsStore = useWorkflowArtifactsStore(pinia)
  return normalizedPath(artifactsStore.currentRun?.reportXlsxPath)
}

function resolvedLatestReportPath(state: { latestReportPath: string }): string {
  return normalizedPath(state.latestReportPath)
    || workflowArtifactsReportPath()
    || rootRuntimePath('latestReportPath')
}

function resolvedLatestMetaReportPath(state: { latestMetaReportPath: string }): string {
  return normalizedPath(state.latestMetaReportPath)
    || rootRuntimePath('latestMetaReportPath')
}

async function openPathWithFallback(path: string): Promise<void> {
  const nextPath = String(path || '').trim()
  if (!nextPath) return
  try {
    await invokeCommand('open_path', { path: nextPath })
  } catch {
    await invokeCommand('reveal_path', { path: nextPath }).catch(() => undefined)
  }
}

export const useCommandActionStore = defineStore('commandActions', {
  state: () => ({
    latestReportPath: rootRuntimePath('latestReportPath'),
    latestMetaReportPath: rootRuntimePath('latestMetaReportPath'),
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
    }): void {
      if (Object.prototype.hasOwnProperty.call(snapshot, 'latestReportPath')) {
        this.latestReportPath = normalizedPath(snapshot.latestReportPath)
      }
      if (Object.prototype.hasOwnProperty.call(snapshot, 'latestMetaReportPath')) {
        this.latestMetaReportPath = normalizedPath(snapshot.latestMetaReportPath)
      }
      this.source = 'legacy-event'
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
      void openPathWithFallback(path)
    },

    openLatestMetaReport(): void {
      if (runtimeRunning()) return
      void openPathWithFallback(resolvedLatestMetaReportPath(this))
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
