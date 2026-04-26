import { normalizeResultsTab, normalizeWorkflowStep, normalizeWorkspaceTab } from './core/workflowModel'

export type WorkbenchInspectorMode = 'selection' | 'validation' | 'history' | 'tasks' | 'diagnostics'
export type WorkbenchInspectorTone = 'neutral' | 'accent' | 'success' | 'warning' | 'danger'

export interface WorkbenchInspectorInput {
  workspace?: unknown
  workflowStep?: unknown
  resultsTab?: unknown
  requestedMode?: WorkbenchInspectorMode | ''
  datasetLabel?: string
  strategyLabel?: string
  backendLabel?: string
  selectedCandidateTitle?: string
  selectedCandidateSummary?: string
  status?: string
  validationStage?: string
  validationSummary?: string
  validationOutputPreview?: string
  validationCrossSummary?: string
  validationTableRows?: number
  validationGatesLabel?: string
  validationCompatibilitySummary?: string
  validationCompatibilityDetail?: string
  validationStatus?: string
  historySummary?: string
  historySessionSummary?: string
  historySavedSummary?: string
  historyCompareSummary?: string
  historyDetailTitle?: string
  historyDetailMeta?: string
  historyStatus?: string
  historyCanApplySession?: boolean
  historyCanRefreshSaved?: boolean
  historyCanLoadSaved?: boolean
  historyCanApplySaved?: boolean
  historyCanClearSession?: boolean
  historyCanOpenRunsDir?: boolean
  activeTaskCount?: number
  taskSummary?: string
  diagnosticsSummary?: string
}

export interface WorkbenchInspectorModeItem {
  id: WorkbenchInspectorMode
  label: string
  tone: WorkbenchInspectorTone
  current: boolean
  actionId: string
}

export interface WorkbenchInspectorSection {
  id: string
  title: string
  value: string
  detail: string
  tone: WorkbenchInspectorTone
}

export interface WorkbenchInspectorAction {
  id: string
  label: string
  actionId: string
  disabled: boolean
  tone: WorkbenchInspectorTone
}

export interface WorkbenchInspectorState {
  activeMode: WorkbenchInspectorMode
  modes: WorkbenchInspectorModeItem[]
  sections: WorkbenchInspectorSection[]
  actions: WorkbenchInspectorAction[]
  attention: boolean
}

const MODE_LABELS: Record<WorkbenchInspectorMode, string> = {
  selection: 'Selection',
  validation: 'Validation',
  history: 'History',
  tasks: 'Tasks',
  diagnostics: 'Diagnostics',
}

function clean(value: unknown, fallback = '—'): string {
  const text = String(value ?? '').trim()
  if (!text || text === 'undefined' || text === 'NaN') return fallback
  return text
}

function hasUsefulText(value: unknown): boolean {
  const text = clean(value, '')
  return text.length > 0 && text !== '-' && text !== '—'
}

function positiveCount(value: unknown): number {
  const parsed = Math.floor(Number(value ?? 0))
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 0
}

function inferInspectorMode(input: WorkbenchInspectorInput): WorkbenchInspectorMode {
  if (input.requestedMode) return input.requestedMode
  const workspace = normalizeWorkspaceTab(input.workspace) || 'workflow'
  const step = normalizeWorkflowStep(input.workflowStep) || 'data'
  const results = normalizeResultsTab(input.resultsTab) || 'overview'
  if ((input.activeTaskCount ?? 0) > 0) return 'tasks'
  if (workspace === 'pro') return 'diagnostics'
  if (workspace === 'library' || results === 'history') return 'history'
  if (step === 'validate' || results === 'validate') return 'validation'
  return 'selection'
}

function section(id: string, title: string, value: unknown, detail: unknown, tone: WorkbenchInspectorTone): WorkbenchInspectorSection {
  return {
    id,
    title,
    value: clean(value),
    detail: clean(detail, ''),
    tone,
  }
}

