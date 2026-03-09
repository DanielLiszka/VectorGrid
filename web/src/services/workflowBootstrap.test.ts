import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { applyWorkflowBootstrapFromStorage, readWorkflowBootstrapState } from './workflowBootstrap'
import { useWorkflowStore } from '../stores/workflowStore'

type StorageMap = Record<string, string>

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
    setActivePinia(createPinia())
    installStorage({})
  })

  it('reads and normalizes persisted bootstrap keys', () => {
    installStorage({
      vta_workspace_tab_v1: 'library',
      vta_workflow_tab_v1: 'history',
      vta_workflow_step_v1: 'optimize',
      vectorbt_controls_tab_v1: 'strategy',
      vectorbt_results_tab_v1: 'history',
      vta_aggregate_tab_v1: 'saved',
    })

    expect(readWorkflowBootstrapState()).toEqual({
      workspace: 'library',
      workflow: 'history',
      workflowStep: 'optimize',
      controlsTab: 'strategy',
      resultsTab: 'history',
      aggregateTab: 'saved',
    })
  })

  it('ignores invalid persisted values', () => {
    installStorage({
      vta_workspace_tab_v1: 'invalid_ws',
      vta_workflow_tab_v1: 'oops',
      vta_workflow_step_v1: 'wat',
      vectorbt_controls_tab_v1: 'x',
      vectorbt_results_tab_v1: 'y',
      vta_aggregate_tab_v1: 'z',
    })

    expect(readWorkflowBootstrapState()).toEqual({
      workspace: '',
      workflow: '',
      workflowStep: '',
      controlsTab: '',
      resultsTab: '',
      aggregateTab: '',
    })
  })

  it('applies persisted workflow bootstrap through workflow actions', () => {
    installStorage({
      vta_workspace_tab_v1: 'library',
      vta_workflow_tab_v1: 'history',
      vectorbt_controls_tab_v1: 'data',
      vectorbt_results_tab_v1: 'history',
      vta_aggregate_tab_v1: 'batch',
    })

    applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore()
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
    const store = useWorkflowStore()
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
    const store = useWorkflowStore()
    expect(store.workspace).toBe('aggregate')
    expect(store.aggregateTab).toBe('compare')
  })

  it('restores local tabs without changing the selected workflow location', () => {
    installStorage({
      vta_workspace_tab_v1: 'workflow',
      vta_workflow_step_v1: 'optimize',
      vectorbt_controls_tab_v1: 'strategy',
      vectorbt_results_tab_v1: 'top',
    })

    applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore()
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
      vectorbt_controls_tab_v1: 'advanced',
      vectorbt_results_tab_v1: 'meta',
    })

    applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore()
    expect(store.workspace).toBe('workflow')
    expect(store.workflowStep).toBe('finalize')
    expect(store.workflow).toBe('finalize')
    expect(store.controlsTab).toBe('advanced')
    expect(store.resultsTab).toBe('meta')
  })
})
