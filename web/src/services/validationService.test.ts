import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  crossRefresh,
  exportReportJson,
  exportReportXlsx,
  exportShortlistCsv,
  exportValidationJson,
  openReportsDir,
  runValidation,
  toggleCrossDataset,
} from './validationService'
import { invokeCommand } from '../tauri/invoke'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => ({ datasets: [] })),
}))

const invokeCommandMock = vi.mocked(invokeCommand)
let pinia: ReturnType<typeof createPinia>

function installDefaultInvokeCommandMock(): void {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'run_double_ma_validation' || command === 'run_composite_validation') {
      return {
        candidates: [{ score: 1 }],
        runtime_ms: 123,
        cross_ticker: { datasets: [] },
      }
    }
    if (command === 'save_run_artifact') {
      return {
        path: 'runs/vectorbt_run_validation_test.json',
        bytes: 256,
      }
    }
    if (command === 'build_validation_shortlist_csv') {
      return 'rank,fast_len,slow_len'
    }
    if (command === 'save_report_xlsx') {
      return {
        path: 'runs/vectorbt_report_validation_test.xlsx',
      }
    }
    if (command === 'build_report_xlsx') {
      return 'AA=='
    }
    if (command === 'open_path' || command === 'reveal_path') {
      return true
    }
    if (command === 'polygon_get_cache_info') {
      return { datasets: [] }
    }
    return {}
  })
}

function firstValidationRunCall(): [string, Record<string, unknown>] {
  const runCall = invokeCommandMock.mock.calls.find(([command]) =>
    command === 'run_double_ma_validation' || command === 'run_composite_validation',
  )
  expect(runCall).toBeTruthy()
  return [String(runCall?.[0] || ''), (runCall?.[1] ?? {}) as Record<string, unknown>]
}

