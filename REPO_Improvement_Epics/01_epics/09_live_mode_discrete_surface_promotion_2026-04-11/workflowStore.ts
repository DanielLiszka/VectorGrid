import { defineStore } from 'pinia'
import {
  AGGREGATE_TAB_KEY,
  CONTROLS_TAB_KEY,
  RESULTS_TAB_KEY,
  WORKFLOW_MODE_KEY,
  WORKFLOW_STATE_KEY,
  WORKFLOW_STEP_KEY,
  WORKFLOW_TAB_KEY,
  WORKSPACE_SPLIT_KEY,
  WORKSPACE_TAB_KEY,
} from '../utils/core/storageKeys'
import {
  type ShellMode,
  TAB_LABELS,
  WORKSPACE_SUBTITLES,
  normalizeAggregateTab,
  normalizeControlsTab,
  normalizeResultsTab,
  normalizeShellMode,
  normalizeWorkflowStep,
  normalizeWorkflowTab,
  normalizeWorkspaceTab,
  shellModeLabel,
  workflowAllowedForMode,
  workflowDefaultsForMode,
  workflowStepFromWorkflow,
  workflowStepLabelForMode,
  workflowStepSubtitleForMode,
} from '../utils/core/workflowModel'
import { readStorageString, writeStorageString } from '../utils/storage'

interface WorkflowState {
  mode: ShellMode
  workspace: string
  workflow: string
  workflowStep: string
  controlsTab: string
  resultsTab: string
  aggregateTab: string
  splitFraction: number
  workflowSubtitle: string
  source: 'vue' | 'unknown'
  updatedAtUnixMs: number
}

interface WorkflowMemoryEntry {
  controls: string
  results: string
}

type WorkflowMemory = Partial<Record<string, WorkflowMemoryEntry>>

interface WorkflowCommitOptions {
  persist?: boolean
}

const DEFAULT_WORKSPACE = 'workflow'
const DEFAULT_MODE: ShellMode = 'conventional'
const DEFAULT_WORKFLOW = 'data'
const DEFAULT_STEP = 'data'
const DEFAULT_CONTROLS_TAB = 'data'
const DEFAULT_RESULTS_TAB = 'overview'
const DEFAULT_AGGREGATE_TAB = 'meta'
const DEFAULT_SPLIT_FRACTION = 0.34

function normalizeWorkspace(value: unknown): string {
  return normalizeWorkspaceTab(value) || ''
}

function normalizeMode(value: unknown): ShellMode {
  return normalizeShellMode(value) || DEFAULT_MODE
}

function parseMode(value: unknown): ShellMode | '' {
  return normalizeShellMode(value) || ''
}

function normalizeWorkflow(value: unknown): string {
  return normalizeWorkflowTab(value) || ''
}

function normalizeStep(value: unknown): string {
  return normalizeWorkflowStep(value) || ''
}

function normalizeControls(value: unknown): string {
  return normalizeControlsTab(value) || ''
}

function normalizeResults(value: unknown): string {
  return normalizeResultsTab(value) || ''
}

function normalizeAggregate(value: unknown): string {
  return normalizeAggregateTab(value) || ''
}

function workflowStep(workflow: string): string {
  return normalizeStep(workflowStepFromWorkflow(workflow)) || DEFAULT_STEP
}

function workflowMemoryKey(mode: ShellMode, workflow: string): string {
  return `${mode}:${workflow}`
}

function storageSet(key: string, value: string): void {
  writeStorageString(key, value)
}

function normalizeSplitFraction(value: unknown): number {
  const next = Number(value)
  if (!Number.isFinite(next)) return DEFAULT_SPLIT_FRACTION
  return Math.min(0.9, Math.max(0.1, next))
}

function setRootAttr(name: string, value: string): void {
  if (typeof document === 'undefined') return
  const root = document.documentElement
  if (!root || !root.dataset) return
  if (value) root.dataset[name as keyof DOMStringMap] = value
  else delete root.dataset[name as keyof DOMStringMap]
}

