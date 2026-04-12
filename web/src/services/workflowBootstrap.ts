import {
  AGGREGATE_TAB_KEY,
  CONTROLS_TAB_KEY,
  RESULTS_TAB_KEY,
  WORKFLOW_MODE_KEY,
  WORKFLOW_STEP_KEY,
  WORKFLOW_TAB_KEY,
  WORKSPACE_SPLIT_KEY,
  WORKSPACE_TAB_KEY,
} from '../utils/core/storageKeys'
import { normalizeShellMode } from '../utils/core/workflowModel'
import { resolveRuntimePinia } from '../stores/runtimePinia'
import { useDataStore } from '../stores/dataStore'
import { readStorageString } from '../utils/storage'
import { useWorkflowStore } from '../stores/workflowStore'
import {
  artifactHistoryPerfNow,
  measureArtifactHistoryPerf,
  recordArtifactHistoryPerf,
} from './artifactHistoryPerf'
import { syncWorkflowArtifactsFromLegacyRuntime } from './workflowArtifactsSync'

const WORKSPACE_KEYS = new Set(['workflow', 'library', 'aggregate', 'pro', 'live', 'settings'])
const WORKFLOW_MODE_KEYS = new Set(['conventional', 'ml-signal'])
const WORKFLOW_KEYS = new Set(['data', 'optimize', 'validate', 'analyze', 'finalize', 'history', 'settings'])
const WORKFLOW_STEP_KEYS = new Set(['data', 'optimize', 'validate', 'analyze', 'finalize'])
const CONTROLS_TAB_KEYS = new Set(['data', 'opt', 'strategy', 'advanced'])
const RESULTS_TAB_KEYS = new Set(['overview', 'top', 'pareto', 'validate', 'drill', 'heatmap', 'history', 'meta', 'debug'])
const AGGREGATE_TAB_KEYS = new Set(['meta', 'batch', 'saved', 'compare'])

export interface WorkflowBootstrapState {
  mode: string
  workspace: string
  workflow: string
  workflowStep: string
  controlsTab: string
  resultsTab: string
  aggregateTab: string
  splitFraction: string
}

export interface WorkflowBootstrapOptions {
  includeDeferredData?: boolean
}

