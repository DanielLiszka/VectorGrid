import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { invokeCommand } from '../tauri/invoke'
import { runBatch } from './metaService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useOptimizeStore } from '../stores/optimizeStore'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

vi.mock('../tauri/events', () => ({
  listenEvent: vi.fn(async () => () => undefined),
}))

vi.mock('./optimizeService', () => ({
  buildOptimizationRequestFromVueStores: vi.fn(async () => ({
    strategy_kind: 'DoubleMa',
    data_id: 'data-1',
    ma_source: 'close',
  })),
}))

const invokeCommandMock = vi.mocked(invokeCommand)
const supportedCampaignStrategies = [
  'DoubleMa',
  'Pit',
  'Set',
  'Mts',
  'Orc',
  'Marrt',
  'Vfct',
  'Rocb',
  'Vcet',
  'Mkcs',
  'Kqsr',
  'Carm',
  'Kvdf',
  'Cdmc',
  'Asr',
  'Vcc',
  'Vemr',
  'CompositeCpu',
  'CompositeMo',
  'CompositeTrend',
  'RegimeHybrid',
  'BreakoutMomentum',
] as const

function summaryFor(strategyKind: string) {
  return {
    version: 1,
    kind: 'vectorgrid_campaign_summary',
    ts: '2026-04-03T12:00:00Z',
    state: 'completed',
    strategy_kind: strategyKind,
    counts: {
      total: 1,
      optimized: 1,
      validated: 1,
      skipped: 0,
      failed: 0,
      cancelled: false,
    },
    items: [],
    warnings: [],
    runtime_ms: 10,
    tags: ['campaign'],
    holdout_pct: 0,
    request: {
      strategy_kind: strategyKind,
      datasets: [{ csv_path: 'C:/a.csv', label: 'AAPL', timeframe: '1m' }],
    },
  }
}

async function invokeDefaultMock(command: string, args?: Record<string, unknown>): Promise<unknown> {
  if (command === 'polygon_get_cache_info') {
    return { datasets: [] }
  }
  if (command === 'list_repo_data_csvs') {
    return {
      dir: 'C:/repo-data',
      truncated: false,
      csv_entries: [
        {
          csv_path: 'C:/a.csv',
          label: 'AAPL',
          timeframe: '1m',
          market: 'local',
          source: 'repo_data',
        },
      ],
    }
  }
  if (command === 'run_research_campaign') {
    const req = (args?.req ?? {}) as Record<string, unknown>
    const strategyKind = String(req.strategy_kind ?? 'DoubleMa')
    return {
      summary_json_path: 'C:/runs/vectorgrid_campaign_1.json',
      summary: summaryFor(strategyKind),
    }
  }
  if (command === 'list_run_artifacts') return []
  if (command === 'list_artifact_library') {
    return { meta: [], stage5: [], policyEval: [] }
  }
  return undefined
}

