import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { applyWorkflowBootstrapFromStorage, readWorkflowBootstrapState } from './workflowBootstrap'
import { clearArtifactHistoryPerfEntries, readArtifactHistoryPerfEntries } from './artifactHistoryPerf'
import { useDataStore } from '../stores/dataStore'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useWorkflowStore } from '../stores/workflowStore'
import { CONTROLS_TAB_KEY, RESULTS_TAB_KEY } from '../utils/core/storageKeys'
import { syncWorkflowArtifactsFromLegacyRuntime } from './workflowArtifactsSync'

type StorageMap = Record<string, string>
let pinia: ReturnType<typeof createPinia>
let hydrateStartupStateSpy: ReturnType<typeof vi.spyOn>
const syncWorkflowArtifactsFromLegacyRuntimeMock = vi.mocked(syncWorkflowArtifactsFromLegacyRuntime)

vi.mock('./workflowArtifactsSync', () => ({
  syncWorkflowArtifactsFromLegacyRuntime: vi.fn(async () => false),
}))

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

function createDeferred<T = void>() {
  let resolve!: (value: T | PromiseLike<T>) => void
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve
    void nextReject
  })
  return { promise, resolve }
}

describe('workflow bootstrap service', () => {
  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    installStorage({})
    vi.clearAllMocks()
    clearArtifactHistoryPerfEntries()
    hydrateStartupStateSpy = vi
      .spyOn(useDataStore(pinia), 'hydrateStartupState')
      .mockResolvedValue(undefined)
    syncWorkflowArtifactsFromLegacyRuntimeMock.mockResolvedValue(false)
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
      mode: '',
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
      mode: '',
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
      mode: '',
      workspace: '',
      workflow: '',
      workflowStep: '',
      controlsTab: '',
      resultsTab: '',
      aggregateTab: '',
      splitFraction: '',
    })
  })

  it('applies persisted workflow bootstrap through workflow actions', async () => {
    installStorage({
      vta_workspace_tab_v1: 'library',
      vta_workflow_tab_v1: 'history',
      vectorgrid_controls_tab_v1: 'data',
      vectorgrid_results_tab_v1: 'history',
      vta_aggregate_tab_v1: 'batch',
    })

    await applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(hydrateStartupStateSpy).toHaveBeenCalled()
    expect(store.workspace).toBe('library')
    expect(store.mode).toBe('conventional')
    expect(store.workflow).toBe('history')
    expect(store.controlsTab).toBe('data')
    expect(store.resultsTab).toBe('history')
    expect(store.aggregateTab).toBe('batch')
    expect(readArtifactHistoryPerfEntries().map((entry) => entry.name)).toEqual(
      expect.arrayContaining([
        'workflowBootstrap.restoreShellStateMs',
        'workflowBootstrap.startupDataHydrationMs',
      ]),
    )
  })

  it('applies workspace bootstrap when workflow value is absent', async () => {
    installStorage({
      vta_workspace_tab_v1: 'aggregate',
    })

    await applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.workspace).toBe('aggregate')
    expect(store.mode).toBe('conventional')
    expect(store.workflow).toBe('settings')
    expect(store.controlsTab).toBe('advanced')
    expect(store.resultsTab).toBe('meta')
  })

  it('applies aggregate bootstrap through workflow aggregate action when aggregate workspace is restored', async () => {
    installStorage({
      vta_workspace_tab_v1: 'aggregate',
      vta_aggregate_tab_v1: 'compare',
    })

    await applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.workspace).toBe('aggregate')
    expect(store.mode).toBe('conventional')
    expect(store.aggregateTab).toBe('compare')
  })

  it('restores local tabs without changing the selected workflow location', async () => {
    installStorage({
      vta_workspace_tab_v1: 'workflow',
      vta_workflow_step_v1: 'optimize',
      vectorgrid_controls_tab_v1: 'strategy',
      vectorgrid_results_tab_v1: 'top',
    })

    await applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.workspace).toBe('workflow')
    expect(store.mode).toBe('conventional')
    expect(store.workflow).toBe('optimize')
    expect(store.workflowStep).toBe('optimize')
    expect(store.controlsTab).toBe('strategy')
    expect(store.resultsTab).toBe('top')
  })

  it('restores the persisted live workspace together with its underlying workflow posture', async () => {
    installStorage({
      vta_workspace_tab_v1: 'live',
      vta_workflow_tab_v1: 'optimize',
      vta_workflow_step_v1: 'optimize',
      vectorgrid_controls_tab_v1: 'strategy',
      vectorgrid_results_tab_v1: 'top',
    })

    await applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.workspace).toBe('live')
    expect(store.mode).toBe('conventional')
    expect(store.workflow).toBe('optimize')
    expect(store.workflowStep).toBe('optimize')
    expect(store.controlsTab).toBe('strategy')
    expect(store.resultsTab).toBe('top')
  })

  it('clamps invalid persisted live-workspace tabs for MlSignal posture', async () => {
    installStorage({
      vta_workflow_mode_v1: 'ml-signal',
      vta_workspace_tab_v1: 'live',
      vta_workflow_tab_v1: 'validate',
      vta_workflow_step_v1: 'validate',
      vectorgrid_controls_tab_v1: 'strategy',
      vectorgrid_results_tab_v1: 'top',
    })

    await applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.workspace).toBe('live')
    expect(store.mode).toBe('ml-signal')
    expect(store.workflow).toBe('validate')
    expect(store.workflowStep).toBe('validate')
    expect(store.controlsTab).toBe('opt')
    expect(store.resultsTab).toBe('overview')
  })

  it('restores the finalizer workflow step from storage', async () => {
    installStorage({
      vta_workspace_tab_v1: 'workflow',
      vta_workflow_step_v1: 'finalize',
      vectorgrid_controls_tab_v1: 'advanced',
      vectorgrid_results_tab_v1: 'meta',
    })

    await applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.workspace).toBe('workflow')
    expect(store.mode).toBe('conventional')
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
      mode: '',
      workspace: '',
      workflow: '',
      workflowStep: '',
      controlsTab: 'strategy',
      resultsTab: 'history',
      aggregateTab: '',
      splitFraction: '',
    })
  })

  it('restores persisted shell split fraction when present', async () => {
    installStorage({
      vta_workspace_tab_v1: 'workflow',
      vta_workflow_step_v1: 'analyze',
      vta_workspace_split_v1: '0.52',
    })

    expect(readWorkflowBootstrapState()).toEqual({
      mode: '',
      workspace: 'workflow',
      workflow: '',
      workflowStep: 'analyze',
      controlsTab: '',
      resultsTab: '',
      aggregateTab: '',
      splitFraction: '0.52',
    })

    await applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.splitFraction).toBe(0.52)
  })

  it('restores persisted MlSignal shell mode before workflow posture', async () => {
    installStorage({
      vta_workflow_mode_v1: 'ml-signal',
      vta_workspace_tab_v1: 'workflow',
      vta_workflow_step_v1: 'validate',
      vectorgrid_controls_tab_v1: 'opt',
    })

    expect(readWorkflowBootstrapState()).toEqual({
      mode: 'ml-signal',
      workspace: 'workflow',
      workflow: '',
      workflowStep: 'validate',
      controlsTab: 'opt',
      resultsTab: '',
      aggregateTab: '',
      splitFraction: '',
    })

    await applyWorkflowBootstrapFromStorage()
    const store = useWorkflowStore(pinia)
    expect(store.mode).toBe('ml-signal')
    expect(store.workflowStep).toBe('validate')
    expect(store.controlsTab).toBe('opt')
    expect(store.resultsTab).toBe('overview')
  })

  it('hydrates startup data even when there is no saved workflow bootstrap state', async () => {
    await applyWorkflowBootstrapFromStorage()

    expect(hydrateStartupStateSpy).toHaveBeenCalled()
  })

  it('returns the startup hydration promise so boot can await critical readiness', async () => {
    const deferred = createDeferred<void>()
    hydrateStartupStateSpy.mockReturnValueOnce(deferred.promise)

    let settled = false
    const bootstrapPromise = applyWorkflowBootstrapFromStorage({ includeDeferredData: false }).then(() => {
      settled = true
    })

    expect(hydrateStartupStateSpy).toHaveBeenCalledWith({ includeDeferred: false })
    await Promise.resolve()
    expect(settled).toBe(false)

    deferred.resolve()
    await bootstrapPromise
    expect(settled).toBe(true)
    expect(readArtifactHistoryPerfEntries()).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          name: 'workflowBootstrap.startupDataHydrationMs',
          meta: expect.objectContaining({
            includeDeferredData: false,
            outcome: 'ok',
          }),
        }),
      ]),
    )
  })

  it('schedules latest-run artifact sync after startup hydration without blocking bootstrap readiness', async () => {
    const deferred = createDeferred<void>()
    const artifactSyncDeferred = createDeferred<boolean>()
    hydrateStartupStateSpy.mockReturnValueOnce(deferred.promise)
    syncWorkflowArtifactsFromLegacyRuntimeMock.mockReturnValueOnce(artifactSyncDeferred.promise)

    let settled = false
    const bootstrapPromise = applyWorkflowBootstrapFromStorage({ includeDeferredData: false }).then(() => {
      settled = true
    })

    await Promise.resolve()
    expect(syncWorkflowArtifactsFromLegacyRuntimeMock).not.toHaveBeenCalled()
    expect(settled).toBe(false)

    deferred.resolve()
    await bootstrapPromise
    await Promise.resolve()
    expect(settled).toBe(true)
    expect(syncWorkflowArtifactsFromLegacyRuntimeMock).toHaveBeenCalledWith('legacy-runtime')
    artifactSyncDeferred.resolve(false)
    await Promise.resolve()
    expect(readArtifactHistoryPerfEntries()).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          name: 'workflowBootstrap.latestRunSyncMs',
          meta: expect.objectContaining({
            source: 'legacy-runtime',
            synced: false,
            outcome: 'ok',
          }),
        }),
      ]),
    )
  })

  it('propagates startup hydration failures so boot can surface them', async () => {
    hydrateStartupStateSpy.mockReturnValueOnce(Promise.reject(new Error('hydrate failed')))

    await expect(applyWorkflowBootstrapFromStorage({ includeDeferredData: false })).rejects.toThrow('hydrate failed')
  })
})
