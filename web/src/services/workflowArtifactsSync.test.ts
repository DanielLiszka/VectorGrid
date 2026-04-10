import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { refreshAnalysisFromWorkflowRun } from './analysisService'
import { syncWorkflowArtifactsFromLegacyRuntime } from './workflowArtifactsSync'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

vi.mock('./analysisService', () => ({
  refreshAnalysisFromWorkflowRun: vi.fn(() => true),
}))

const invokeCommandMock = vi.mocked(invokeCommand)
const refreshAnalysisFromWorkflowRunMock = vi.mocked(refreshAnalysisFromWorkflowRun)
let pinia: ReturnType<typeof createPinia>

describe('workflowArtifactsSync', () => {
  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    useWorkflowArtifactsStore(pinia).clearArtifacts()
    vi.clearAllMocks()
  })

  it('hydrates workflow artifact store from latest saved run artifact', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'list_run_artifacts') {
        return [{
          ts: '2026-02-25T12:00:00Z',
          path: 'C:/runs/run.json',
          report_xlsx_path: 'C:/runs/report.xlsx',
          csv_path: 'C:/data/sample.csv',
          tags: ['alpha'],
        }]
      }

      if (command === 'load_run_artifact_hydration') {
        return {
          currentRun: {
            ts: '2026-02-25T12:00:00Z',
            artifact_path: 'C:/runs/run.json',
            report_xlsx_path: 'C:/runs/report.xlsx',
            csv_path: 'C:/data/sample.csv',
            tags: ['alpha'],
            req: { strategy_kind: 'DoubleMa' },
            res: { rows: 1 },
          },
          currentValidation: {
            ts: '2026-02-25T12:00:00Z',
            req: { candidates: 32 },
            res: { rows: [] },
          },
        }
      }

      return undefined
    })

    const synced = await syncWorkflowArtifactsFromLegacyRuntime('legacy-runtime')
    const store = useWorkflowArtifactsStore(pinia)

    expect(synced).toBe(true)
    expect(store.currentRun?.artifactPath).toContain('run.json')
    expect(store.currentValidation?.ts).toBe('2026-02-25T12:00:00Z')
    expect(store.source).toBe('legacy-runtime')
    expect(refreshAnalysisFromWorkflowRunMock).toHaveBeenCalledTimes(1)
    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'list_run_artifacts')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'load_run_artifact_hydration', { path: 'C:/runs/run.json' })
  })

  it('preserves dataset metadata, tags, ml signal, and metric surface when hydrating the latest artifact', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'list_run_artifacts') {
        return [{
          ts: '2026-02-25T12:00:00Z',
          path: 'C:/runs/run.json',
          report_xlsx_path: 'C:/runs/report.xlsx',
          csv_path: 'C:/data/sample.csv',
          tags: ['alpha', 'beta'],
        }]
      }

      if (command === 'load_run_artifact_hydration') {
        return {
          currentRun: {
            ts: '2026-02-25T12:00:00Z',
            artifact_path: 'C:/runs/run.json',
            report_xlsx_path: 'C:/runs/report.xlsx',
            csv_path: 'C:/data/sample.csv',
            dataset_meta: {
              csv_path: 'C:/data/sample.csv',
              timeframe: '15m',
              base_timeframe: '1m',
              derived_timeframe: '15m',
              resample_applied: true,
            },
            ml_signal: {
              stage: 'stage5',
              bundle_id: 'bundle-1',
              score_cache_path: 'C:/runs/score_cache.json',
            },
            optimization_metric_surface: {
              surface: 'trade22',
              fields: ['pnl', 'sharpe'],
            },
            tags: ['alpha', 'beta'],
            req: { strategy_kind: 'CompositeTrend' },
            res: { rows: 1 },
          },
          currentValidation: null,
        }
      }

      return undefined
    })

    const synced = await syncWorkflowArtifactsFromLegacyRuntime('legacy-runtime')
    const store = useWorkflowArtifactsStore(pinia)

    expect(synced).toBe(true)
    expect(store.currentRun?.datasetMeta).toMatchObject({
      csv_path: 'C:/data/sample.csv',
      timeframe: '15m',
      base_timeframe: '1m',
      derived_timeframe: '15m',
      resample_applied: true,
    })
    expect(store.currentRun?.mlSignal).toMatchObject({
      stage: 'stage5',
      bundleId: 'bundle-1',
      scoreCachePath: 'C:/runs/score_cache.json',
    })
    expect(store.currentRun?.optimizationMetricSurface).toMatchObject({
      surface: 'trade22',
      fields: ['pnl', 'sharpe'],
    })
    expect(store.currentRun?.tags).toEqual(['alpha', 'beta'])
    expect(refreshAnalysisFromWorkflowRunMock).toHaveBeenCalledTimes(1)
  })

  it('returns false when no artifacts exist', async () => {
    invokeCommandMock.mockResolvedValueOnce([])

    const synced = await syncWorkflowArtifactsFromLegacyRuntime('vue')

    expect(synced).toBe(false)
    expect(invokeCommandMock).toHaveBeenCalledWith('list_run_artifacts')
  })

  it('returns false when invoke throws', async () => {
    invokeCommandMock.mockRejectedValueOnce(new Error('boom'))

    const synced = await syncWorkflowArtifactsFromLegacyRuntime('vue')

    expect(synced).toBe(false)
  })

  it('skips the artifact load when the latest identity is already loaded with validation', async () => {
    const store = useWorkflowArtifactsStore(pinia)
    store.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        artifact_path: 'C:/runs/run.json',
        req: { strategy_kind: 'DoubleMa' },
        res: { rows: 1 },
      },
      currentValidation: {
        ts: '2026-02-25T12:00:00Z',
        req: { candidates: 32 },
        res: { rows: [] },
      },
    })
    invokeCommandMock.mockResolvedValueOnce([{
      ts: '2026-02-25T12:00:00Z',
      path: 'C:/runs/run.json',
    }])

    const synced = await syncWorkflowArtifactsFromLegacyRuntime('legacy-runtime')

    expect(synced).toBe(false)
    expect(store.source).toBe('legacy-runtime')
    expect(refreshAnalysisFromWorkflowRunMock).not.toHaveBeenCalled()
    expect(invokeCommandMock).toHaveBeenCalledTimes(1)
    expect(invokeCommandMock).toHaveBeenCalledWith('list_run_artifacts')
  })

  it('still loads same-path same-ts artifacts when validation is currently absent', async () => {
    const store = useWorkflowArtifactsStore(pinia)
    store.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        artifact_path: 'C:/runs/run.json',
        req: { strategy_kind: 'DoubleMa' },
        res: { rows: 1 },
      },
      currentValidation: null,
    })
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'list_run_artifacts') {
        return [{
          ts: '2026-02-25T12:00:00Z',
          path: 'C:/runs/run.json',
        }]
      }
      if (command === 'load_run_artifact_hydration') {
        return {
          currentRun: {
            ts: '2026-02-25T12:00:00Z',
            artifact_path: 'C:/runs/run.json',
            req: { strategy_kind: 'DoubleMa' },
            res: { rows: 1 },
          },
          currentValidation: {
            ts: '2026-02-25T12:00:00Z',
            req: { candidates: 32 },
            res: { rows: [] },
          },
        }
      }
      return undefined
    })

    const synced = await syncWorkflowArtifactsFromLegacyRuntime('vue')

    expect(synced).toBe(true)
    expect(store.currentValidation?.req).toMatchObject({ candidates: 32 })
    expect(refreshAnalysisFromWorkflowRunMock).toHaveBeenCalledTimes(1)
    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'list_run_artifacts')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'load_run_artifact_hydration', { path: 'C:/runs/run.json' })
  })

  it('reloads same-path same-ts artifacts when summary tags or report paths changed', async () => {
    const store = useWorkflowArtifactsStore(pinia)
    store.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        artifact_path: 'C:/runs/run.json',
        report_xlsx_path: 'C:/runs/old_report.xlsx',
        csv_path: 'C:/data/sample.csv',
        tags: ['alpha'],
        req: { strategy_kind: 'DoubleMa' },
        res: { rows: 1 },
      },
      currentValidation: {
        ts: '2026-02-25T12:00:00Z',
        req: { candidates: 32 },
        res: { rows: [] },
      },
    })
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'list_run_artifacts') {
        return [{
          ts: '2026-02-25T12:00:00Z',
          path: 'C:/runs/run.json',
          report_xlsx_path: 'C:/runs/new_report.xlsx',
          csv_path: 'C:/data/sample.csv',
          tags: ['alpha', 'beta'],
        }]
      }
      if (command === 'load_run_artifact_hydration') {
        return {
          currentRun: {
            ts: '2026-02-25T12:00:00Z',
            artifact_path: 'C:/runs/run.json',
            report_xlsx_path: 'C:/runs/new_report.xlsx',
            csv_path: 'C:/data/sample.csv',
            tags: ['alpha', 'beta'],
            req: { strategy_kind: 'DoubleMa' },
            res: { rows: 1 },
          },
          currentValidation: {
            ts: '2026-02-25T12:00:00Z',
            req: { candidates: 32 },
            res: { rows: [] },
          },
        }
      }
      return undefined
    })

    const synced = await syncWorkflowArtifactsFromLegacyRuntime('legacy-runtime')

    expect(synced).toBe(true)
    expect(store.currentRun?.reportXlsxPath).toBe('C:/runs/new_report.xlsx')
    expect(store.currentRun?.tags).toEqual(['alpha', 'beta'])
    expect(refreshAnalysisFromWorkflowRunMock).toHaveBeenCalledTimes(1)
    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'list_run_artifacts')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'load_run_artifact_hydration', { path: 'C:/runs/run.json' })
  })

  it('returns false when the hydration payload is malformed', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'list_run_artifacts') {
        return [{
          ts: '2026-02-25T12:00:00Z',
          path: 'C:/runs/run.json',
        }]
      }
      if (command === 'load_run_artifact_hydration') {
        return 'invalid-payload'
      }
      return undefined
    })

    const synced = await syncWorkflowArtifactsFromLegacyRuntime('legacy-runtime')
    const store = useWorkflowArtifactsStore(pinia)

    expect(synced).toBe(false)
    expect(store.currentRun).toBeNull()
    expect(store.currentValidation).toBeNull()
    expect(refreshAnalysisFromWorkflowRunMock).not.toHaveBeenCalled()
  })
})
