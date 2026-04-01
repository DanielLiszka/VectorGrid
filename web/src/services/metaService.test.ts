import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { bindRuntimePinia } from '../stores/runtimePinia'
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
  runMlPolicyEvaluation,
  runMeta,
  runStage5,
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
let pinia: ReturnType<typeof createPinia>

function mockInvokeDefaults(): void {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'run_meta_selection') {
      return {
        ts: '2026-02-26T10:00:00Z',
        report_xlsx_path: 'C:/reports/meta.xlsx',
        meta_json_path: 'C:/runs/vectorgrid_meta.json',
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

    if (command === 'run_meta_stage5_ml_confidence') {
      return {
        ts: '2026-02-26T10:02:00Z',
        stage4_json_path: 'C:/runs/vectorgrid_stage5.json',
        stage5_json_path: 'C:/runs/vectorgrid_stage5.json',
        result: {
          strategy_kind: 'CompositeMo',
          timeframe: '1m',
          source_meta_json_path: 'C:/runs/vectorgrid_meta.json',
          model_path: 'C:/models/meta_model.json',
          model_state: 'healthy',
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

    if (command === 'evaluate_ml_confidence_policy') {
      return {
        ts: '2026-02-26T10:03:00Z',
        eval_json_path: 'C:/runs/vectorgrid_ml_policy_eval.json',
        result: {
          strategy_kind: 'CompositeMo',
          timeframe: '1m',
          artifacts_considered: 4,
          artifacts_evaluated: 3,
          artifacts_skipped_no_history: 1,
          artifacts_skipped_training_failure: 0,
          artifacts_skipped_unhealthy_model: 0,
          artifacts_skipped_no_labels: 0,
          top_k: 15,
          min_ml_confidence: 0.58,
          meta_weight: 0.6,
          ml_weight: 0.35,
          uncertainty_weight: 0.05,
          meta_only: { selected_total: 3, positives_total: 2, precision_at_k: 2 / 3 },
          ml_only: { selected_total: 3, positives_total: 1, precision_at_k: 1 / 3 },
          stage5_blended: { selected_total: 3, positives_total: 3, precision_at_k: 1.0 },
          warnings: [],
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
    if (command === 'save_batch_summary') return { path: 'C:/runs/vectorgrid_batch_1.json' }
    if (command === 'list_meta_artifacts') return []
    if (command === 'list_stage5_artifacts') return []
    if (command === 'list_ml_policy_eval_artifacts') return []
    return undefined
  })
}

describe('meta service', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.unstubAllGlobals()
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    mockInvokeDefaults()
  })

  it('runs meta with auto-built request via direct tauri invoke', async () => {
    const analysisStore = useAnalysisStore(pinia)
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
      metaTopK: '25',
      metaMinSamples: '30',
    })
    const validationStore = useValidationStore(pinia)
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

  it('runs stage5 with auto-built request via direct invoke', async () => {
    const analysisStore = useAnalysisStore(pinia)
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
      metaStage4TopK: '15',
      metaStage4MinConf: '0.58',
    })

    await runMeta()
    await runStage5()

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'run_meta_stage5_ml_confidence')
    expect(call).toBeTruthy()
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.strategy_kind).toBe('CompositeMo')
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.top_k).toBe(15)
    expect((call?.[1] as { req?: { source_meta_path?: string | null } }).req?.source_meta_path).toBe('C:/runs/vectorgrid_meta.json')
  })

  it('runs ML policy evaluation with auto-built request via direct invoke', async () => {
    const analysisStore = useAnalysisStore(pinia)
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
      metaStage4TopK: '15',
      metaStage4MinConf: '0.58',
    })

    await runMeta()
    await runMlPolicyEvaluation()

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'evaluate_ml_confidence_policy')
    expect(call).toBeTruthy()
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.strategy_kind).toBe('CompositeMo')
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.top_k).toBe(15)

    expect(analysisStore.metaPolicyEvalStatus).toContain('Policy evaluation complete')
    expect(analysisStore.metaPolicyEvalOutputPreview).toContain('stage5_blended')
  })

  it('preserves Asr strategy kind for meta selection request', async () => {
    const analysisStore = useAnalysisStore(pinia)
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'Asr',
      metaTimeframe: '1m',
      metaTopK: '10',
      metaMinSamples: '20',
    })
    const validationStore = useValidationStore(pinia)
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
    const analysisStore = useAnalysisStore(pinia)
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
    const analysisStore = useAnalysisStore(pinia)
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
    const optimizeStore = useOptimizeStore(pinia)
    optimizeStore.strategyKind = 'CompositeMo'
    optimizeStore.updatedAtUnixMs = Date.now()
    const validationStore = useValidationStore(pinia)
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

    const analysisStore = useAnalysisStore(pinia)
    expect(analysisStore.canBatchRevealSummary).toBe(true)
    expect(analysisStore.batchOutputPreview).toContain('vectorgrid_batch_summary')
    expect(validationStore.autoRun).toBe(true)
    expect(validationStore.autoExportXlsx).toBe(true)
    expect(validationStore.autoRevealReport).toBe(true)
  })

  it('refreshes, selects, clears, toggles, and reveals batch datasets in vue path', async () => {
    const optimizeStore = useOptimizeStore(pinia)
    optimizeStore.strategyKind = 'CompositeMo'
    optimizeStore.updatedAtUnixMs = Date.now()

    await batchRefresh()
    let analysisStore = useAnalysisStore(pinia)
    expect(analysisStore.batchTotalCount).toBeGreaterThan(0)

    await batchSelectAll()
    analysisStore = useAnalysisStore(pinia)
    expect(analysisStore.batchSelectedCount).toBe(analysisStore.batchTotalCount)

    await batchSelectNone()
    analysisStore = useAnalysisStore(pinia)
    expect(analysisStore.batchSelectedCount).toBe(0)

    await toggleBatchDataset('C:/a.csv', true)
    analysisStore = useAnalysisStore(pinia)
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

    expect(invokeCommandMock).toHaveBeenCalledWith('reveal_path', { path: 'C:/runs/vectorgrid_batch_1.json' })
  })
})