function normalize(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeFromSet(value: unknown, allowed: Set<string>): string {
  const next = normalize(value)
  return allowed.has(next) ? next : ''
}

function readStorageValue(key: string): string {
  return normalize(readStorageString(key, ''))
}

export function readWorkflowBootstrapState(): WorkflowBootstrapState {
  return {
    mode: normalizeFromSet(readStorageValue(WORKFLOW_MODE_KEY), WORKFLOW_MODE_KEYS),
    workspace: normalizeFromSet(readStorageValue(WORKSPACE_TAB_KEY), WORKSPACE_KEYS),
    workflow: normalizeFromSet(readStorageValue(WORKFLOW_TAB_KEY), WORKFLOW_KEYS),
    workflowStep: normalizeFromSet(readStorageValue(WORKFLOW_STEP_KEY), WORKFLOW_STEP_KEYS),
    controlsTab: normalizeFromSet(readStorageValue(CONTROLS_TAB_KEY), CONTROLS_TAB_KEYS),
    resultsTab: normalizeFromSet(readStorageValue(RESULTS_TAB_KEY), RESULTS_TAB_KEYS),
    aggregateTab: normalizeFromSet(readStorageValue(AGGREGATE_TAB_KEY), AGGREGATE_TAB_KEYS),
    splitFraction: readStorageValue(WORKSPACE_SPLIT_KEY),
  }
}

export function applyWorkflowBootstrapFromStorage(options: WorkflowBootstrapOptions = {}): Promise<void> {
  const state = readWorkflowBootstrapState()

  const pinia = resolveRuntimePinia()
  if (!pinia) return Promise.resolve()
  const restoreShellStateStartedAt = artifactHistoryPerfNow()
  const includeDeferredData = options.includeDeferredData !== false
  const startupDataPromise = measureArtifactHistoryPerf(
    'workflowBootstrap.startupDataHydrationMs',
    () => useDataStore(pinia).hydrateStartupState({ includeDeferred: includeDeferredData }),
    {
      meta: { includeDeferredData },
    },
  )
  void startupDataPromise
    .then(() =>
      measureArtifactHistoryPerf(
        'workflowBootstrap.latestRunSyncMs',
        () => syncWorkflowArtifactsFromLegacyRuntime('legacy-runtime'),
        {
          meta: { source: 'legacy-runtime' },
          onSuccessMeta: (synced) => ({ synced }),
        },
      ),
    )
    .catch(() => undefined)

  const restoredState = !!(
    state.mode ||
    state.workspace ||
    state.workflow ||
    state.workflowStep ||
    state.controlsTab ||
    state.resultsTab ||
    state.aggregateTab ||
    state.splitFraction
  )

  if (!restoredState) {
    recordArtifactHistoryPerf(
      'workflowBootstrap.restoreShellStateMs',
      artifactHistoryPerfNow() - restoreShellStateStartedAt,
      {
        restoredState: false,
      },
    )
    return startupDataPromise
  }
  const store = useWorkflowStore(pinia)

  if (state.workspace === 'live') {
    store.applyLegacySnapshot({
      mode: normalizeShellMode(state.mode) || store.mode,
      workspace: state.workspace,
      workflow: state.workflow,
      workflowStep: state.workflowStep,
      controlsTab: state.controlsTab,
      resultsTab: state.resultsTab,
      aggregateTab: state.aggregateTab,
      splitFraction: state.splitFraction ? Number(state.splitFraction) : store.splitFraction,
    })
    recordArtifactHistoryPerf(
      'workflowBootstrap.restoreShellStateMs',
      artifactHistoryPerfNow() - restoreShellStateStartedAt,
      {
        restoredState: true,
        mode: state.mode || null,
        workspace: state.workspace || null,
        workflow: state.workflow || null,
        workflowStep: state.workflowStep || null,
        controlsTab: state.controlsTab || null,
        resultsTab: state.resultsTab || null,
        aggregateTab: state.aggregateTab || null,
        splitFraction: state.splitFraction || null,
      },
    )
    return startupDataPromise
  }

  if (state.mode) {
    const restoredMode = normalizeShellMode(state.mode)
    if (restoredMode) store.selectMode(restoredMode, { persist: false })
  }

  if (state.workspace) {
    if (state.workspace === 'workflow') {
      const candidateStep = state.workflowStep || (WORKFLOW_STEP_KEYS.has(state.workflow) ? state.workflow : '')
      store.selectWorkspace(state.workspace, candidateStep || undefined)
    } else {
      store.selectWorkspace(state.workspace)
    }
  } else if (state.workflow) {
    store.selectWorkflow(state.workflow)
  }

  if ((state.workspace === 'workflow' || (!state.workspace && store.workspace === 'workflow')) && state.workflowStep) {
    store.selectWorkflowStep(state.workflowStep)
  }

  if (state.controlsTab) {
    store.selectControlsTab(state.controlsTab)
  }

  if (state.resultsTab) {
    store.selectResultsTab(state.resultsTab)
  }

  if (state.aggregateTab) {
    const activeWorkspace = state.workspace || String(store.workspace || '').trim()
    if (activeWorkspace === 'aggregate') {
      store.selectAggregateTab(state.aggregateTab)
    } else {
      store.primeAggregateTab(state.aggregateTab)
    }
  }

  if (state.splitFraction) {
    store.setSplitFraction(Number(state.splitFraction), { persist: false })
  }

  recordArtifactHistoryPerf(
    'workflowBootstrap.restoreShellStateMs',
    artifactHistoryPerfNow() - restoreShellStateStartedAt,
    {
      restoredState: true,
      mode: state.mode || null,
      workspace: state.workspace || null,
      workflow: state.workflow || null,
      workflowStep: state.workflowStep || null,
      controlsTab: state.controlsTab || null,
      resultsTab: state.resultsTab || null,
      aggregateTab: state.aggregateTab || null,
      splitFraction: state.splitFraction || null,
    },
  )
  return startupDataPromise
}
