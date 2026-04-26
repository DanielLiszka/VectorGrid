export const ANALYSIS_DOCK_OPEN_STORAGE_KEY = 'VTA_ANALYSIS_DOCK_OPEN_V1'
export const OPTIMIZE_DOCK_OPEN_STORAGE_KEY = 'VTA_OPTIMIZE_DOCK_OPEN_V1'

interface ShellDockState {
  mode?: unknown
  workspace?: unknown
  workflow?: unknown
  controlsTab?: unknown
  resultsTab?: unknown
}

function normalize(value: unknown): string {
  return String(value ?? '').trim().toLowerCase()
}

export function isAnalysisDockVisible(state: ShellDockState): boolean {
  const workspace = normalize(state.workspace)
  const workflow = normalize(state.workflow)
  const mode = normalize(state.mode)
  if (workspace === 'aggregate') return true
  if (mode === 'ml-signal') return false
  return workflow === 'optimize' || workflow === 'validate' || workflow === 'analyze'
}

export function isHistoryDockVisible(state: ShellDockState): boolean {
  const workspace = normalize(state.workspace)
  const workflow = normalize(state.workflow)
  return workspace === 'library' || workflow === 'history'
}

export function isOptimizeDockVisible(state: ShellDockState): boolean {
  const workspace = normalize(state.workspace)
  const workflow = normalize(state.workflow)
  const controlsTab = normalize(state.controlsTab)
  const mode = normalize(state.mode)
  return workspace === 'workflow' && workflow === 'optimize' && controlsTab === 'opt' && mode !== 'ml-signal'
}

export function isValidationDockVisible(state: ShellDockState): boolean {
  const workspace = normalize(state.workspace)
  const workflow = normalize(state.workflow)
  const resultsTab = normalize(state.resultsTab)
  const mode = normalize(state.mode)
  return workspace === 'workflow' && workflow !== 'validate' && resultsTab === 'validate' && mode !== 'ml-signal'
}