function buildSections(input: WorkbenchInspectorInput, mode: WorkbenchInspectorMode): WorkbenchInspectorSection[] {
  if (mode === 'tasks') {
    const activeCount = Math.max(0, Number(input.activeTaskCount ?? 0))
    return [
      section('active_tasks', 'Active Tasks', activeCount ? `${activeCount} active` : 'Idle', input.taskSummary || input.status || 'No active task.', activeCount ? 'accent' : 'neutral'),
      section('status', 'Status', input.status || 'Ready', input.datasetLabel || 'No dataset', activeCount ? 'accent' : 'neutral'),
    ]
  }

  if (mode === 'validation') {
    const validationStage = clean(input.validationStage, 'not ready')
    const ready = validationStage === 'ready'
    const compatibility = clean(input.validationCompatibilitySummary, '')
    const compatibilityTone: WorkbenchInspectorTone = compatibility.toLowerCase().includes('incompatible')
      ? 'danger'
      : compatibility.toLowerCase().includes('partial') || compatibility.toLowerCase().includes('awaiting')
        ? 'warning'
        : compatibility
          ? 'success'
          : ready
            ? 'success'
            : 'warning'
    const rows = positiveCount(input.validationTableRows)
    return [
      section('validation_stage', 'Validation Stage', validationStage, input.validationSummary || 'No validation result selected.', ready ? 'success' : 'warning'),
      section('candidate', 'Candidate', input.selectedCandidateTitle || 'No candidate', input.datasetLabel || 'No dataset', clean(input.selectedCandidateTitle, '') ? 'accent' : 'neutral'),
      section('compatibility', 'Compatibility', compatibility || 'Awaiting artifact', input.validationCompatibilityDetail || input.validationStatus || '', compatibilityTone),
      section('validation_rows', 'Rows', rows ? `${rows} rows` : 'No rows', input.validationOutputPreview || input.validationCrossSummary || '', rows ? 'success' : 'neutral'),
      section('gates', 'Gates', input.validationGatesLabel || '—', input.validationCrossSummary || '', hasUsefulText(input.validationGatesLabel) ? 'accent' : 'neutral'),
    ]
  }

  if (mode === 'history') {
    return [
      section('history_scope', 'History Scope', input.historySummary || 'Session and saved runs', input.historyStatus || input.status || 'Ready', 'neutral'),
      section('session_runs', 'Session', input.historySessionSummary || '0 runs', input.selectedCandidateTitle || 'No session run selected', hasUsefulText(input.historySessionSummary) ? 'accent' : 'neutral'),
      section('saved_runs', 'Saved', input.historySavedSummary || '0 saved runs', input.historyDetailMeta || input.historyCompareSummary || '', hasUsefulText(input.historySavedSummary) ? 'accent' : 'neutral'),
      section('detail', 'Detail', input.historyDetailTitle || 'No saved run selected', input.historyDetailMeta || input.historyStatus || '', hasUsefulText(input.historyDetailTitle) ? 'success' : 'neutral'),
      section('dataset', 'Dataset', input.datasetLabel || 'No dataset', input.historyCompareSummary || 'No selected run', clean(input.datasetLabel, '') ? 'accent' : 'neutral'),
    ]
  }

  if (mode === 'diagnostics') {
    return [
      section('route', 'Route', 'Diagnostics', input.diagnosticsSummary || input.status || 'Debug output and runtime state.', 'neutral'),
      section('context', 'Context', input.datasetLabel || 'No dataset', input.selectedCandidateTitle || 'No selected candidate', 'neutral'),
      section('strategy', 'Strategy', input.strategyLabel || 'No strategy', input.backendLabel || '', hasUsefulText(input.strategyLabel) ? 'accent' : 'neutral'),
    ]
  }

  return [
    section('dataset', 'Dataset', input.datasetLabel || 'No dataset', input.status || 'Load or select data before running.', clean(input.datasetLabel, '') ? 'accent' : 'warning'),
    section('strategy', 'Strategy', input.strategyLabel || 'No strategy', input.backendLabel || 'Configure strategy inputs before running.', hasUsefulText(input.strategyLabel) ? 'accent' : 'neutral'),
    section('candidate', 'Candidate', input.selectedCandidateTitle || 'No candidate', input.selectedCandidateSummary || input.validationSummary || 'Run optimization or choose a saved run.', clean(input.selectedCandidateTitle, '') ? 'success' : 'neutral'),
  ]
}

function action(
  id: string,
  label: string,
  actionId: string,
  disabled: boolean,
  tone: WorkbenchInspectorTone = 'neutral',
): WorkbenchInspectorAction {
  return { id, label, actionId, disabled, tone }
}

function buildActions(input: WorkbenchInspectorInput, mode: WorkbenchInspectorMode): WorkbenchInspectorAction[] {
  if (mode === 'history') {
    return [
      action('history.applySession', 'Apply session', 'workbench.inspector.history.applySession', input.historyCanApplySession === false),
      action('history.refreshSaved', 'Refresh saved', 'workbench.inspector.history.refreshSaved', input.historyCanRefreshSaved === false, 'accent'),
      action('history.loadSaved', 'Load saved', 'workbench.inspector.history.loadSaved', input.historyCanLoadSaved === false),
      action('history.applySaved', 'Apply saved', 'workbench.inspector.history.applySaved', input.historyCanApplySaved === false),
      action('history.clearSession', 'Clear session', 'workbench.inspector.history.clearSession', input.historyCanClearSession === false),
      action('history.openRunsDir', 'Open runs dir', 'workbench.inspector.history.openRunsDir', input.historyCanOpenRunsDir === false),
    ]
  }
  if (mode === 'validation') {
    return [
      action('validation.open', 'Open validation', 'workbench.inspector.validation.open', false, 'accent'),
    ]
  }
  if (mode === 'tasks') {
    return [
      action('tasks.open', 'Open task center', 'workbench.inspector.tasks.open', false, 'accent'),
    ]
  }
  if (mode === 'selection') {
    return [
      action('selection.openData', 'Open data', 'workbench.inspector.selection.openData', false),
      action('selection.openOptimize', 'Open optimize', 'workbench.inspector.selection.openOptimize', false, 'accent'),
      action('selection.openAnalyze', 'Open analysis', 'workbench.inspector.selection.openAnalyze', !hasUsefulText(input.selectedCandidateTitle)),
    ]
  }
  return []
}

export function resolveWorkbenchInspectorState(input: WorkbenchInspectorInput): WorkbenchInspectorState {
  const activeMode = inferInspectorMode(input)
  const taskAttention = (input.activeTaskCount ?? 0) > 0
  const validationAttention = activeMode === 'validation' && clean(input.validationStage, 'not ready') !== 'ready'
  const modes = (Object.keys(MODE_LABELS) as WorkbenchInspectorMode[]).map((id) => {
    const current = id === activeMode
    const tone: WorkbenchInspectorTone =
      id === 'tasks' && taskAttention
        ? 'accent'
        : id === 'validation' && validationAttention
          ? 'warning'
          : current
            ? 'accent'
            : 'neutral'
    return {
      id,
      label: MODE_LABELS[id],
      tone,
      current,
      actionId: `workbench.inspector.mode.${id}`,
    }
  })

  return {
    activeMode,
    modes,
    sections: buildSections(input, activeMode),
    actions: buildActions(input, activeMode),
    attention: taskAttention || validationAttention,
  }
}