function parseWorkflowMemory(raw: string): WorkflowMemory {
  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>
    if (!parsed || typeof parsed !== 'object') return {}
    const out: WorkflowMemory = {}
    for (const [key, value] of Object.entries(parsed)) {
      const separatorIndex = key.indexOf(':')
      const legacyWorkflow = normalizeWorkflow(key)
      const workflow = separatorIndex >= 0 ? normalizeWorkflow(key.slice(separatorIndex + 1)) : legacyWorkflow
      const mode = separatorIndex >= 0 ? normalizeMode(key.slice(0, separatorIndex)) : DEFAULT_MODE
      if (!workflow || !value || typeof value !== 'object') continue
      const entry = value as Record<string, unknown>
      const controls = normalizeControls(entry.controls)
      const results = normalizeResults(entry.results)
      if (!controls || !results) continue
      out[workflowMemoryKey(mode, workflow)] = { controls, results }
    }
    return out
  } catch {
    return {}
  }
}

function readWorkflowMemory(): WorkflowMemory {
  try {
    const raw = readStorageString(WORKFLOW_STATE_KEY, '')
    if (!raw) return {}
    return parseWorkflowMemory(raw)
  } catch {
    return {}
  }
}

const workflowMemory: WorkflowMemory = readWorkflowMemory()

function persistWorkflowMemory(): void {
  writeStorageString(WORKFLOW_STATE_KEY, JSON.stringify(workflowMemory))
}

function allowedTabs(mode: ShellMode, workflow: string): { controls: string[]; results: string[] } {
  const model = (workflowAllowedForMode(mode) as Record<string, { controls?: readonly string[]; results?: readonly string[] }>)[workflow] || {}
  const controls = Array.isArray(model.controls)
    ? model.controls.map((entry) => normalizeControls(entry)).filter((entry) => entry.length > 0)
    : []
  const results = Array.isArray(model.results)
    ? model.results.map((entry) => normalizeResults(entry)).filter((entry) => entry.length > 0)
    : []
  return { controls, results }
}

function defaultTabs(mode: ShellMode, workflow: string): { controls: string; results: string } {
  const model = (workflowDefaultsForMode(mode) as Record<string, { controls?: string; results?: string }>)[workflow] || {}
  return {
    controls: normalizeControls(model.controls) || DEFAULT_CONTROLS_TAB,
    results: normalizeResults(model.results) || DEFAULT_RESULTS_TAB,
  }
}

function resolveTabs(
  mode: ShellMode,
  workflow: string,
  preferred?: { controls?: string; results?: string },
): { controls: string; results: string } {
  const wf = normalizeWorkflow(workflow) || DEFAULT_WORKFLOW
  const allowed = allowedTabs(mode, wf)
  const defaults = defaultTabs(mode, wf)
  const remembered = workflowMemory[workflowMemoryKey(mode, wf)]

  const desiredControls =
    normalizeControls(preferred?.controls)
    || normalizeControls(remembered?.controls)
    || defaults.controls
  const desiredResults =
    normalizeResults(preferred?.results)
    || normalizeResults(remembered?.results)
    || defaults.results

  const controls = allowed.controls.includes(desiredControls)
    ? desiredControls
    : (allowed.controls.find((entry) => entry === defaults.controls) || allowed.controls[0] || DEFAULT_CONTROLS_TAB)
  const results = allowed.results.includes(desiredResults)
    ? desiredResults
    : (allowed.results.find((entry) => entry === defaults.results) || allowed.results[0] || DEFAULT_RESULTS_TAB)

  return { controls, results }
}

function tabAllowed(kind: 'controls' | 'results', mode: ShellMode, workflow: string, tab: string): boolean {
  const allowed = allowedTabs(mode, workflow)
  const next = kind === 'controls' ? normalizeControls(tab) : normalizeResults(tab)
  if (!next) return false
  return kind === 'controls'
    ? allowed.controls.includes(next)
    : allowed.results.includes(next)
}

function rememberWorkflowTabs(mode: ShellMode, workflow: string, controls: string, results: string): void {
  const wf = normalizeWorkflow(workflow)
  if (!wf) return
  const tabs = resolveTabs(mode, wf, { controls, results })
  workflowMemory[workflowMemoryKey(mode, wf)] = tabs
  persistWorkflowMemory()
}

