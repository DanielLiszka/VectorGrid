import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { buildOptimizationRequestFromVueStores, runOptimization } from './optimizeService'
import { runValidation } from './validationService'
import {
  batchRefresh,
  batchRevealSummary,
  batchSelectAll,
  batchSelectNone,
  mlApply,
  mlRefresh,
  mlTrain,
  runBatch,
  runMeta,
  runStage4,
  toggleBatchDataset,
} from './metaService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useValidationStore } from '../stores/validationStore'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

vi.mock('./optimizeService', () => ({
  buildOptimizationRequestFromVueStores: vi.fn(async () => ({
    strategy_kind: 'CompositeMo',
    data_id: 'data-1',
    ma_source: 'close',
    candidates: [],
  })),
  runOptimization: vi.fn(async () => undefined),
}))

vi.mock('./validationService', () => ({
  runValidation: vi.fn(async () => undefined),
}))

vi.mock('./dataService', () => ({
  loadCsv: vi.fn(async (path: string) => ({
    path,
    dataId: 'data-1',
    meta: {
      len: 1000,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1700000000000,
      maxTs: 1700003600000,
      timeframe: '1m',
      sourceSizeBytes: 123,
      sourceMtimeUtcMs: 1700000000000,
    },
    cacheHit: false,
    cachePath: '',
  })),
  applyTimeframe: vi.fn(async () => ({
    path: '',
    dataId: 'data-1@1m',
    meta: {
      len: 1000,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1700000000000,
      maxTs: 1700003600000,
      timeframe: '1m',
      sourceSizeBytes: null,
      sourceMtimeUtcMs: null,
    },
    cacheHit: false,
    cachePath: '',
  })),
}))

const invokeCommandMock = vi.mocked(invokeCommand)
const runOptimizationMock = vi.mocked(runOptimization)
const runValidationMock = vi.mocked(runValidation)
const buildOptimizationRequestMock = vi.mocked(buildOptimizationRequestFromVueStores)

function mockInvokeDefaults(): void {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'run_meta_selection') {
      return {
        ts: '2026-02-26T10:00:00Z',
        report_xlsx_path: 'C:/reports/meta.xlsx',
        meta_json_path: 'C:/runs/vectorbt_meta.json',
        result: {
          strategy_kind: 'CompositeMo',
          timeframe: '1m',
          artifacts_used: 2,
          artifacts_considered: 3,
          candidates_considered: 40,
          runtime_ms: 123,
          groups: [{
            params: { fast_ma_type: 'ema', fast_len: 10, slow_ma_type: 'ema', slow_len: 20 },
            pass_rate: 0.55,
            samples: 10,
            meta_score: 0.8,
          }],
        },
      }
    }

    if (command === 'run_meta_stage4_experimental') {
      return {
        ts: '2026-02-26T10:02:00Z',
        stage4_json_path: 'C:/runs/vectorbt_stage4.json',
        result: {
          strategy_kind: 'CompositeMo',
          timeframe: '1m',
          source_meta_json_path: 'C:/runs/vectorbt_meta.json',
          model_path: 'C:/models/meta_model.json',
          model_healthy: true,
          model_trained_now: false,
          groups_selected: 1,
          groups_scored: 1,
          groups_considered: 2,
          min_ml_confidence: 0.55,
          decision_counts: { accept: 1, review: 0, reject: 1 },
          selected_groups: [{
            stage4_rank: 1,
            decision_bucket: 'accept',
            final_rank_score: 0.9,
            ml_confidence: 0.8,
            ml_uncertainty: 0.1,
            params: { fast_ma_type: 'ema', fast_len: 10, slow_ma_type: 'ema', slow_len: 20 },
          }],
        },
      }
    }

    if (command === 'ml_confidence_model_status') {
      return {
        exists: true,
        model_path: 'C:/models/meta_model.json',
        bytes: 1024,
        available_meta_artifacts: 10,
        available_labeled_rows: 300,
        stale: false,
        retrain_recommended: false,
        model: {
          label: {
            min_holdout_pass_rate: 0.6,
            min_holdout_score_p10: 0.1,
            min_holdout_datasets: 5,
          },
          meta: {
            healthy: true,
            cv_auc: 0.71,
            cv_brier_calibrated: 0.19,
          },
        },
      }
    }

    if (command === 'apply_ml_confidence_model') {
      return { confidences: [0.88], warnings: [] }
    }

    if (command === 'polygon_get_cache_info') {
      return {
        datasets: [
          {
            csv_path: 'C:/a.csv',
            display_symbol: 'AAPL',
            market: 'stocks',
            timeframe: '1m',
            min_ts: 1700000000000,
            max_ts: 1700003600000,
            last_updated_utc_ms: 1700010000000,
            row_count: 1000,
            adjusted: true,
          },
        ],
      }
    }

    if (command === 'list_repo_data_csvs') {
      return {
        dir: 'C:/repo/data',
        truncated: false,
        csv_entries: [
          {
            csv_path: 'C:/b.csv',
            source_size_bytes: 123,
            source_mtime_utc_ms: 1700020000000,
          },
        ],
      }
    }

    if (command === 'list_run_artifacts') return []
    if (command === 'save_batch_summary') return { path: 'C:/runs/vectorbt_batch_1.json' }
    if (command === 'list_meta_artifacts') return []
    if (command === 'list_stage4_artifacts') return []
    return undefined
  })
}

