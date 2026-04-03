import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { buildOptimizationRequestFromVueStores } from './optimizeService'
import {
  batchRefresh,
  batchRevealSummary,
  batchSelectAll,
  batchSelectNone,
  loadPolicyEvalSaved,
  loadSaved,
  loadStage5Saved,
  mlApply,
  mlRefresh,
  mlTrain,
  refreshPolicyEvalSaved,
  refreshSaved,
  refreshStage5Saved,
  runBatch,
  runMlPolicyEvaluation,
  runMeta,
  runStage5,
  toggleBatchDataset,
} from './metaService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'

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
    loadSource: 'csv_parsed_cache_write_skipped',
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
    loadSource: 'derived_fresh_resample',
  })),
}))

const invokeCommandMock = vi.mocked(invokeCommand)
const buildOptimizationRequestMock = vi.mocked(buildOptimizationRequestFromVueStores)
let pinia: ReturnType<typeof createPinia>

function createDeferred<T>(): {
  promise: Promise<T>
  resolve: (value: T) => void
  reject: (reason?: unknown) => void
} {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

async function invokeDefaultMock(command: string): Promise<unknown> {
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
          robust_score_p10: 0.12,
          robust_score_median: 0.18,
          max_dd_p90: 0.09,
          cross_score_median: 0.04,
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
      state: 'healthy',
      model_healthy: true,
      model_summary: {
        label: {
          min_holdout_pass_rate: 0.6,
          min_holdout_score_p10: 0.1,
          min_holdout_datasets: 5,
        },
        meta: {
          healthy: true,
          cv_auc: 0.71,
          cv_brier_calibrated: 0.19,
          trained_rows: 300,
          target: 'holdout_dataset_pass',
        },
      },
      meta_scan_error: null,
    }
  }

  if (command === 'train_ml_confidence_model') {
    return {
      model_path: 'C:/models/meta_model.json',
      model_bytes: 1024,
      model_summary: {
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
      metrics: {
        rows: 300,
        positives: 180,
        negatives: 120,
        target: 'holdout_gate',
        model_kind: 'logistic',
        train_auc: 0.8,
        train_brier: 0.18,
        baseline_brier: 0.24,
        healthy: true,
        warnings: [],
      },
      status: {
        exists: true,
        model_path: 'C:/models/meta_model.json',
        bytes: 1024,
        available_meta_artifacts: 10,
        available_labeled_rows: 300,
        stale: false,
        retrain_recommended: false,
        state: 'healthy',
        model_healthy: true,
        model_summary: {
          label: {
            min_holdout_pass_rate: 0.6,
            min_holdout_score_p10: 0.1,
            min_holdout_datasets: 5,
          },
          meta: {
            healthy: true,
            cv_auc: 0.71,
            cv_brier_calibrated: 0.19,
            trained_rows: 320,
            target: 'holdout_dataset_pass',
          },
        },
        meta_scan_error: null,
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
  if (command === 'run_research_campaign') {
    return {
      summary_json_path: 'C:/runs/vectorgrid_campaign_1.json',
      summary: {
        version: 1,
        kind: 'vectorgrid_campaign_summary',
        ts: '2026-04-03T12:00:00Z',
        state: 'completed',
        strategy_kind: 'CompositeMo',
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
        holdout_tag: 'holdout',
      },
    }
  }
  if (command === 'list_artifact_library') {
    return {
      meta: [],
      stage5: [],
      policyEvals: [],
    }
  }
  return undefined
}

function mockInvokeDefaults(): void {
  invokeCommandMock.mockImplementation(invokeDefaultMock)
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

  it('runs meta with auto-built request via direct tauri invoke without waiting on saved refresh', async () => {
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

    const deferredSaved = createDeferred<unknown>()
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'list_artifact_library') return deferredSaved.promise
      return invokeDefaultMock(command)
    })

    const runPromise = runMeta()
    await Promise.resolve()
    await new Promise((resolve) => setTimeout(resolve, 0))

    expect(analysisStore.metaSavedSelected).toBe('C:/runs/vectorgrid_meta.json')
    expect(analysisStore.aggregateSummarySavedMeta).toBe('1')

    deferredSaved.resolve({
      meta: [],
      stage5: [],
      policyEvals: [],
    })
    await runPromise

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'run_meta_selection')
    expect(call).toBeTruthy()
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.strategy_kind).toBe('CompositeMo')
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.top_k).toBe(25)
    expect(invokeCommandMock.mock.calls.some(([command]) => command === 'list_artifact_library')).toBe(true)
  })

  it('runs stage5 with auto-built request without waiting on saved refresh or redundant ML status refresh', async () => {
    const analysisStore = useAnalysisStore(pinia)
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
      metaStage4TopK: '15',
      metaStage4MinConf: '0.58',
    })

    await runMeta()
    await mlRefresh()
    invokeCommandMock.mockClear()

    const deferredSaved = createDeferred<unknown>()
    invokeCommandMock.mockImplementation(async (command: string) => {
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
      if (command === 'list_artifact_library') return deferredSaved.promise
      return invokeDefaultMock(command)
    })

    let settled = false
    const runPromise = runStage5().then(() => {
      settled = true
    })
    await Promise.resolve()
    await Promise.resolve()

    expect(settled).toBe(true)
    expect(analysisStore.stage4SavedSelected).toBe('C:/runs/vectorgrid_stage5.json')
    expect(invokeCommandMock.mock.calls.filter(([command]) => command === 'ml_confidence_model_status')).toHaveLength(0)

    deferredSaved.resolve({
      meta: [],
      stage5: [],
      policyEvals: [],
    })
    await runPromise

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'run_meta_stage5_ml_confidence')
    expect(call).toBeTruthy()
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.strategy_kind).toBe('CompositeMo')
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.top_k).toBe(15)
    expect((call?.[1] as { req?: { source_meta_path?: string | null } }).req?.source_meta_path).toBe('C:/runs/vectorgrid_meta.json')
    expect(invokeCommandMock.mock.calls.some(([command]) => command === 'list_artifact_library')).toBe(true)
  })

  it('runs ML policy evaluation with auto-built request without waiting on saved refresh', async () => {
    const analysisStore = useAnalysisStore(pinia)
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
      metaStage4TopK: '15',
      metaStage4MinConf: '0.58',
    })

    await runMeta()
    invokeCommandMock.mockClear()

    const deferredSaved = createDeferred<unknown>()
    invokeCommandMock.mockImplementation(async (command: string) => {
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
            best_policy: 'stage5_blended',
            stage5_vs_meta_precision_delta: 0.1,
            stage5_vs_ml_precision_delta: 0.2,
            warnings: [],
          },
        }
      }
      if (command === 'list_artifact_library') return deferredSaved.promise
      return invokeDefaultMock(command)
    })

    let settled = false
    const runPromise = runMlPolicyEvaluation().then(() => {
      settled = true
    })
    await Promise.resolve()
    await Promise.resolve()

    expect(settled).toBe(true)
    expect(analysisStore.policyEvalSavedSelected).toBe('C:/runs/vectorgrid_ml_policy_eval.json')

    deferredSaved.resolve({
      meta: [],
      stage5: [],
      policyEvals: [],
    })
    await runPromise

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'evaluate_ml_confidence_policy')
    expect(call).toBeTruthy()
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.strategy_kind).toBe('CompositeMo')
    expect((call?.[1] as { req?: { strategy_kind?: string; top_k?: number } }).req?.top_k).toBe(15)
    expect(invokeCommandMock.mock.calls.some(([command]) => command === 'list_artifact_library')).toBe(true)

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

  it('uses summary-only ML status payloads and minimal apply DTOs on direct command path', async () => {
    const analysisStore = useAnalysisStore(pinia)
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
      metaUseMlConfidence: false,
    })

    await runMeta()
    await mlRefresh()
    expect(analysisStore.metaMlStatus).toContain('cv_auc=0.710')

    invokeCommandMock.mockClear()
    await mlTrain()

    expect(invokeCommandMock.mock.calls.filter(([command]) => command === 'ml_confidence_model_status')).toHaveLength(0)
    expect(analysisStore.metaMlStatus).toContain('trained_rows=320')
    expect(invokeCommandMock).toHaveBeenCalledWith('train_ml_confidence_model', {
      scope: { strategy_kind: 'CompositeMo', timeframe: '1m' },
      cfg: null,
    })

    analysisStore.metaUseMlConfidence = true
    invokeCommandMock.mockClear()
    await mlApply()

    const applyCall = invokeCommandMock.mock.calls.find(([command]) => command === 'apply_ml_confidence_model')
    expect(applyCall?.[1]).toEqual({
      scope: { strategy_kind: 'CompositeMo', timeframe: '1m' },
      groups: [{
        samples: 10,
        pass_rate: 0.55,
        robust_score_p10: 0.12,
        robust_score_median: 0.18,
        max_dd_p90: 0.09,
        cross_score_median: 0.04,
        meta_score: 0.8,
      }],
    })
  })

  it('includes meta scan errors in formatted ML status', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'ml_confidence_model_status') {
        return {
          exists: false,
          state: 'missing',
          available_meta_artifacts: 7,
          available_labeled_rows: 140,
          meta_scan_error: 'scan failed',
        }
      }
      return invokeDefaultMock(command)
    })

    const analysisStore = useAnalysisStore(pinia)
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
    })

    await mlRefresh()

    expect(analysisStore.metaMlStatus).toContain('meta_scan_error=scan failed')
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
      groups: [{
        samples: 10,
        pass_rate: 0.55,
        robust_score_p10: 0.12,
        robust_score_median: 0.18,
        max_dd_p90: 0.09,
        cross_score_median: 0.04,
        meta_score: 0.8,
      }],
    })
  })

  it('refreshes ML status after stage5 auto-trains a model', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_meta_selection') {
        return {
          ts: '2026-02-26T10:00:00Z',
          report_xlsx_path: 'C:/reports/meta.xlsx',
          meta_json_path: 'C:/runs/vectorgrid_meta.json',
          result: {
            strategy_kind: 'CompositeMo',
            timeframe: '1m',
            groups: [{
              samples: 10,
              pass_rate: 0.55,
              robust_score_p10: 0.12,
              robust_score_median: 0.18,
              max_dd_p90: 0.09,
              cross_score_median: 0.04,
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
            model_trained_now: true,
            groups_selected: 1,
            groups_scored: 1,
            groups_considered: 1,
            min_ml_confidence: 0.55,
            decision_counts: { accept: 1, review: 0, reject: 0 },
            selected_groups: [],
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
          model_healthy: true,
          model_summary: {
            label: { min_holdout_pass_rate: 0.6, min_holdout_score_p10: 0.1, min_holdout_datasets: 5 },
            meta: { healthy: true },
          },
        }
      }
      if (command === 'list_artifact_library') {
        return {
          meta: [],
          stage5: [],
          policyEvals: [],
        }
      }
      return undefined
    })

    const analysisStore = useAnalysisStore(pinia)
    analysisStore.applyLegacySnapshot({
      metaStrategyKind: 'CompositeMo',
      metaTimeframe: '1m',
    })

    await runMeta()
    await mlRefresh()
    invokeCommandMock.mockClear()
    await runStage5()
    await Promise.resolve()
    await Promise.resolve()

    expect(invokeCommandMock).toHaveBeenCalledWith('ml_confidence_model_status', {
      scope: { strategy_kind: 'CompositeMo', timeframe: '1m' },
    })
  })

  it('refreshes saved artifact lists from artifact library snapshot', async () => {
    const analysisStore = useAnalysisStore(pinia)
    analysisStore.metaSavedSelected = 'C:/runs/meta_saved.json'
    analysisStore.stage4SavedSelected = 'C:/runs/stage5_saved.json'
    analysisStore.policyEvalSavedSelected = 'C:/runs/policy_saved.json'
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'list_artifact_library') {
        return {
          meta: [{ path: 'C:/runs/meta_saved.json', ts: '2026-02-26T10:00:00Z' }],
          stage5: [{ path: 'C:/runs/stage5_saved.json', ts: '2026-02-26T10:02:00Z' }],
          policyEvals: [{ path: 'C:/runs/policy_saved.json', ts: '2026-02-26T10:03:00Z' }],
        }
      }
      return invokeDefaultMock(command)
    })

    await refreshSaved()
    await refreshStage5Saved()
    await refreshPolicyEvalSaved()

    expect(analysisStore.metaSavedSelected).toBe('C:/runs/meta_saved.json')
    expect(analysisStore.stage4SavedSelected).toBe('C:/runs/stage5_saved.json')
    expect(analysisStore.policyEvalSavedSelected).toBe('C:/runs/policy_saved.json')
    expect(
      invokeCommandMock.mock.calls.filter(([command]) => command === 'list_artifact_library'),
    ).toHaveLength(3)
  })

  it('loads saved meta/stage5/policy-eval replays through typed artifact commands', async () => {
    const analysisStore = useAnalysisStore(pinia)
    analysisStore.metaSavedSelected = 'C:/runs/meta_saved.json'
    analysisStore.stage4SavedSelected = 'C:/runs/stage5_saved.json'
    analysisStore.policyEvalSavedSelected = 'C:/runs/policy_saved.json'
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'list_artifact_library') {
        return {
          meta: [{
            path: 'C:/runs/meta_saved.json',
            ts: '2026-02-26T10:00:00Z',
            report_xlsx_path: 'C:/reports/meta_saved.xlsx',
          }],
          stage5: [{ path: 'C:/runs/stage5_saved.json', ts: '2026-02-26T10:02:00Z' }],
          policyEvals: [{ path: 'C:/runs/policy_saved.json', ts: '2026-02-26T10:03:00Z' }],
        }
      }
      if (command === 'load_meta_artifact_replay') {
        return {
          ts: '2026-02-26T10:00:00Z',
          meta_json_path: 'C:/runs/meta_saved.json',
          report_xlsx_path: 'C:/reports/meta_saved.xlsx',
          result: {
            strategy_kind: 'CompositeMo',
            timeframe: '1m',
            groups: [],
          },
        }
      }
      if (command === 'load_stage5_artifact_replay') {
        return {
          ts: '2026-02-26T10:02:00Z',
          stage5_json_path: 'C:/runs/stage5_saved.json',
          result: {
            strategy_kind: 'CompositeMo',
            timeframe: '1m',
            selected_groups: [],
          },
        }
      }
      if (command === 'load_ml_policy_eval_artifact_replay') {
        return {
          ts: '2026-02-26T10:03:00Z',
          eval_json_path: 'C:/runs/policy_saved.json',
          result: {
            strategy_kind: 'CompositeMo',
            timeframe: '1m',
            stage5_blended: { precision_at_k: 1 },
          },
        }
      }
      return invokeDefaultMock(command)
    })

    await refreshSaved()
    await loadSaved()
    await refreshStage5Saved()
    await loadStage5Saved()
    await refreshPolicyEvalSaved()
    await loadPolicyEvalSaved()

    expect(invokeCommandMock).toHaveBeenCalledWith('load_meta_artifact_replay', {
      path: 'C:/runs/meta_saved.json',
    })
    expect(invokeCommandMock).toHaveBeenCalledWith('load_stage5_artifact_replay', {
      path: 'C:/runs/stage5_saved.json',
    })
    expect(invokeCommandMock).toHaveBeenCalledWith('load_ml_policy_eval_artifact_replay', {
      path: 'C:/runs/policy_saved.json',
    })
    expect(analysisStore.statusLine).toContain('Loaded policy evaluation')
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
    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-26T10:00:00Z',
        artifact_path: 'C:/runs/vectorgrid_run.json',
        csv_path: 'C:/a.csv',
        dataset_meta: {
          csv_path: 'C:/a.csv',
          timeframe: '1m',
          label: 'AAPL',
        },
        ml_signal: {
          stage: 'stage5',
          model_summary: 'cached model',
        },
        tags: ['existing'],
        req: {
          strategy_kind: 'CompositeMo',
        },
        res: {
          top: [],
        },
      },
      currentValidation: null,
    })

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

    expect(invokeCommandMock).toHaveBeenCalledWith(
      'run_research_campaign',
      expect.objectContaining({
        req: expect.objectContaining({
          strategy_kind: 'CompositeMo',
          holdout_pct: 10,
          holdout_tag: 'holdout',
          auto_meta: false,
          tags: ['batch-1'],
          datasets: [
            expect.objectContaining({
              csv_path: 'C:/a.csv',
              label: 'AAPL',
              timeframe: '1m',
            }),
          ],
        }),
      }),
    )
    expect(
      invokeCommandMock.mock.calls.some(([command]) => command === 'save_batch_summary'),
    ).toBe(false)

    const analysisStore = useAnalysisStore(pinia)
    expect(analysisStore.canBatchRevealSummary).toBe(true)
    expect(analysisStore.batchOutputPreview).toContain('vectorgrid_campaign_summary')
    expect(validationStore.autoRun).toBe(true)
    expect(validationStore.autoExportXlsx).toBe(true)
    expect(validationStore.autoRevealReport).toBe(true)
    const workflowArtifacts = useWorkflowArtifactsStore(pinia)
    expect(workflowArtifacts.currentRun?.datasetMeta).toMatchObject({
      csv_path: 'C:/a.csv',
      timeframe: '1m',
      label: 'AAPL',
    })
    expect(workflowArtifacts.currentRun?.mlSignal).toMatchObject({
      stage: 'stage5',
    })
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

    expect(invokeCommandMock).toHaveBeenCalledWith('reveal_path', {
      path: 'C:/runs/vectorgrid_campaign_1.json',
    })
  })
})