function inferWorkflowFromTabs(controlsTab: string, resultsTab: string, currentWorkflow: string): string {
  const controls = normalizeControls(controlsTab)
  const results = normalizeResults(resultsTab)

  if (results === 'validate') return 'validate'
  if (results === 'history') return 'history'
  if (results === 'meta') {
    return normalizeWorkflow(currentWorkflow) === 'finalize' ? 'finalize' : 'settings'
  }
  if (results === 'debug') return 'settings'
  if (results === 'drill') return 'analyze'
  if (controls === 'data' && (!results || results === 'overview')) return 'data'
  if (normalizeWorkflow(currentWorkflow) === 'data') return 'optimize'
  return normalizeWorkflow(currentWorkflow) || 'optimize'
}

function workspaceFromWorkflow(workflow: string, resultsTab: string, keepSettingsOverride = false): string {
  const wf = normalizeWorkflow(workflow) || DEFAULT_WORKFLOW
  if (wf === 'history') return 'library'
  if (wf === 'settings') {
    const results = normalizeResults(resultsTab)
    if (results === 'debug') return 'pro'
    return 'aggregate'
  }
  if (wf === 'finalize') return 'workflow'
  if (keepSettingsOverride && wf === 'data') return 'settings'
  return 'workflow'
}

function tabLabel(kind: 'controls' | 'results', key: string): string {
  const map = (TAB_LABELS as Record<string, Record<string, string>>)[kind] || {}
  return String(map[key] || '').trim()
}

function deriveSubtitle(state: Pick<WorkflowState, 'mode' | 'workspace' | 'workflow' | 'workflowStep' | 'controlsTab' | 'resultsTab'>): string {
  const mode = normalizeMode(state.mode)
  const workspace = normalizeWorkspace(state.workspace) || DEFAULT_WORKSPACE
  const workflow = normalizeWorkflow(state.workflow) || DEFAULT_WORKFLOW
  const step = normalizeStep(state.workflowStep) || workflowStep(workflow)
  const controls = normalizeControls(state.controlsTab) || DEFAULT_CONTROLS_TAB
  const results = normalizeResults(state.resultsTab) || DEFAULT_RESULTS_TAB

  const workspaceSubtitles = WORKSPACE_SUBTITLES as Record<string, string>

  const base = workspace === 'workflow'
    ? workflowStepSubtitleForMode(step, mode)
    : String(workspaceSubtitles[workspace] || '').trim()

  let view = ''
  if (workspace === 'workflow' && mode === 'ml-signal') {
    const modeText = shellModeLabel(mode)
    const stepLabel = workflowStepLabelForMode(step, mode)
    view = `${modeText} • ${stepLabel}`
  } else if (workspace === 'workflow' && workflow === 'optimize') {
    const controlsLabel = tabLabel('controls', controls)
    if (controlsLabel && controls !== 'opt') view = controlsLabel
    const resultsLabel = tabLabel('results', results)
    if (resultsLabel && results !== 'overview') view = view ? `${view} • ${resultsLabel}` : resultsLabel
  } else if (workspace === 'workflow' && workflow === 'analyze') {
    view = tabLabel('results', results)
  } else if (workspace === 'workflow' && workflow === 'finalize') {
    view = 'Meta + ML'
  } else if (workspace === 'aggregate' || workspace === 'pro') {
    const controlsLabel = tabLabel('controls', controls)
    const resultsLabel = tabLabel('results', results)
    if (controlsLabel && workspace === 'pro') view = controlsLabel
    if (resultsLabel && (workspace === 'pro' || results !== 'meta')) {
      view = view ? `${view} • ${resultsLabel}` : resultsLabel
    }
  } else if (workspace === 'settings') {
    view = 'Appearance + storage'
  }

  const next = base ? (view ? `${base} (${view})` : base) : (view || '—')
  return String(next || '').trim() || '—'
}

function syncRuntimeFromState(state: WorkflowState): void {
  setRootAttr('mode', state.mode)
  setRootAttr('workspace', state.workspace)
  setRootAttr('workflow', state.workflow)
  setRootAttr('workflowStep', state.workflowStep)
  setRootAttr('controlsTab', state.controlsTab)
  setRootAttr('resultsTab', state.resultsTab)
  setRootAttr('aggregateTab', state.aggregateTab)
  setRootAttr('splitFraction', String(normalizeSplitFraction(state.splitFraction)))
  setRootAttr('workflowSubtitle', state.workflowSubtitle)
}

