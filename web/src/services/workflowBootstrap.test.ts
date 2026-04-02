import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { applyWorkflowBootstrapFromStorage, readWorkflowBootstrapState } from './workflowBootstrap'
import { useDataStore } from '../stores/dataStore'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useWorkflowStore } from '../stores/workflowStore'
import { CONTROLS_TAB_KEY, RESULTS_TAB_KEY } from '../utils/core/storageKeys'

type StorageMap = Record<string, string>
let pinia: ReturnType<typeof createPinia>
let hydrateStartupStateSpy: ReturnType<typeof vi.spyOn>

function installStorage(values: StorageMap): void {
  const state: StorageMap = { ...values }
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => (Object.prototype.hasOwnProperty.call(state, key) ? state[key] : null),
    setItem: (key: string, value: string) => {
      state[key] = String(value)
    },
    removeItem: (key: string) => {
      delete state[key]
    },
  })
}

describe('workflow bootstrap service', () => {
  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    installStorage({})
    hydrateStartupStateSpy = vi
      .spyOn(useDataStore(pinia), 'hydrateStartupState')
      .mockResolvedValue(undefined)
  })

  it('reads and normalizes persisted bootstrap keys', () => {
    installStorage({
      vta_workspace_tab_v1: 'library',
      vta_workflow_tab_v1: 'history',
      vta_workflow_step_v1: 'optimize',
      vectorgrid_controls_tab_v1: 'strategy',
      vectorgrid_results_tab_v1: 'history',
      vta_aggregate_tab_v1: 'saved',
    })

    expect(readWorkflowBootstrapState()).toEqual({
      workspace: 'library',
      workflow: 'history',
      workflowStep: 'optimize',
      controlsTab: 'strategy',
      resultsTab: 'history',
      aggregateTab: 'saved',
      splitFraction: '',
    })
  })

  it('reads renamed vectorgrid bootstrap keys', () => {
    installStorage({
      vta_workspace_tab_v1: 'workflow',
      vta_workflow_tab_v1: 'validate',
      [CONTROLS_TAB_KEY]: 'advanced',
      [RESULTS_TAB_KEY]: 'validate',
    })

    expect(readWorkflowBootstrapState()).toEqual({
      workspace: 'workflow',
      workflow: 'validate',
      workflowStep: '',
      controlsTab: 'advanced',
      resultsTab: 'validate',
      aggregateTab: '',
      splitFraction: '',
    })
  })

  it('ignores invalid persisted values', () => {
    installStorage({
      vta_workspace_tab_v1: 'invalid_ws',
      vta_workflow_tab_v1: 'oops',
      vta_workflow_step_v1: 'wat',
      vectorgrid_controls_tab_v1: 'x',
      vectorgrid_results_tab_v1: 'y',
      vta_aggregate_tab_v1: 'z',
    })

    expect(readWorkflowBootstrapState()).toEqual({
      workspace: '',
      workflow: '',
      workflowStep: '',
      controlsTab: '',
      resultsTab: '',
      aggregateTab: '',
      splitFraction: '',
    })
  })

  it('applies persisted workflow bootstrap through workflow actions', () => {
    installStorage({
      vta_workspace_tab_v1: 'library',
      vta_workflow_tab_v1: 'history',
      vectorgrid_controls_tab_v1: 'data',
      vectorgrid_results_tab_v1: 'history',
      vta_aggregate_tab_v1: 'batch',
    })

    applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(hydrateStartupStateSpy).toHaveBeenCalled()
    expect(store.workspace).toBe('library')
    expect(store.workflow).toBe('history')
    expect(store.controlsTab).toBe('data')
    expect(store.resultsTab).toBe('history')
    expect(store.aggregateTab).toBe('batch')
  })

  it('applies workspace bootstrap when workflow value is absent', () => {
    installStorage({
      vta_workspace_tab_v1: 'aggregate',
    })

    applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.workspace).toBe('aggregate')
    expect(store.workflow).toBe('settings')
    expect(store.controlsTab).toBe('advanced')
    expect(store.resultsTab).toBe('meta')
  })

  it('applies aggregate bootstrap through workflow aggregate action when aggregate workspace is restored', () => {
    installStorage({
      vta_workspace_tab_v1: 'aggregate',
      vta_aggregate_tab_v1: 'compare',
    })

    applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.workspace).toBe('aggregate')
    expect(store.aggregateTab).toBe('compare')
  })

  it('restores local tabs without changing the selected workflow location', () => {
    installStorage({
      vta_workspace_tab_v1: 'workflow',
      vta_workflow_step_v1: 'optimize',
      vectorgrid_controls_tab_v1: 'strategy',
      vectorgrid_results_tab_v1: 'top',
    })

    applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.workspace).toBe('workflow')
    expect(store.workflow).toBe('optimize')
    expect(store.workflowStep).toBe('optimize')
    expect(store.controlsTab).toBe('strategy')
    expect(store.resultsTab).toBe('top')
  })

  it('restores the finalizer workflow step from storage', () => {
    installStorage({
      vta_workspace_tab_v1: 'workflow',
      vta_workflow_step_v1: 'finalize',
      vectorgrid_controls_tab_v1: 'advanced',
      vectorgrid_results_tab_v1: 'meta',
    })

    applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.workspace).toBe('workflow')
    expect(store.workflowStep).toBe('finalize')
    expect(store.workflow).toBe('finalize')
    expect(store.controlsTab).toBe('advanced')
    expect(store.resultsTab).toBe('meta')
  })

  it('reads legacy vectorbt tab keys through storage aliases', () => {
    installStorage({
      vectorbt_controls_tab_v1: 'strategy',
      vectorbt_results_tab_v1: 'history',
    })

    expect(readWorkflowBootstrapState()).toEqual({
      workspace: '',
      workflow: '',
      workflowStep: '',
      controlsTab: 'strategy',
      resultsTab: 'history',
      aggregateTab: '',
      splitFraction: '',
    })
  })

  it('restores persisted shell split fraction when present', () => {
    installStorage({
      vta_workspace_tab_v1: 'workflow',
      vta_workflow_step_v1: 'analyze',
      vta_workspace_split_v1: '0.52',
    })

    expect(readWorkflowBootstrapState()).toEqual({
      workspace: 'workflow',
      workflow: '',
      workflowStep: 'analyze',
      controlsTab: '',
      resultsTab: '',
      aggregateTab: '',
      splitFraction: '0.52',
    })

    applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.splitFraction).toBe(0.52)
  })

  it('hydrates startup data even when there is no saved workflow bootstrap state', () => {
    applyWorkflowBootstrapFromStorage()

    expect(hydrateStartupStateSpy).toHaveBeenCalled()
  })
})
