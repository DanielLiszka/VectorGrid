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
})