function persistRuntimeState(state: WorkflowState): void {
  storageSet(WORKFLOW_MODE_KEY, state.mode)
  storageSet(WORKSPACE_TAB_KEY, state.workspace)
  storageSet(WORKFLOW_TAB_KEY, state.workflow)
  storageSet(WORKFLOW_STEP_KEY, state.workflowStep)
  storageSet(CONTROLS_TAB_KEY, state.controlsTab)
  storageSet(RESULTS_TAB_KEY, state.resultsTab)
  storageSet(AGGREGATE_TAB_KEY, state.aggregateTab)
  storageSet(WORKSPACE_SPLIT_KEY, String(normalizeSplitFraction(state.splitFraction)))
}

export const useWorkflowStore = defineStore('workflow', {
  state: (): WorkflowState => ({
    mode: DEFAULT_MODE,
    workspace: DEFAULT_WORKSPACE,
    workflow: DEFAULT_WORKFLOW,
    workflowStep: DEFAULT_STEP,
    controlsTab: DEFAULT_CONTROLS_TAB,
    resultsTab: DEFAULT_RESULTS_TAB,
    aggregateTab: DEFAULT_AGGREGATE_TAB,
    splitFraction: DEFAULT_SPLIT_FRACTION,
    workflowSubtitle: '—',
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
    selectMode(mode: unknown, options?: WorkflowCommitOptions): void {
      const nextMode = parseMode(mode)
      if (!nextMode) return
      if (this.mode === nextMode) return

      const previousMode = this.mode
      rememberWorkflowTabs(previousMode, this.workflow, this.controlsTab, this.resultsTab)

      this.mode = nextMode

      if (this.workspace === 'workflow') {
        const nextWorkflow = normalizeWorkflow(this.workflowStep) || DEFAULT_WORKFLOW
        const tabs = resolveTabs(nextMode, nextWorkflow)
        this.workflow = nextWorkflow
        this.workflowStep = workflowStep(nextWorkflow)
        this.controlsTab = tabs.controls
        this.resultsTab = tabs.results
      } else if (this.workspace === 'aggregate') {
        this.workflow = 'settings'
        this.controlsTab = 'advanced'
        this.resultsTab = 'meta'
      } else if (this.workspace === 'pro') {
        this.workflow = 'settings'
        this.controlsTab = 'advanced'
        this.resultsTab = 'debug'
      } else if (this.workspace === 'settings') {
        this.workflow = 'data'
        this.controlsTab = 'data'
        this.resultsTab = 'overview'
      } else if (this.workspace === 'library') {
        this.workflow = 'history'
        this.controlsTab = 'data'
        this.resultsTab = 'history'
      }

      this.commitState('vue', true, options)
    },

    commitState(
      source: WorkflowState['source'],
      rememberTabs = true,
      options?: WorkflowCommitOptions,
    ): void {
      this.workflowSubtitle = deriveSubtitle(this)
      this.source = source
      this.updatedAtUnixMs = Date.now()
      syncRuntimeFromState(this)
      const shouldPersist = options?.persist !== false
      if (source === 'vue' && shouldPersist) persistRuntimeState(this)
      if (rememberTabs) rememberWorkflowTabs(this.mode, this.workflow, this.controlsTab, this.resultsTab)
    },

    syncRuntimeState(): void {
      this.workflowSubtitle = deriveSubtitle(this)
      syncRuntimeFromState(this)
    },

    applyLegacySnapshot(snapshot: Partial<WorkflowState>): void {
      const nextMode = parseMode(snapshot.mode) || this.mode || DEFAULT_MODE
      const explicitControls = normalizeControls(snapshot.controlsTab)
      const explicitResults = normalizeResults(snapshot.resultsTab)

      let workflow = normalizeWorkflow(snapshot.workflow) || normalizeWorkflow(this.workflow) || DEFAULT_WORKFLOW
      if (!normalizeWorkflow(snapshot.workflow) && (explicitControls || explicitResults)) {
        const inferred = inferWorkflowFromTabs(explicitControls || this.controlsTab, explicitResults || this.resultsTab, workflow)
        workflow = normalizeWorkflow(inferred) || workflow
      }

      const fallbackTabs = resolveTabs(nextMode, workflow, {
        controls: explicitControls || this.controlsTab,
        results: explicitResults || this.resultsTab,
      })
      const controls = explicitControls || fallbackTabs.controls
      const results = explicitResults || fallbackTabs.results

      const explicitWorkspace = normalizeWorkspace(snapshot.workspace)
      const keepSettingsOverride = this.workspace === 'settings' && workflow === 'data'
      const shouldInferWorkspace = !!explicitControls || !!explicitResults || workflow !== this.workflow
      const workspace = explicitWorkspace
        || (shouldInferWorkspace
          ? workspaceFromWorkflow(workflow, results, keepSettingsOverride)
          : (this.workspace || workspaceFromWorkflow(workflow, results, keepSettingsOverride)))

      const explicitStep = normalizeStep(snapshot.workflowStep)
      const step = explicitStep || (workflow !== this.workflow ? workflowStep(workflow) : (this.workflowStep || workflowStep(workflow)))

      this.mode = nextMode
      this.workspace = workspace
      this.workflow = workflow
      this.workflowStep = step
      this.controlsTab = controls
      this.resultsTab = results
      this.aggregateTab = normalizeAggregate(snapshot.aggregateTab) || this.aggregateTab || DEFAULT_AGGREGATE_TAB
      this.splitFraction = normalizeSplitFraction(snapshot.splitFraction ?? this.splitFraction)

      const subtitle = String(snapshot.workflowSubtitle ?? '').trim()
      this.workflowSubtitle = subtitle || deriveSubtitle(this)
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
      syncRuntimeFromState(this)
      rememberWorkflowTabs(this.mode, this.workflow, this.controlsTab, this.resultsTab)
    },

    selectWorkspace(workspace: string, step?: string, options?: WorkflowCommitOptions): void {
      const nextWorkspace = normalizeWorkspace(workspace)
      if (!nextWorkspace) return

      const prevWorkflow = this.workflow
      const prevControls = this.controlsTab
      const prevResults = this.resultsTab

      let nextWorkflow = this.workflow
      let nextStep = this.workflowStep
      let nextControls = this.controlsTab
      let nextResults = this.resultsTab

      if (nextWorkspace === 'workflow') {
        const desiredStep = normalizeStep(step) || normalizeStep(this.workflowStep) || workflowStep(this.workflow)
        if (desiredStep !== this.workflow) rememberWorkflowTabs(this.mode, prevWorkflow, prevControls, prevResults)
        const tabs = resolveTabs(this.mode, desiredStep)
        nextWorkflow = desiredStep
        nextStep = desiredStep
        nextControls = tabs.controls
        nextResults = tabs.results
      } else if (nextWorkspace === 'library') {
        if (this.workflow !== 'history') rememberWorkflowTabs(this.mode, prevWorkflow, prevControls, prevResults)
        const tabs = resolveTabs(this.mode, 'history')
        nextWorkflow = 'history'
        nextStep = workflowStep('history')
        nextControls = tabs.controls
        nextResults = tabs.results
      } else if (nextWorkspace === 'aggregate') {
        if (this.workflow !== 'settings') rememberWorkflowTabs(this.mode, prevWorkflow, prevControls, prevResults)
        nextWorkflow = 'settings'
        nextStep = workflowStep('settings')
        nextControls = 'advanced'
        nextResults = 'meta'
      } else if (nextWorkspace === 'pro') {
        if (this.workflow !== 'settings') rememberWorkflowTabs(this.mode, prevWorkflow, prevControls, prevResults)
        nextWorkflow = 'settings'
        nextStep = workflowStep('settings')
        nextControls = 'advanced'
        nextResults = 'debug'
      } else if (nextWorkspace === 'settings') {
        if (this.workflow !== 'data') rememberWorkflowTabs(this.mode, prevWorkflow, prevControls, prevResults)
        nextWorkflow = 'data'
        nextStep = workflowStep('data')
        nextControls = 'data'
        nextResults = 'overview'
      }

      const changed =
        this.workspace !== nextWorkspace
        || this.workflow !== nextWorkflow
        || this.workflowStep !== nextStep
        || this.controlsTab !== nextControls
        || this.resultsTab !== nextResults

      if (!changed) return

      this.workspace = nextWorkspace
      this.workflow = nextWorkflow
      this.workflowStep = nextStep
      this.controlsTab = nextControls
      this.resultsTab = nextResults
      this.commitState('vue', true, options)
    },

    selectWorkflow(workflow: string, options?: WorkflowCommitOptions): void {
      const nextWorkflow = normalizeWorkflow(workflow)
      if (!nextWorkflow) return

      const keepSettingsOverride = this.workspace === 'settings' && nextWorkflow === 'data'
      const prevWorkflow = this.workflow
      const prevControls = this.controlsTab
      const prevResults = this.resultsTab

      let nextWorkspace = workspaceFromWorkflow(nextWorkflow, this.resultsTab, keepSettingsOverride)
      let nextStep = workflowStep(nextWorkflow)
      let nextControls = this.controlsTab
      let nextResults = this.resultsTab

      if (nextWorkspace === 'aggregate') {
        nextControls = 'advanced'
        nextResults = 'meta'
      } else if (nextWorkspace === 'pro') {
        nextControls = 'advanced'
        nextResults = 'debug'
      } else if (nextWorkspace === 'settings') {
        nextControls = 'data'
        nextResults = 'overview'
      } else {
        const tabs = resolveTabs(this.mode, nextWorkflow)
        nextControls = tabs.controls
        nextResults = tabs.results
      }

      if (nextWorkflow !== this.workflow) rememberWorkflowTabs(this.mode, prevWorkflow, prevControls, prevResults)

      const changed =
        this.workflow !== nextWorkflow
        || this.workspace !== nextWorkspace
        || this.workflowStep !== nextStep
        || this.controlsTab !== nextControls
        || this.resultsTab !== nextResults

      if (!changed) return

      this.workflow = nextWorkflow
      this.workspace = nextWorkspace
      this.workflowStep = nextStep
      this.controlsTab = nextControls
      this.resultsTab = nextResults
      this.commitState('vue', true, options)
    },

    selectWorkflowStep(step: string, options?: WorkflowCommitOptions): void {
      const nextStep = normalizeStep(step)
      if (!nextStep) return
      if (this.workspace === 'workflow' && this.workflow === nextStep && this.workflowStep === nextStep) return

      rememberWorkflowTabs(this.mode, this.workflow, this.controlsTab, this.resultsTab)
      const tabs = resolveTabs(this.mode, nextStep)

      this.workspace = 'workflow'
      this.workflow = nextStep
      this.workflowStep = nextStep
      this.controlsTab = tabs.controls
      this.resultsTab = tabs.results
      this.commitState('vue', true, options)
    },

    selectControlsTab(tab: string, options?: WorkflowCommitOptions): void {
      const nextTab = normalizeControls(tab)
      if (!nextTab || this.controlsTab === nextTab) return
      if (!tabAllowed('controls', this.mode, this.workflow, nextTab)) return

      this.controlsTab = nextTab
      this.commitState('vue', true, options)
    },

    selectResultsTab(tab: string, options?: WorkflowCommitOptions): void {
      const nextTab = normalizeResults(tab)
      if (!nextTab || this.resultsTab === nextTab) return
      if (!tabAllowed('results', this.mode, this.workflow, nextTab)) return

      this.resultsTab = nextTab
      this.commitState('vue', true, options)
    },

    selectAggregateTab(tab: string, options?: WorkflowCommitOptions): void {
      const nextTab = normalizeAggregate(tab)
      if (!nextTab) return
      if (this.workspace === 'aggregate' && this.aggregateTab === nextTab) return

      if (this.workflow !== 'settings') {
        rememberWorkflowTabs(this.mode, this.workflow, this.controlsTab, this.resultsTab)
      }

      this.workspace = 'aggregate'
      this.workflow = 'settings'
      this.workflowStep = workflowStep('settings')
      this.controlsTab = 'advanced'
      this.resultsTab = 'meta'
      this.aggregateTab = nextTab
      this.commitState('vue', true, options)
    },

    setSplitFraction(value: number, options?: WorkflowCommitOptions): void {
      const next = normalizeSplitFraction(value)
      if (Math.abs(this.splitFraction - next) < 0.0005) return
      this.splitFraction = next
      this.commitState('vue', false, options)
    },

    primeAggregateTab(tab: string, options?: WorkflowCommitOptions): void {
      const nextTab = normalizeAggregate(tab)
      if (!nextTab || this.aggregateTab === nextTab) return
      this.aggregateTab = nextTab
      this.commitState('vue', true, options)
    },
  },
})

