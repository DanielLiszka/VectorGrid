import { defineStore, getActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { useAnalysisStore } from './analysisStore'
import { useDataStore } from './dataStore'
import { useOptimizeStore } from './optimizeStore'
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
  const pinia = getActivePinia()
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
      useOptimizeStore().runOptimization()
    },

    cancelOptimization(): void {
      useOptimizeStore().cancelOptimization()
    },

    validateOptimization(): void {
      useOptimizeStore().runValidation()
    },

    browseCsv(): void {
      useDataStore().browseCsv()
    },

    loadData(): void {
      useDataStore().loadCsv()
    },

    openPolygon(): void {
      useDataStore().openPolygon()
    },

    exportTopCsv(): void {
      useAnalysisStore().exportTopCsv()
    },

    exportValidationShortlistCsv(): void {
      useValidationStore().exportShortlistCsv()
    },

    exportTopJson(): void {
      useAnalysisStore().exportTopJson()
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
      const settingsStore = useSettingsStore()
      settingsStore.setCheckboxField('settingsControlsCollapsed', !settingsStore.controlsCollapsed)
    },

    clearDrillTradeSelection(): void {
      useAnalysisStore().drillClearTradeSelection()
    },

    drillPrevTrade(): void {
      useAnalysisStore().drillPrevTrade()
    },

    drillNextTrade(): void {
      useAnalysisStore().drillNextTrade()
    },
  },
})