describe('meta-service campaign strategy routing', () => {
  beforeEach(() => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    invokeCommandMock.mockReset()
    invokeCommandMock.mockImplementation(invokeDefaultMock)
  })

  it.each(supportedCampaignStrategies)(
    'passes %s unchanged into the backend research campaign request',
    async (strategyKind) => {
      const optimizeStore = useOptimizeStore()
      optimizeStore.strategyKind = strategyKind
      optimizeStore.updatedAtUnixMs = Date.now()

      await runBatch({
        request: {
          selected_paths: ['C:/a.csv'],
          holdout_pct: 0,
          holdout_tag: 'holdout',
          auto_meta: false,
          skip_done: false,
          tags: ['campaign'],
          optimization_template: {
            strategy_kind: strategyKind,
            data_id: 'data-1',
            ma_source: 'close',
          },
          validation_config: {
            max_candidates: 8,
            seed: 1,
          },
        },
      })

      const call = invokeCommandMock.mock.calls.find(
        ([command]) => command === 'run_research_campaign',
      )
      expect(call).toBeTruthy()
      expect(call?.[1]).toMatchObject({
        req: {
          strategy_kind: strategyKind,
        },
      })
    },
  )

  it('passes the selected preload policy through to the backend campaign request', async () => {
    const optimizeStore = useOptimizeStore()
    optimizeStore.strategyKind = 'DoubleMa'
    optimizeStore.updatedAtUnixMs = Date.now()

    await runBatch({
      request: {
        selected_paths: ['C:/a.csv'],
        holdout_pct: 0,
        holdout_tag: 'holdout',
        auto_meta: false,
        auto_stage5: false,
        skip_done: false,
        tags: ['campaign'],
        preload_policy: 'host_plus_doublema_vram',
        optimization_template: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-1',
          ma_source: 'close',
        },
        validation_config: {
          max_candidates: 8,
          seed: 1,
        },
      },
    })

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === 'run_research_campaign',
    )
    expect(call).toBeTruthy()
    expect(call?.[1]).toMatchObject({
      req: {
        preload_policy: 'host_plus_doublema_vram',
      },
    })
    expect(useAnalysisStore().batchPreloadPolicy).toBe(
      'host_plus_doublema_vram',
    )
  })

  it('passes validate_only through to the backend campaign request', async () => {
    const optimizeStore = useOptimizeStore()
    optimizeStore.strategyKind = 'DoubleMa'
    optimizeStore.updatedAtUnixMs = Date.now()

    await runBatch({
      request: {
        selected_paths: ['C:/a.csv'],
        holdout_pct: 0,
        holdout_tag: 'holdout',
        execution_mode: 'validate_only',
        auto_meta: true,
        auto_stage5: true,
        skip_done: true,
        tags: ['campaign'],
        preload_policy: 'host_only',
        optimization_template: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-1',
          ma_source: 'close',
        },
        validation_config: {
          max_candidates: 8,
          seed: 1,
        },
      },
    })

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === 'run_research_campaign',
    )
    expect(call).toBeTruthy()
    expect(call?.[1]).toMatchObject({
      req: {
        execution_mode: 'validate_only',
        auto_meta: true,
        auto_stage5: true,
      },
    })
    expect(useAnalysisStore().batchExecutionMode).toBe('validate_only')
  })

  it('suppresses meta and stage5 follow-up payloads for optimize_only mode', async () => {
    const optimizeStore = useOptimizeStore()
    optimizeStore.strategyKind = 'DoubleMa'
    optimizeStore.updatedAtUnixMs = Date.now()

    await runBatch({
      request: {
        selected_paths: ['C:/a.csv'],
        holdout_pct: 0,
        holdout_tag: 'holdout',
        execution_mode: 'optimize_only',
        auto_meta: true,
        auto_stage5: true,
        skip_done: false,
        tags: ['campaign'],
        preload_policy: 'host_only',
        optimization_template: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-1',
          ma_source: 'close',
        },
        validation_config: {
          max_candidates: 8,
          seed: 1,
        },
      },
    })

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === 'run_research_campaign',
    )
    expect(call).toBeTruthy()
    expect(call?.[1]).toMatchObject({
      req: {
        execution_mode: 'optimize_only',
        auto_meta: false,
        auto_stage5: false,
      },
    })
    expect(useAnalysisStore().batchExecutionMode).toBe('optimize_only')
    expect(useAnalysisStore().batchAutoMeta).toBe(false)
    expect(useAnalysisStore().batchAutoStage5).toBe(false)
  })

  it('rejects MlSignal before invoking the backend campaign runner', async () => {
    const optimizeStore = useOptimizeStore()
    optimizeStore.strategyKind = 'MlSignal'
    optimizeStore.updatedAtUnixMs = Date.now()

    await runBatch({
      request: {
        selected_paths: ['C:/a.csv'],
        holdout_pct: 0,
        holdout_tag: 'holdout',
        auto_meta: false,
        skip_done: false,
        tags: ['campaign'],
      },
    })

    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === 'run_research_campaign',
      ),
    ).toBe(false)
    expect(useAnalysisStore().statusLine).toContain(
      'do not support MlSignal',
    )
  })

  it('rejects DynamicDoubleMa before invoking the backend campaign runner', async () => {
    const optimizeStore = useOptimizeStore()
    optimizeStore.strategyKind = 'DynamicDoubleMa'
    optimizeStore.updatedAtUnixMs = Date.now()

    await runBatch({
      request: {
        selected_paths: ['C:/a.csv'],
        holdout_pct: 0,
        holdout_tag: 'holdout',
        auto_meta: false,
        skip_done: false,
        tags: ['campaign'],
      },
    })

    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === 'run_research_campaign',
      ),
    ).toBe(false)
    expect(useAnalysisStore().statusLine).toContain(
      'do not support DynamicDoubleMa',
    )
  })

  it('rejects unsupported future strategies before invoking the backend campaign runner', async () => {
    const optimizeStore = useOptimizeStore()
    optimizeStore.strategyKind = 'UnknownFutureStrategy'
    optimizeStore.updatedAtUnixMs = Date.now()

    await runBatch({
      request: {
        selected_paths: ['C:/a.csv'],
        holdout_pct: 0,
        holdout_tag: 'holdout',
        auto_meta: false,
        skip_done: false,
        tags: ['campaign'],
      },
    })

    expect(
      invokeCommandMock.mock.calls.some(
        ([command]) => command === 'run_research_campaign',
      ),
    ).toBe(false)
    expect(useAnalysisStore().statusLine).toContain(
      "do not support strategy 'UnknownFutureStrategy'",
    )
  })
})