describe('meta service', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.unstubAllGlobals()
    setActivePinia(createPinia())
    mockInvokeDefaults()
  })

  it('runs meta with auto-built request via direct tauri invoke', async () => {
    const analysisStore = useAnalysisStore()
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
      metaTopK: '25',
      metaMinSamples: '30',
    })
    const validationStore = useValidationStore()
    validationStore.applyLegacySnapshot({
      seed: '1',
      wfWindow: '20000',
      wfStep: '20000',
      perturbEnable: false,
      mcEnable: false,
    })

    await runMeta()

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'run_meta_selection')
    expect(call).toBeTruthy()
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.strategy_kind).toBe('CompositeMo')
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.top_k).toBe(25)
    expect(invokeCommandMock).toHaveBeenCalledWith('list_meta_artifacts')
  })

  it('runs stage4 with auto-built request via direct invoke', async () => {
    const analysisStore = useAnalysisStore()
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
      metaStage4TopK: '15',
      metaStage4MinConf: '0.58',
    })

    await runMeta()
    await runStage4()

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'run_meta_stage4_experimental')
    expect(call).toBeTruthy()
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.strategy_kind).toBe('CompositeMo')
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.top_k).toBe(15)
    expect((call?.[1] as { req?: { source_meta_path?: string | null } }).req?.source_meta_path).toBe('C:/runs/vectorbt_meta.json')
  })

  it('preserves Asr strategy kind for meta selection request', async () => {
    const analysisStore = useAnalysisStore()
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'Asr',
      metaTimeframe: '1m',
      metaTopK: '10',
      metaMinSamples: '20',
    })
    const validationStore = useValidationStore()
    validationStore.applyLegacySnapshot({
      seed: '1',
      wfWindow: '20000',
      wfStep: '20000',
      perturbEnable: false,
      mcEnable: false,
    })

    await runMeta()

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'run_meta_selection')
    expect(call).toBeTruthy()
    expect((call?.[1] as { req?: { strategy_kind?: string } }).req?.strategy_kind).toBe('Asr')
  })

  it('runs ml refresh/train/apply on direct command path', async () => {
    const analysisStore = useAnalysisStore()
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
      metaUseMlConfidence: true,
    })

    await runMeta()
    await mlRefresh()
    await mlTrain()
    await mlApply()

    expect(invokeCommandMock).toHaveBeenCalledWith('ml_confidence_model_status', {
      scope: { strategy_kind: 'CompositeMo', timeframe: '1m' },
    })
    expect(invokeCommandMock).toHaveBeenCalledWith('train_ml_confidence_model', {
      scope: { strategy_kind: 'CompositeMo', timeframe: '1m' },
      cfg: null,
    })
    expect(invokeCommandMock).toHaveBeenCalledWith('apply_ml_confidence_model', {
      scope: { strategy_kind: 'CompositeMo', timeframe: '1m' },
      groups: expect.any(Array),
    })
  })

  it('runs ML autopilot after meta when enabled and ML ranking is active', async () => {
    const analysisStore = useAnalysisStore()
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
      metaUseMlConfidence: true,
      metaMlAutopilot: true,
    })

    await runMeta()

    expect(invokeCommandMock).toHaveBeenCalledWith('ml_confidence_model_status', {
      scope: { strategy_kind: 'CompositeMo', timeframe: '1m' },
    })
    expect(invokeCommandMock).toHaveBeenCalledWith('apply_ml_confidence_model', {
      scope: { strategy_kind: 'CompositeMo', timeframe: '1m' },
      groups: expect.any(Array),
    })
  })

  it('runs batch via vue-owned orchestration path and saves summary', async () => {
    const optimizeStore = useOptimizeStore()
    optimizeStore.strategyKind = 'CompositeMo'
    optimizeStore.updatedAtUnixMs = Date.now()
    const validationStore = useValidationStore()
    validationStore.autoRun = true
    validationStore.autoExportXlsx = true
    validationStore.autoRevealReport = true
    validationStore.updatedAtUnixMs = Date.now()

    buildOptimizationRequestMock.mockResolvedValue({
      strategy_kind: 'CompositeMo',
      data_id: 'data-1',
      ma_source: 'close',
      candidates: [],
    })

    await runBatch({
      request: {
        selected_paths: ['C:/a.csv'],
        holdout_pct: 10,
        holdout_tag: 'holdout',
        auto_meta: false,
        skip_done: false,
        tags: ['batch-1'],
      },
    })

    expect(runOptimizationMock).toHaveBeenCalledWith({ switchTab: false })
    expect(runValidationMock).toHaveBeenCalledWith({ useFiltered: false, switchTab: false })
    expect(invokeCommandMock).toHaveBeenCalledWith('save_batch_summary', { summary: expect.any(Object) })

    const analysisStore = useAnalysisStore()
    expect(analysisStore.canBatchRevealSummary).toBe(true)
    expect(analysisStore.batchOutputPreview).toContain('vectorbt_batch_summary')
    expect(validationStore.autoRun).toBe(true)
    expect(validationStore.autoExportXlsx).toBe(true)
    expect(validationStore.autoRevealReport).toBe(true)
  })

  it('refreshes, selects, clears, toggles, and reveals batch datasets in vue path', async () => {
    const optimizeStore = useOptimizeStore()
    optimizeStore.strategyKind = 'CompositeMo'
    optimizeStore.updatedAtUnixMs = Date.now()

    await batchRefresh()
    let analysisStore = useAnalysisStore()
    expect(analysisStore.batchTotalCount).toBeGreaterThan(0)

    await batchSelectAll()
    analysisStore = useAnalysisStore()
    expect(analysisStore.batchSelectedCount).toBe(analysisStore.batchTotalCount)

    await batchSelectNone()
    analysisStore = useAnalysisStore()
    expect(analysisStore.batchSelectedCount).toBe(0)

    await toggleBatchDataset('C:/a.csv', true)
    analysisStore = useAnalysisStore()
    expect(analysisStore.batchSelectedCount).toBe(1)

    await runBatch({
      request: {
        selected_paths: ['C:/a.csv'],
        holdout_pct: 0,
        holdout_tag: 'holdout',
        auto_meta: false,
        skip_done: false,
        tags: ['batch-2'],
      },
    })
    await batchRevealSummary()

    expect(invokeCommandMock).toHaveBeenCalledWith('reveal_path', { path: 'C:/runs/vectorbt_batch_1.json' })
  })
})
