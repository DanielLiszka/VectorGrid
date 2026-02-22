import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  CONTROLS_TAB_KEY,
  RESULTS_TAB_KEY,
  WORKFLOW_STEP_KEY,
  WORKFLOW_TAB_KEY,
  WORKSPACE_TAB_KEY,
} from '../legacy/core/storageKeys.js'
import { useWorkflowStore } from '../stores/workflowStore'
import { installWorkflowRuntimeDispatch } from './workflowRuntimeDispatch'

type StorageMap = Record<string, string>

function installStorage(values: StorageMap): StorageMap {
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
  return state
}

describe('workflow runtime dispatch service', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.stubGlobal('window', {})
    installStorage({})
    delete (globalThis.window as Window).__VTA_VUE_WORKFLOW_DISPATCH__
  })

  it('registers dispatcher and routes actions into workflow store', () => {
    const detach = installWorkflowRuntimeDispatch()
    const store = useWorkflowStore()
    const dispatch = (globalThis.window as Window).__VTA_VUE_WORKFLOW_DISPATCH__

    expect(typeof dispatch).toBe('function')
    expect(dispatch?.('workspace.select', { workspace: 'library' })).toBe(true)
    expect(store.workspace).toBe('library')
    expect(store.workflow).toBe('history')

    expect(dispatch?.('controls.select', { tab: 'data' })).toBe(true)
    expect(store.controlsTab).toBe('data')
    expect(dispatch?.('results.select', { tab: 'history' })).toBe(true)
    expect(store.resultsTab).toBe('history')

    expect(dispatch?.('aggregate.select', { tab: 'saved', activeWorkspace: 'aggregate' })).toBe(true)
    expect(store.aggregateTab).toBe('saved')
    expect(store.workspace).toBe('aggregate')

    expect(dispatch?.('unknown.action', {})).toBe(false)

    detach()
    expect((globalThis.window as Window).__VTA_VUE_WORKFLOW_DISPATCH__).toBeUndefined()
  })

  it('supports non-persisting runtime changes for legacy save=false parity', () => {
    const storage = installStorage({})
    const dispatch = installWorkflowRuntimeDispatch()
    const store = useWorkflowStore()
    const invoke = (globalThis.window as Window).__VTA_VUE_WORKFLOW_DISPATCH__

    expect(invoke?.('workspace.select', { workspace: 'library', persist: false })).toBe(true)
    expect(invoke?.('controls.select', { tab: 'data', persist: false })).toBe(true)
    expect(invoke?.('results.select', { tab: 'history', persist: false })).toBe(true)

    expect(store.workspace).toBe('library')
    expect(store.workflow).toBe('history')
    expect(store.controlsTab).toBe('data')
    expect(store.resultsTab).toBe('history')

    expect(storage[WORKSPACE_TAB_KEY]).toBeUndefined()
    expect(storage[WORKFLOW_TAB_KEY]).toBeUndefined()
    expect(storage[WORKFLOW_STEP_KEY]).toBeUndefined()
    expect(storage[CONTROLS_TAB_KEY]).toBeUndefined()
    expect(storage[RESULTS_TAB_KEY]).toBeUndefined()

    dispatch()
  })
})
