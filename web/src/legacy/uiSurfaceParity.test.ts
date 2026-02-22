import fs from 'node:fs'
import path from 'node:path'
import { describe, expect, it } from 'vitest'

function readLegacyFile(rel: string): string {
  return fs.readFileSync(path.join(process.cwd(), 'src', 'legacy', rel), 'utf8')
}

function readComponentFile(rel: string): string {
  return fs.readFileSync(path.join(process.cwd(), 'src', 'components', rel), 'utf8')
}

function uniqueSorted(values: string[]): string[] {
  return Array.from(new Set(values)).sort()
}

function extractValues(text: string, regex: RegExp): string[] {
  const out: string[] = []
  for (const m of text.matchAll(regex)) {
    const value = String(m[1] ?? '').trim()
    if (value) out.push(value)
  }
  return out
}

function extractArrayKeys(text: string, constName: string): string[] {
  const section = text.match(new RegExp(`const\\s+${constName}\\s*=\\s*\\[([\\s\\S]*?)\\]\\s+as const`, 'm'))
  if (!section?.[1]) return []
  return extractValues(section[1], /key:\s*'([^']+)'/g)
}

describe('legacy UI surface parity', () => {
  const template = readLegacyFile('template.html')
  const legacyApp = readLegacyFile('app.js')
  const workspaceShell = readComponentFile('VueWorkspaceShell.vue')
  const globalShortcutBridge = readComponentFile('GlobalShortcutBridge.vue')

  it('preserves Vue-owned workspace/workflow/controls/results tab surfaces', () => {
    expect(uniqueSorted(extractArrayKeys(workspaceShell, 'workspaceItems'))).toEqual(
      ['aggregate', 'library', 'pro', 'settings', 'workflow'],
    )

    expect(uniqueSorted(extractArrayKeys(workspaceShell, 'workflowStepItems'))).toEqual(
      ['analyze', 'data', 'optimize', 'validate'],
    )

    expect(uniqueSorted(extractArrayKeys(workspaceShell, 'workflowItems'))).toEqual(
      ['analyze', 'data', 'history', 'optimize', 'settings', 'validate'],
    )

    expect(uniqueSorted(extractArrayKeys(workspaceShell, 'controlsItems'))).toEqual(
      ['advanced', 'data', 'distributed', 'opt', 'strategy'],
    )

    expect(uniqueSorted(extractArrayKeys(workspaceShell, 'resultsItems'))).toEqual(
      ['debug', 'drill', 'heatmap', 'history', 'meta', 'overview', 'pareto', 'top', 'validate'],
    )

    expect(template.includes('id="workspaceBar"')).toBe(false)
    expect(template.includes('id="workflowStepBar"')).toBe(false)
    expect(template.includes('id="workflowTabBar"')).toBe(false)
    expect(template.includes('id="controlsTabBar"')).toBe(false)
    expect(template.includes('id="resultsTabBar"')).toBe(false)
    expect(template.includes('id="aggregateTabBar"')).toBe(false)
    expect(template.includes('id="valCrossSelected"')).toBe(false)
    expect(template.includes('id="batchSelected"')).toBe(false)
    expect(template.includes('id="settingsThemeSelect"')).toBe(false)
    expect(template.includes('id="settingsEffectSelect"')).toBe(false)
    expect(template.includes('id="settingsMarketDataDir"')).toBe(false)
    expect(template.includes('id="settingsWorkspaceCard"')).toBe(false)
    expect(template.includes('id="btnSettingsOpenRunsDir"')).toBe(false)
    expect(template.includes('id="btnToggleControls"')).toBe(false)
    expect(template.includes('id="themeSelect"')).toBe(false)
    expect(template.includes('id="effectSelect"')).toBe(false)
    expect(template.includes('id="batchWrap"')).toBe(false)
    expect(template.includes('id="aggregateMetaSection"')).toBe(false)
    expect(template.includes('id="metaSavedWrap"')).toBe(false)
    expect(template.includes('id="metaCompareWrap"')).toBe(false)
    expect(template.includes('id="mainSplitter"')).toBe(false)
    expect(template.includes('id="controlsCard"')).toBe(false)
    expect(template.includes('id="resultsCard"')).toBe(false)
    expect(template.includes('id="aggregateSummaryGrid"')).toBe(false)
    expect(template.includes('id="controlsTabPanels"')).toBe(false)
    expect(template.includes('id="resultsTabPanels"')).toBe(false)
    expect(legacyApp.includes('resultsTabBar')).toBe(false)
  })

  it('preserves polygon modal host and removes legacy command palette host', () => {
    expect(template.includes('id="polygonModal"')).toBe(true)
    expect(template.includes('id="cmdPaletteModal"')).toBe(false)
    expect(template.includes('id="presetFile"')).toBe(false)
    expect(template.includes('id="quickBar"')).toBe(false)
    expect(template.includes('id="btnQaRun"')).toBe(false)
    expect(template.includes('id="quickBarHint"')).toBe(false)
    expect(template.includes('id="quickActions"')).toBe(false)
    expect(template.includes('id="btnOpenLatestReport"')).toBe(false)
    expect(template.includes('id="btnOpenLatestMetaReport"')).toBe(false)
    expect(template.includes('id="btnOpenReportsDir"')).toBe(false)
    expect(template.includes('id="btnOpenRunsDir"')).toBe(false)
    expect(template.includes('id="btnBrowseExportAll"')).toBe(false)
    expect(template.includes('id="btnBrowseCsv"')).toBe(false)
    expect(template.includes('id="btnPolygonOpen"')).toBe(false)
    expect(template.includes('id="btnPolygonClose"')).toBe(false)
    expect(template.includes('id="btnPolygonSaveKey"')).toBe(false)
    expect(template.includes('id="btnPolygonClearKey"')).toBe(false)
    expect(template.includes('id="btnPolygonDownloadLoad"')).toBe(false)
    expect(template.includes('id="btnLoad"')).toBe(false)
    expect(template.includes('id="btnClear"')).toBe(false)
    expect(template.includes('id="btnMoResetSearchSpace"')).toBe(false)
    expect(template.includes('id="btnOverviewEmptyPrimary"')).toBe(false)
    expect(template.includes('id="btnOverviewEmptySecondary"')).toBe(false)
    expect(template.includes('id="btnBrowseMarketDataDir"')).toBe(false)
    expect(template.includes('id="btnSetMarketDataDir"')).toBe(false)
    expect(template.includes('id="btnResetMarketDataDir"')).toBe(false)
    expect(template.includes('id="btnOpenMarketDataDir"')).toBe(false)
    expect(template.includes('id="btnApplyTimeframe"')).toBe(false)
    expect(template.includes('id="btnApplyPreset"')).toBe(false)
    expect(template.includes('id="btnPresetKernelSmaAlma"')).toBe(false)
    expect(template.includes('id="btnRefreshGpuStatus"')).toBe(false)
    expect(template.includes('id="btnGpuParityCheck"')).toBe(false)
    expect(template.includes('id="btnRun"')).toBe(false)
    expect(template.includes('id="btnCancel"')).toBe(false)
    expect(template.includes('id="btnValidate"')).toBe(false)
    expect(template.includes('id="btnCrossRefresh"')).toBe(false)
    expect(template.includes('id="btnCrossSelectAll"')).toBe(false)
    expect(template.includes('id="btnCrossSelectNone"')).toBe(false)
    expect(template.includes('id="btnCrossSelectMatch"')).toBe(false)
    expect(template.includes('id="btnExportValidationCsv"')).toBe(false)
    expect(template.includes('id="btnExportValidationJson"')).toBe(false)
    expect(template.includes('id="btnExportReportJson"')).toBe(false)
    expect(template.includes('id="btnExportReportXlsx"')).toBe(false)
    expect(template.includes('id="btnApplyHistory"')).toBe(false)
    expect(template.includes('id="btnExportPreset"')).toBe(false)
    expect(template.includes('id="btnImportPreset"')).toBe(false)
    expect(template.includes('id="btnClearHistory"')).toBe(false)
    expect(template.includes('id="btnSaveSavedRunTags"')).toBe(false)
    expect(template.includes('id="btnRefreshSavedRuns"')).toBe(false)
    expect(template.includes('id="btnLoadSavedRun"')).toBe(false)
    expect(template.includes('id="btnApplySavedRun"')).toBe(false)
    expect(template.includes('id="btnRerunSavedRun"')).toBe(false)
    expect(template.includes('id="btnOpenSavedRunReport"')).toBe(false)
    expect(template.includes('id="btnRevealSavedRun"')).toBe(false)
    expect(template.includes('id="btnDeleteSavedRun"')).toBe(false)
    expect(template.includes('id="btnCopyBest"')).toBe(false)
    expect(template.includes('id="btnExportCsv"')).toBe(false)
    expect(template.includes('id="btnExportJson"')).toBe(false)
    expect(template.includes('id="btnSetBaseline"')).toBe(false)
    expect(template.includes('id="btnClearBaseline"')).toBe(false)
    expect(template.includes('id="btnClearTopFilters"')).toBe(false)
    expect(template.includes('id="btnHeatmapExportPng"')).toBe(false)
    expect(template.includes('id="btnRunBatch"')).toBe(false)
    expect(template.includes('id="btnBatchStop"')).toBe(false)
    expect(template.includes('id="btnBatchRefresh"')).toBe(false)
    expect(template.includes('id="btnBatchSelectAll"')).toBe(false)
    expect(template.includes('id="btnBatchSelectNone"')).toBe(false)
    expect(template.includes('id="btnBatchRetryFailed"')).toBe(false)
    expect(template.includes('id="btnBatchRevealSummary"')).toBe(false)
    expect(template.includes('id="btnRunMeta"')).toBe(false)
    expect(template.includes('id="btnOpenMetaReport"')).toBe(false)
    expect(template.includes('id="btnRevealMetaFiles"')).toBe(false)
    expect(template.includes('id="btnMetaMlRefresh"')).toBe(false)
    expect(template.includes('id="btnMetaMlTrain"')).toBe(false)
    expect(template.includes('id="btnMetaMlApply"')).toBe(false)
    expect(template.includes('id="btnMetaMlDelete"')).toBe(false)
    expect(template.includes('id="btnRunMetaStage4"')).toBe(false)
    expect(template.includes('id="btnRevealMetaStage4"')).toBe(false)
    expect(template.includes('id="btnSaveMetaSavedTags"')).toBe(false)
    expect(template.includes('id="btnRefreshMetaSaved"')).toBe(false)
    expect(template.includes('id="btnLoadMetaSaved"')).toBe(false)
    expect(template.includes('id="btnOpenMetaSavedReport"')).toBe(false)
    expect(template.includes('id="btnRevealMetaSaved"')).toBe(false)
    expect(template.includes('id="btnDeleteMetaSaved"')).toBe(false)
    expect(template.includes('id="btnSaveStage4SavedTags"')).toBe(false)
    expect(template.includes('id="btnRefreshStage4Saved"')).toBe(false)
    expect(template.includes('id="btnLoadStage4Saved"')).toBe(false)
    expect(template.includes('id="btnRevealStage4Saved"')).toBe(false)
    expect(template.includes('id="btnDeleteStage4Saved"')).toBe(false)
    expect(template.includes('id="btnDistGenToken"')).toBe(false)
    expect(template.includes('id="btnDistStartWorker"')).toBe(false)
    expect(template.includes('id="btnDistStopWorker"')).toBe(false)
    expect(template.includes('id="btnDistWorkerStatus"')).toBe(false)
    expect(template.includes('id="btnDistFillFromPolygon"')).toBe(false)
    expect(template.includes('id="btnDistUseCurrentCsv"')).toBe(false)
    expect(template.includes('id="btnDistRunAssets"')).toBe(false)
    expect(template.includes('id="btnDistRunTiles"')).toBe(false)
    expect(template.includes('id="btnDistCancel"')).toBe(false)
    expect(template.includes('id="btnExecTestConnection"')).toBe(false)
    expect(template.includes('id="btnExecClearKeys"')).toBe(false)
    expect(template.includes('id="btnExecRunOnce"')).toBe(false)
    expect(template.includes('id="btnExecRefresh"')).toBe(false)
    expect(template.includes('id="btnExecLiveStart"')).toBe(false)
    expect(template.includes('id="btnExecLiveStop"')).toBe(false)
    expect(template.includes('id="btnExecRunnerStart"')).toBe(false)
    expect(template.includes('id="btnExecRunnerStop"')).toBe(false)
    expect(template.includes('id="btnToggleWorkflowAdvanced"')).toBe(false)
    expect(template.includes('id="workflowSubtitle"')).toBe(false)
    expect(template.includes('id="workspaceCtxDataset"')).toBe(false)
    expect(template.includes('id="workspaceCtxTimeframe"')).toBe(false)
    expect(template.includes('id="workspaceCtxStrategy"')).toBe(false)
    expect(template.includes('id="workspaceCtxBackend"')).toBe(false)
    expect(template.includes('id="workspaceCtxStatus"')).toBe(false)
    expect(template.includes('id="toastHost"')).toBe(false)
    expect(template.includes('id="runFooter"')).toBe(false)
    expect(template.includes('id="stabilityTable"')).toBe(false)
    expect(template.includes('id="tradeTable"')).toBe(false)
    expect(template.includes('id="execOrdersTable"')).toBe(false)
  })

  it('preserves required keyboard shortcuts in Vue global shortcut bridge', () => {
    expect(/mod && lower === 'k'/.test(globalShortcutBridge)).toBe(true) // Ctrl+K
    expect(/mod && key === '\\\\'/.test(globalShortcutBridge)).toBe(true) // Ctrl+\
    expect(/mod && lower === 'o' && !e\.shiftKey/.test(globalShortcutBridge)).toBe(true) // Ctrl+O
    expect(/mod && key === 'Enter'/.test(globalShortcutBridge)).toBe(true) // Ctrl+Enter
    expect(/mod && e\.shiftKey && lower === 'v'/.test(globalShortcutBridge)).toBe(true) // Ctrl+Shift+V
    expect(/mod && e\.shiftKey && lower === 'c'/.test(globalShortcutBridge)).toBe(true) // Ctrl+Shift+C
  })
})