function firstValidationRequest(): Record<string, unknown> {
  const [, args] = firstValidationRunCall()
  return (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
}

describe('validationService', () => {
  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    useWorkflowArtifactsStore(pinia).clearArtifacts()
    vi.clearAllMocks()
    installDefaultInvokeCommandMock()
    if (typeof window !== 'undefined') {
      window.__VTA_LEGACY_BOOTED__ = false
    }
  })

  it('builds validation request from workflow artifacts and runs direct validation command', async () => {
    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '32',
      seed: '1',
      minPsr: '',
      minDsr: '',
      wfEnable: false,
      wfWindow: '20000',
      wfStep: '20000',
      perturbEnable: false,
      perturbIters: '8',
      perturbMin: '0.015',
      perturbMax: '0.06',
      mcEnable: false,
      mcSims: '500',
      cscvEnable: false,
      cscvSlices: '',
      crossEnable: true,
      crossMax: '2',
      crossDatasets: [
        { path: 'C:/data/a.csv', label: 'A', checked: true },
      ],
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-123',
          ma_source: 'close',
          fast_ma_types: ['sma'],
          slow_ma_types: ['alma'],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'sma',
                slow_ma_type: 'alma',
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const [command, args] = firstValidationRunCall()
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    expect(command).toBe('run_double_ma_validation')
    expect(req.data_id).toBe('data-123')
    expect(req.max_candidates).toBe(32)
    expect(req.seed).toBe(1)
    expect(req.cross_ticker).toEqual({
      max_datasets: 2,
      datasets: [{ csv_path: 'C:/data/a.csv', label: 'A' }],
    })
    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(Array.isArray(validationState.validationColumns)).toBe(true)
    expect(Array.isArray(validationState.validationRows)).toBe(true)
    expect(Number(validationState.tableRowCount || 0)).toBeGreaterThan(0)
    expect(invokeCommandMock).toHaveBeenCalledWith('save_run_artifact', expect.any(Object))

  })

  it('prefers Vue validation store inputs when store source is ready', async () => {
    ;(pinia.state.value as Record<string, unknown>).validation = {
      source: 'vue',
      candidates: '9',
      seed: '7',
      minPsr: '',
      minDsr: '',
      wfEnable: false,
      wfWindow: '12345',
      wfStep: '2345',
      perturbEnable: false,
      perturbIters: '6',
      perturbMin: '0.01',
      perturbMax: '0.05',
      mcEnable: false,
      mcSims: '400',
      cscvEnable: false,
      cscvSlices: '',
      crossEnable: true,
      crossMax: '1',
      crossDatasets: [
        { path: 'C:/data/from_store.csv', label: 'FromStore', checked: true },
      ],
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-store-inputs',
          ma_source: 'close',
          fast_ma_types: ['sma'],
          slow_ma_types: ['ema'],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'sma',
                slow_ma_type: 'ema',
                fast_len: 10,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    })
    const fields: Record<string, { value?: string; checked?: boolean }> = {
      valCandidates: { value: '32' },
      valSeed: { value: '1' },
      valCrossEnable: { checked: false },
      valCrossMax: { value: '6' },
    }
    vi.stubGlobal('document', {
      getElementById: (id: string) => fields[id] ?? null,
    })
    await runValidation({ switchTab: false })

    const req = firstValidationRequest()
    expect(req.max_candidates).toBe(9)
    expect(req.seed).toBe(7)
    expect(req.cross_ticker).toEqual({
      max_datasets: 1,
      datasets: [{ csv_path: 'C:/data/from_store.csv', label: 'FromStore' }],
    })

    vi.unstubAllGlobals()
  })

  it('throws when build context is unavailable and no run snapshot exists', async () => {
    await expect(runValidation()).rejects.toThrow('Run an optimization first.')

    expect(invokeCommandMock).not.toHaveBeenCalledWith('run_double_ma_validation', expect.anything())
    expect(invokeCommandMock).not.toHaveBeenCalledWith('run_composite_validation', expect.anything())
  })

  it('prefers run snapshot top rows from workflow artifact store', async () => {
    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '16',
      seed: '2',
      minPsr: '',
      minDsr: '',
      wfEnable: false,
      wfWindow: '20000',
      wfStep: '20000',
      perturbEnable: false,
      perturbIters: '8',
      perturbMin: '0.015',
      perturbMax: '0.06',
      mcEnable: false,
      mcSims: '500',
      cscvEnable: false,
      cscvSlices: '',
      crossEnable: false,
      crossMax: '0',
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-store-1',
          ma_source: 'close',
          fast_ma_types: ['sma'],
          slow_ma_types: ['ema'],
          strategy: { fee_bps: 2 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'sma',
                slow_ma_type: 'ema',
                fast_len: 8,
                slow_len: 32,
              },
            },
          ],
        },
      },
      currentValidation: null,
    })
    await runValidation({ switchTab: false })

    const req = firstValidationRequest()
    expect(req.data_id).toBe('data-store-1')
    expect(req.max_candidates).toBe(16)
  })

  it('normalizes cross-refresh datasets from polygon cache info', async () => {
    invokeCommandMock.mockResolvedValueOnce({
      datasets: [
        {
          csv_path: ' C:/data/z.csv ',
          display_symbol: 'ZETA',
          market: 'stocks',
          timeframe: '1M',
          min_ts: 1735689600000,
          max_ts: 1735776000000,
          last_updated_utc_ms: 1735862400000,
          row_count: 1000,
          adjusted: true,
        },
        {
          csv_path: 'C:/data/a.csv',
          polygon_ticker: 'ALPHA',
          market: 'crypto',
          timeframe: '5m',
          min_ts: 1735689600000,
          max_ts: 1735776000000,
          row_count: 0,
        },
        {
          csv_path: '',
          display_symbol: 'SKIP',
        },
      ],
    })

    const summary = await crossRefresh()
    expect(invokeCommandMock).toHaveBeenCalledWith('polygon_get_cache_info')
    expect(summary.datasets).toHaveLength(2)
    const [first, second] = summary.datasets
    expect(first).toBeDefined()
    expect(second).toBeDefined()
    expect(first!).toMatchObject({
      path: 'C:/data/a.csv',
      label: 'ALPHA',
      market: 'crypto',
      timeframe: '5m',
      adjusted: null,
    })
    expect(first!.meta).toContain('crypto')
    expect(first!.meta).toContain('rows=')
    expect(second!).toMatchObject({
      path: 'C:/data/z.csv',
      label: 'ZETA',
      market: 'stocks',
      timeframe: '1m',
      adjusted: true,
    })
    expect(second!.meta).toContain('stocks (adj)')
    expect(second!.meta).toContain('updated=')
  })

  it('normalizes cross toggle payload and updates Vue fallback selection', async () => {
    ;(pinia.state.value as Record<string, unknown>).validation = {
      crossDatasets: [
        { path: 'C:/data/msft.csv', label: 'MSFT', checked: true, disabled: false, meta: '—' },
        { path: 'C:/data/aapl.csv', label: 'AAPL', checked: false, disabled: false, meta: '—' },
      ],
      crossSelectedCount: 1,
      crossTotalCount: 2,
      source: 'vue',
    }

    await toggleCrossDataset(' C:/data/aapl.csv ', true)
    await toggleCrossDataset('C:/data/msft.csv', false)
    await toggleCrossDataset('', false)

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    const rows = Array.isArray(validationState.crossDatasets)
      ? validationState.crossDatasets as Array<Record<string, unknown>>
      : []
    expect(validationState.crossSelectedCount).toBe(1)
    expect(rows).toEqual([
      expect.objectContaining({ path: 'C:/data/msft.csv', checked: false }),
      expect.objectContaining({ path: 'C:/data/aapl.csv', checked: true }),
    ])
  })

  it('keeps validation export flows command-driven and reports directory command direct', async () => {
    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:34:56Z',
        artifact_path: 'runs/vectorbt_run_validation_test.json',
        report_xlsx_path: '',
        csv_path: 'C:/data/test.csv',
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-export-test',
          ma_source: 'close',
          fast_ma_types: ['sma'],
          slow_ma_types: ['ema'],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'sma',
                slow_ma_type: 'ema',
                fast_len: 10,
                slow_len: 40,
              },
            },
          ],
        },
      },
      currentValidation: {
        ts: '2026-02-25T12:34:56Z',
        req: {
          data_id: 'data-export-test',
          candidates: [
            {
              fast_ma_type: 'sma',
              slow_ma_type: 'ema',
              fast_len: 10,
              slow_len: 40,
            },
          ],
        },
        res: {
          candidates: [{ score: 1 }],
        },
      },
    })

    await exportShortlistCsv()
    await exportValidationJson()
    await exportReportJson()
    await exportReportXlsx()
    await openReportsDir()

    expect(invokeCommandMock).toHaveBeenCalledWith('build_validation_shortlist_csv', expect.any(Object))
    expect(invokeCommandMock).toHaveBeenCalledWith('save_report_xlsx', expect.any(Object))
    expect(invokeCommandMock).toHaveBeenCalledWith('open_reports_dir')
  })
})
