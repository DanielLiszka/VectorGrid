import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { syncWorkflowArtifactsFromLegacyRuntime } from './workflowArtifactsSync'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeCommandMock = vi.mocked(invokeCommand)
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

      if (command === 'load_run_artifact') {
        return {
          ts: '2026-02-25T12:00:00Z',
          req: { strategy_kind: 'DoubleMa' },
          res: { rows: 1 },
          validation: {
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
    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'list_run_artifacts')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'load_run_artifact', { path: 'C:/runs/run.json' })
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
})
