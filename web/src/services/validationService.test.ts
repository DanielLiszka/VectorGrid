import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  crossRefresh,
  exportReportJson,
  exportReportXlsx,
  exportShortlistCsv,
  exportValidationJson,
  openReportsDir,
  previewValidationExecutionSummaryFromStores,
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
        backend_used: 'Cpu',
        backend_route: 'GpuKernel->CpuOnly',
        backend_fallback_reason: 'GPU validation runtime not implemented yet; using CPU validation.',
        candidates: [{ score: 1 }],
        runtime_ms: 123,
        cross_ticker: { datasets: [] },
        reopt_walk_forward: {
          windows: 3,
          top_n_candidates: 2,
          unique_winners: 2,
          winner_changes: 1,
          winner_param_drift_mean: 0.15,
          winner_param_drift_max: 0.3,
          forward_positive_pnl_frac: 2 / 3,
          forward_sharpe_median: 0.42,
          forward_max_dd_worst: 0.11,
          winner_sequence: [
            JSON.stringify({ fast_ma_type: 'sma', fast_len: 10, slow_ma_type: 'alma', slow_len: 40 }),
            JSON.stringify({ fast_ma_type: 'sma', fast_len: 12, slow_ma_type: 'alma', slow_len: 44 }),
            JSON.stringify({ fast_ma_type: 'sma', fast_len: 12, slow_ma_type: 'alma', slow_len: 44 }),
          ],
        },
        run_verdict: {
          passed: true,
          approved: false,
          candidates_tested: 1,
          candidates_survived_hard_gates: 1,
          approved_candidate_count: 0,
          final_recommendation: 'evidence_too_short_min_track_record',
          top_candidate_track_record_bars: 120,
          top_candidate_min_track_record_length_bars: 240,
        },
        spa: {
          scope: 'validation_candidates',
          candidate_count: 1,
          slice_count: 8,
          bootstrap_samples: 300,
          block_length: 4,
          benchmark: 'buy_hold',
          best_candidate_key: 'sma(10)/alma(40)',
          best_candidate_mean_diff: 0.02,
          best_candidate_t_stat: 1.4,
          p_value: 0.17,
          passed: false,
        },
      }
    }
    if (command === 'save_run_artifact') {
      return {
        path: 'runs/vectorgrid_run_validation_test.json',
        bytes: 256,
      }
    }
    if (command === 'build_validation_shortlist_csv') {
      return 'rank,fast_len,slow_len'
    }
    if (command === 'save_report_xlsx') {
      return {
        path: 'runs/vectorgrid_report_validation_test.xlsx',
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
      spaEnable: true,
      spaSlices: '8',
      spaBootstrapSamples: '300',
      spaBlockLength: '4',
      reoptWfEnable: true,
      reoptWfWindow: '12000',
      reoptWfStep: '3000',
      reoptWfTopN: '5',
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
          backend: { backend: 'GpuKernel', device_id: 0 },
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
    expect(req.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect(req.max_candidates).toBe(32)
    expect(req.seed).toBe(1)
    expect(req.spa).toEqual({
      slices: 8,
      bootstrap_samples: 300,
      block_length: 4,
      benchmark: 'buy_hold',
    })
    expect(req.reopt_walk_forward).toEqual({
      window_bars: 12000,
      step_bars: 3000,
      top_n_candidates: 5,
    })
    expect(req.cross_ticker).toEqual({
      max_datasets: 2,
      mode: 'preserved_finalists',
      datasets: [{ csv_path: 'C:/data/a.csv', label: 'A' }],
    })
    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(Array.isArray(validationState.validationColumns)).toBe(true)
    expect(Array.isArray(validationState.validationRows)).toBe(true)
    expect(Number(validationState.tableRowCount || 0)).toBeGreaterThan(0)
    expect(String(validationState.validationReoptMeta || '')).toContain('reopt_wf:')
    expect(String(validationState.validationMeta || '')).toContain('backend=Cpu')
    expect(String(validationState.validationMeta || '')).toContain('route=GpuKernel->CpuOnly')
    expect(String(validationState.validationMeta || '')).toContain('fallback=GPU validation runtime not implemented yet; using CPU validation.')
    expect(String(validationState.validationRouteSummary || '')).toContain('Cpu • GpuKernel->CpuOnly • fallback: GPU validation runtime not implemented yet; using CPU validation.')
    expect(String(validationState.validationRouteDetailSummary || '')).toContain('GPU validation runtime not implemented yet; using CPU validation.')
    expect(String(validationState.validationAuxCpuSummary || '')).toContain('none')
    expect(String(validationState.validationBootstrapSummary || '')).toContain('not run')
    expect(String(validationState.validationCrossSummary || '')).toContain('not run')
    expect(String(validationState.validationSkippedSummary || '')).toContain('Cross: unavailable')
    expect(String(validationState.validationWarningSummary || '')).toContain('none')
    expect(String(validationState.validationMeta || '')).toContain('spa=0.170 fail')
    expect(String(validationState.validationMeta || '')).toContain('rec=evidence_too_short_min_track_record')
    const saveCall = invokeCommandMock.mock.calls.find(([commandName]) => commandName === 'save_run_artifact')
    const savedArtifact = (saveCall?.[1] as { artifact?: Record<string, unknown> } | undefined)?.artifact
    expect(savedArtifact?.kind).toBe('vectorgrid_run_artifact')
    expect(invokeCommandMock).toHaveBeenCalledWith('save_run_artifact', expect.any(Object))

  })

  it('surfaces skipped-module and run-note metadata in validation summary text', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_double_ma_validation') {
        return {
          backend_used: 'Gpu',
          backend_route: 'GpuKernel',
          candidates: [],
          runtime_ms: 88,
          cscv_pbo: {
            skipped_reason: 'cscv_skipped_not_enough_bars',
          },
          spa: {
            skipped_reason: 'spa_skipped_not_enough_slices',
          },
          run_verdict: {
            passed: false,
            approved: false,
            candidates_tested: 4,
            candidates_survived_hard_gates: 0,
            approved_candidate_count: 0,
            final_recommendation: 'no_candidate_survived',
            warning_flags: ['cross_ticker_filtered_all_candidates'],
            fail_reasons: [],
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_validation_skipped.json', bytes: 128 }
      }
      return {}
    })

    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '4',
      seed: '1',
      wfEnable: false,
      reoptWfEnable: true,
      reoptWfWindow: '8000',
      reoptWfStep: '2000',
      reoptWfTopN: '4',
      perturbEnable: true,
      perturbIters: '8',
      perturbMin: '0.01',
      perturbMax: '0.05',
      mcEnable: true,
      mcSims: '500',
      cscvEnable: true,
      spaEnable: true,
      crossEnable: true,
      crossMode: 'preserved_finalists',
      crossMax: '1',
      crossDatasets: [
        { path: 'C:/data/SPY.csv', label: 'SPY', meta: 'Stocks • 1h', checked: true, disabled: false },
      ],
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-13T00:00:00Z',
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-skipped-meta',
          backend: { backend: 'GpuKernel', device_id: 0 },
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
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.validationHint || '')).toContain('Validation complete with warnings:')
    expect(String(validationState.validationHint || '')).toContain('CSCV skipped')
    expect(String(validationState.validationHint || '')).toContain('SPA skipped')
    expect(String(validationState.validationMeta || '')).toContain('cscv skipped=cscv_skipped_not_enough_bars')
    expect(String(validationState.validationMeta || '')).toContain('spa skipped=spa_skipped_not_enough_slices')
    expect(String(validationState.validationMeta || '')).toContain('note=cross_ticker_filtered_all_candidates')
    expect(String(validationState.validationRouteSummary || '')).toContain('aux CPU tails: perturb + trade bootstrap compare (iid + stationary, stat block=auto)')
    expect(String(validationState.validationRouteDetailSummary || '')).toContain('none')
    expect(String(validationState.validationAuxCpuSummary || '')).toContain('perturb + trade bootstrap compare (iid + stationary, stat block=auto)')
    expect(String(validationState.validationBootstrapSummary || '')).toContain('iid + stationary closed-trade compare • stat block=auto')
    expect(String(validationState.validationCrossSummary || '')).toContain('not run')
    expect(String(validationState.validationSkippedSummary || '')).toContain('Cross: unavailable')
    expect(String(validationState.validationSkippedSummary || '')).toContain('Cross reruns: omit walk_forward + reopt WF + SPA + perturb + trade bootstrap')
    expect(String(validationState.validationSkippedSummary || '')).toContain('Reopt WF: unavailable')
    expect(String(validationState.validationSkippedSummary || '')).toContain('CSCV: cscv_skipped_not_enough_bars')
    expect(String(validationState.validationSkippedSummary || '')).toContain('SPA: spa_skipped_not_enough_slices')
    expect(String(validationState.validationWarningSummary || '')).toContain('cross_ticker_filtered_all_candidates')
  })

  it('surfaces intentional aux CPU tails in post-run validation warnings', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_double_ma_validation') {
        return {
          backend_used: 'Gpu',
          backend_route: 'GpuKernel',
          runtime_ms: 77,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: 'single_candidate_supported',
            warning_flags: [],
            fail_reasons: [],
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_validation_aux_cpu.json', bytes: 128 }
      }
      return {}
    })

    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '4',
      seed: '1',
      wfEnable: false,
      perturbEnable: true,
      perturbIters: '8',
      perturbMin: '-10',
      perturbMax: '10',
      mcEnable: true,
      mcSims: '200',
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-14T00:00:00Z',
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-double-aux-cpu',
          backend: { backend: 'GpuKernel', device_id: 0 },
          ma_source: 'close',
          fast_ma_types: ['alma'],
          slow_ma_types: ['alma'],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'alma',
                slow_ma_type: 'alma',
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.validationRouteSummary || '')).toContain('aux CPU tails: perturb + trade bootstrap compare (iid + stationary, stat block=auto)')
    expect(String(validationState.validationWarningSummary || '')).toContain('Aux CPU tails are intentional in the current validation route')
    expect(String(validationState.validationWarningSummary || '')).toContain('perturb + trade bootstrap compare (iid + stationary, stat block=auto)')
  })

  it('surfaces regime and friction summaries for the top validation finalist', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_double_ma_validation') {
        return {
          backend_used: 'Gpu',
          backend_route: 'GpuKernel',
          candidates: [
            {
              params: {
                fast_ma_type: 'alma',
                slow_ma_type: 'alma',
                fast_len: 9,
                slow_len: 30,
              },
              validation_verdict: { passed: true, warning_flags: [], fail_reasons: [] },
              regime_slice_robustness: {
                worst_bucket_label: 'range',
                worst_sharpe: -0.08,
                worst_max_dd: 0.22,
                stable_bucket_count: 3,
                stable_bucket_frac: 0.6,
                support_label: 'trend_only',
                trend: { pnl: 0.12, sharpe: 0.41 },
                range: { pnl: -0.02, sharpe: -0.08 },
              },
              friction_stress: {
                pass_frac: 0.5,
                max_rank_drift: 2,
                worst_scenario_label: 'spread_stress',
              },
            },
          ],
          runtime_ms: 91,
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            first_approved_rank: 1,
            final_recommendation: 'single_candidate_supported',
            warning_flags: [],
            fail_reasons: [],
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_validation_regime_friction.json', bytes: 128 }
      }
      return {}
    })

    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '1',
      seed: '1',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-14T01:00:00Z',
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-regime-friction',
          backend: { backend: 'GpuKernel', device_id: 0 },
          ma_source: 'close',
          fast_ma_types: ['alma'],
          slow_ma_types: ['alma'],
          strategy: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'alma',
                slow_ma_type: 'alma',
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.validationRegimeSummary || '')).toContain('selective')
    expect(String(validationState.validationRegimeSummary || '')).toContain('support=trend_only')
    expect(String(validationState.validationRegimeSummary || '')).toContain('worst=range')
    expect(String(validationState.validationRegimeSummary || '')).toContain('stable=3/5 (60%)')
    expect(String(validationState.validationFrictionSummary || '')).toContain('fragile')
    expect(String(validationState.validationFrictionSummary || '')).toContain('worst=spread_stress')
    expect(String(validationState.validationFrictionSummary || '')).toContain('absolute pnl/sharpe rule')
  })

  it('routes composite validation kinds through run_composite_validation', async () => {
    const strategyKinds = ['Kqsr', 'Carm', 'Kvdf', 'Cdmc']
    for (const strategyKind of strategyKinds) {
      invokeCommandMock.mockClear()
      installDefaultInvokeCommandMock()
      ;(pinia.state.value as Record<string, unknown>).validation = {
        candidates: '8',
        seed: '1',
        crossEnable: false,
      }
      useWorkflowArtifactsStore(pinia).clearArtifacts()
      useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
        currentRun: {
          ts: '2026-02-25T12:00:00Z',
          req: {
            strategy_kind: strategyKind,
            data_id: `data-${strategyKind.toLowerCase()}`,
            backend: { backend: 'CpuOnly' },
            ma_source: 'close',
            strategy: { fee_bps: 1.0 },
          },
          res: {
            top: [
              {
                params: {
                  fast_ma_type: 'ema',
                  slow_ma_type: 'sma',
                  fast_len: 10,
                  slow_len: 40,
                  [strategyKind.toLowerCase()]: { alpha: 1.0 },
                },
              },
            ],
          },
        },
        currentValidation: null,
      })

      await runValidation({ switchTab: false })

      const [command] = firstValidationRunCall()
      expect(command, `strategy ${strategyKind} should use composite validation`).toBe('run_composite_validation')
    }
  })

  it('surfaces composite lightweight cross rerun semantics in post-run summaries', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_composite_validation') {
        return {
          backend_used: 'Gpu',
          backend_route: 'GpuKernel',
          runtime_ms: 111,
          cross_ticker: {
            mode: 'preserved_finalists',
            datasets: [
              { label: 'SPY' },
              { label: 'QQQ' },
            ],
          },
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 2,
            candidates_survived_hard_gates: 2,
            approved_candidate_count: 1,
            final_recommendation: 'single_candidate_supported',
            warning_flags: [],
            fail_reasons: [],
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_validation_composite_cross.json', bytes: 128 }
      }
      return {}
    })

    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '8',
      seed: '1',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: true,
      cscvSlices: '16',
      crossEnable: true,
      crossMode: 'preserved_finalists',
      crossMax: '2',
      crossDatasets: [
        { path: 'C:/data/SPY.csv', label: 'SPY', checked: true, disabled: false },
        { path: 'C:/data/QQQ.csv', label: 'QQQ', checked: true, disabled: false },
      ],
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-14T00:00:00Z',
        req: {
          strategy_kind: 'Mkcs',
          data_id: 'data-composite-post-run',
          backend: { backend: 'GpuKernel', device_id: 0 },
          ma_source: 'close',
          strategy: { fee_bps: 1.0 },
          mkcs: { compression_threshold: 0.01, reexpansion_threshold: 0.03 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'ema',
                slow_ma_type: 'sma',
                fast_len: 12,
                slow_len: 36,
                mkcs: { stddev_period: 20, breakout_lookback: 30 },
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.validationRouteSummary || '')).toContain('Gpu • GpuKernel')
    expect(String(validationState.validationCrossSummary || '')).toContain('requested=preserved_finalists')
    expect(String(validationState.validationCrossSummary || '')).toContain('effective lightweight reruns')
    expect(String(validationState.validationSkippedSummary || '')).toContain('Cross reruns: composite main route does not support inline cross_ticker; lightweight composite reruns omit walk_forward + reopt WF + SPA + perturb + trade bootstrap + CSCV + hard gates')
  })

  it('surfaces CompositeMo CPU-preferred validation fallback explicitly in post-run summaries', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_composite_validation') {
        return {
          backend_used: 'Cpu',
          backend_route: 'CpuOnly',
          runtime_ms: 88,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: 'single_candidate_supported',
            warning_flags: [],
            fail_reasons: [],
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_validation_composite_mo_cpu.json', bytes: 128 }
      }
      return {}
    })

    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '4',
      seed: '1',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-14T00:00:00Z',
        req: {
          strategy_kind: 'CompositeMo',
          data_id: 'data-composite-mo-post-run',
          backend: { backend: 'GpuKernel', device_id: 0 },
          ma_source: 'close',
          strategy: { fee_bps: 1.0 },
          signal_mode: 'hard_gate',
          halftrend: {},
          macz: {},
          ott: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'ema',
                slow_ma_type: 'sma',
                fast_len: 12,
                slow_len: 36,
                signal_mode: 'hard_gate',
                strategy: { fee_bps: 1.0 },
                halftrend: {},
                macz: {},
                ott: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.validationRouteSummary || '')).toContain('Cpu • CpuOnly • route note: CompositeMo uses the CPU validation route; GPU fixed-candidate validation currently supports plain CompositeCpu HardGate candidates only.')
    expect(String(validationState.validationRouteDetailSummary || '')).toContain('CompositeMo uses the CPU validation route; GPU fixed-candidate validation currently supports plain CompositeCpu HardGate candidates only.')
    expect(String(validationState.validationWarningSummary || '')).toContain('CompositeMo uses the CPU validation route; GPU fixed-candidate validation currently supports plain CompositeCpu HardGate candidates only.')
  })

  it('surfaces Carm GPU validation route detail explicitly in post-run summaries', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_composite_validation') {
        return {
          backend_used: 'Gpu',
          backend_route: 'GpuKernel',
          runtime_ms: 88,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: 'single_candidate_supported',
            warning_flags: [],
            fail_reasons: [],
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_validation_carm_gpu.json', bytes: 128 }
      }
      return {}
    })

    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '4',
      seed: '1',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-14T00:00:00Z',
        req: {
          strategy_kind: 'Carm',
          data_id: 'data-carm-post-run',
          backend: { backend: 'GpuKernel', device_id: 0 },
          ma_source: 'close',
          signal_mode: 'hard_gate',
          halftrend: {},
          macz: {},
          ott: {},
          carm: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'ema',
                slow_ma_type: 'sma',
                fast_len: 12,
                slow_len: 36,
                signal_mode: 'hard_gate',
                halftrend: {},
                macz: {},
                ott: {},
                carm: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.validationRouteSummary || '')).toContain('Gpu • GpuKernel')
    expect(String(validationState.validationRouteDetailSummary || '')).toContain('CARM GPU validation currently uses CPU-reference signal prep with GPU metric evaluation')
    expect(String(validationState.validationWarningSummary || '')).toContain('none')
  })

  it('surfaces Carm Auto CPU-preferred policy explicitly in post-run summaries', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_composite_validation') {
        return {
          backend_used: 'Cpu',
          backend_route: 'Auto->CpuOnly',
          backend_fallback_reason: 'Auto composite validation currently prefers CPU for CARM because current advanced-stack benches show CPU faster than GPU on measured workloads.',
          runtime_ms: 41,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: 'single_candidate_supported',
            warning_flags: [],
            fail_reasons: [],
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_validation_carm_auto_cpu.json', bytes: 128 }
      }
      return {}
    })

    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '4',
      seed: '1',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-14T00:00:00Z',
        req: {
          strategy_kind: 'Carm',
          data_id: 'data-carm-auto-post-run',
          backend: { backend: 'Auto', device_id: 0 },
          ma_source: 'close',
          signal_mode: 'hard_gate',
          halftrend: {},
          macz: {},
          ott: {},
          carm: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'ema',
                slow_ma_type: 'sma',
                fast_len: 12,
                slow_len: 36,
                signal_mode: 'hard_gate',
                halftrend: {},
                macz: {},
                ott: {},
                carm: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.validationRouteSummary || '')).toContain('Cpu • Auto->CpuOnly • route note: Auto composite validation currently prefers CPU for CARM')
    expect(String(validationState.validationRouteDetailSummary || '')).toContain('Auto composite validation currently prefers CPU for CARM')
    expect(String(validationState.validationWarningSummary || '')).toContain('none')
  })

  it('surfaces Cdmc Auto GPU route explicitly in post-run summaries', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_composite_validation') {
        return {
          backend_used: 'Gpu',
          backend_route: 'Auto->GpuKernel',
          runtime_ms: 284,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: 'single_candidate_supported',
            warning_flags: [],
            fail_reasons: [],
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_validation_cdmc_auto_gpu.json', bytes: 128 }
      }
      return {}
    })

    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '4',
      seed: '1',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-14T00:00:00Z',
        req: {
          strategy_kind: 'Cdmc',
          data_id: 'data-cdmc-auto-post-run',
          backend: { backend: 'Auto', device_id: 0 },
          ma_source: 'close',
          signal_mode: 'hard_gate',
          halftrend: {},
          macz: {},
          ott: {},
          cdmc: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'ema',
                slow_ma_type: 'sma',
                fast_len: 12,
                slow_len: 36,
                signal_mode: 'hard_gate',
                halftrend: {},
                macz: {},
                ott: {},
                cdmc: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.validationRouteSummary || '')).toContain('Gpu • Auto->GpuKernel')
    expect(String(validationState.validationRouteDetailSummary || '')).toContain('CDMC GPU validation currently uses CPU-reference signal prep with GPU metric evaluation')
    expect(String(validationState.validationWarningSummary || '')).toContain('none')
  })


  it('previews validation execution summary from current workflow artifacts', () => {
    ;(pinia.state.value as Record<string, unknown>).data = {
      csvPath: 'C:/data/AAPL.csv',
      dataTimeframe: '1h',
      baseDataMeta: { timeframe: '1h' },
    }
    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '12',
      seed: '1',
      wfEnable: true,
      wfWindow: '12000',
      wfStep: '3000',
      reoptWfEnable: true,
      reoptWfWindow: '12000',
      reoptWfStep: '3000',
      reoptWfTopN: '4',
      spaEnable: true,
      spaSlices: '12',
      spaBootstrapSamples: '300',
      spaBlockLength: '4',
      perturbEnable: true,
      perturbIters: '8',
      perturbMin: '0.01',
      perturbMax: '0.05',
      mcEnable: true,
      mcSims: '500',
      cscvEnable: true,
      cscvSlices: '16',
      crossEnable: true,
      crossMax: '2',
      crossMode: 'preserved_finalists',
      crossDatasets: [
        { path: 'C:/data/SPY.csv', label: 'SPY', checked: true },
        { path: 'C:/data/QQQ.csv', label: 'QQQ', checked: true },
      ],
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        csvPath: 'C:/data/AAPL.csv',
        datasetMeta: { csv_path: 'C:/data/AAPL.csv', timeframe: '1h', symbol: 'AAPL' },
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'aapl_1h',
          backend: { backend: 'GpuKernel', device_id: 0 },
          ma_source: 'close',
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'alma',
                slow_ma_type: 'alma',
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    const summary = previewValidationExecutionSummaryFromStores()
    expect(summary.routeSummary).toContain('GpuKernel requested')
    expect(summary.crossSummary).toContain('2 datasets')
    expect(summary.crossSummary).toContain('finalists only')
    expect(summary.moduleSummary).toContain('auxiliary CPU tails: perturb + trade bootstrap compare (iid + stationary, stat block=auto)')
    expect(summary.categorySummary).toContain('GPU-native: base reruns + hard gates + walk_forward + reopt_walk_forward + cscv_pbo + spa')
    expect(summary.categorySummary).toContain('GPU main + CPU tail: perturb + trade_bootstrap')
    expect(summary.skippedSummary).toContain('Cross reruns: preserved-finalists omit walk_forward + reopt_walk_forward + SPA + perturb + trade bootstrap')
    expect(summary.warningSummary).toContain('Preserved-finalists cross reruns keep CSCV/PBO + hard gates')
  })

  it('surfaces Cdmc GPU validation route detail explicitly in post-run summaries', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_composite_validation') {
        return {
          backend_used: 'Gpu',
          backend_route: 'GpuKernel',
          runtime_ms: 88,
          candidates: [{ score: 1 }],
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: 'single_candidate_supported',
            warning_flags: [],
            fail_reasons: [],
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_validation_cdmc_gpu.json', bytes: 128 }
      }
      return {}
    })

    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '4',
      seed: '1',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-14T00:00:00Z',
        req: {
          strategy_kind: 'Cdmc',
          data_id: 'data-cdmc-post-run',
          backend: { backend: 'GpuKernel', device_id: 0 },
          ma_source: 'close',
          signal_mode: 'hard_gate',
          halftrend: {},
          macz: {},
          ott: {},
          cdmc: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'ema',
                slow_ma_type: 'sma',
                fast_len: 12,
                slow_len: 36,
                signal_mode: 'hard_gate',
                halftrend: {},
                macz: {},
                ott: {},
                cdmc: {},
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.validationRouteSummary || '')).toContain('Gpu • GpuKernel')
    expect(String(validationState.validationRouteDetailSummary || '')).toContain('CDMC GPU validation currently uses CPU-reference signal prep with GPU metric evaluation')
    expect(String(validationState.validationCategorySummary || '')).toContain('GPU-native: base reruns + hard gates')
    expect(String(validationState.validationWarningSummary || '')).toContain('none')
  })


  it('distinguishes GPU route notes from true CPU fallback in validation summary text', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'run_double_ma_validation') {
        return {
          backend_used: 'Gpu',
          backend_route: 'GpuHybridKernel->GpuKernel',
          backend_fallback_reason: 'GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.',
          candidates: [{ score: 1 }],
          runtime_ms: 91,
          run_verdict: {
            passed: true,
            approved: true,
            candidates_tested: 1,
            candidates_survived_hard_gates: 1,
            approved_candidate_count: 1,
            final_recommendation: 'single_candidate_supported',
            warning_flags: [],
            fail_reasons: [],
          },
        }
      }
      if (command === 'save_run_artifact') {
        return { path: 'runs/vectorgrid_run_validation_route_note.json', bytes: 128 }
      }
      return {}
    })

    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '1',
      seed: '1',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-13T12:00:00Z',
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-route-note',
          backend: { backend: 'GpuHybridKernel', device_id: 0 },
          ma_source: 'close',
          fast_ma_types: ['sma'],
          slow_ma_types: ['sma'],
          strategy: {},
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'sma',
                slow_ma_type: 'sma',
                fast_len: 10,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.validationMeta || '')).toContain('route=GpuHybridKernel->GpuKernel')
    expect(String(validationState.validationMeta || '')).toContain('route_note=GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.')
    expect(String(validationState.validationMeta || '')).not.toContain('fallback=GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.')
    expect(String(validationState.validationHint || '')).toContain('route note: GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.')
    expect(String(validationState.validationRouteSummary || '')).toContain('Gpu • GpuHybridKernel->GpuKernel • route note: GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.')
    expect(String(validationState.validationRouteDetailSummary || '')).toContain('GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.')
    expect(String(validationState.validationCategorySummary || '')).toContain('GPU-native: base reruns + hard gates')
    expect(String(validationState.validationAuxCpuSummary || '')).toBe('none')
    expect(String(validationState.validationBootstrapSummary || '')).toBe('not run')
  })

  it('surfaces iid-only bootstrap mode in preview summaries', () => {
    ;(pinia.state.value as Record<string, unknown>).data = {
      csvPath: 'C:/data/AAPL.csv',
      dataTimeframe: '1h',
      baseDataMeta: { timeframe: '1h' },
    }
    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '12',
      seed: '1',
      wfEnable: true,
      wfWindow: '12000',
      wfStep: '3000',
      reoptWfEnable: true,
      reoptWfWindow: '12000',
      reoptWfStep: '3000',
      reoptWfTopN: '4',
      spaEnable: true,
      spaSlices: '12',
      spaBootstrapSamples: '300',
      spaBlockLength: '4',
      perturbEnable: true,
      perturbIters: '8',
      perturbMin: '0.01',
      perturbMax: '0.05',
      mcEnable: true,
      mcSims: '400',
      mcBlockLen: '9',
      mcMode: 'iid_only',
      cscvEnable: true,
      cscvSlices: '16',
      crossEnable: true,
      crossMax: '2',
      crossMode: 'preserved_finalists',
      crossDatasets: [
        { path: 'C:/data/SPY.csv', label: 'SPY', checked: true },
        { path: 'C:/data/QQQ.csv', label: 'QQQ', checked: true },
      ],
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        csvPath: 'C:/data/AAPL.csv',
        datasetMeta: { csv_path: 'C:/data/AAPL.csv', timeframe: '1h', symbol: 'AAPL' },
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'aapl_1h',
          backend: { backend: 'GpuKernel', device_id: 0 },
          ma_source: 'close',
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'alma',
                slow_ma_type: 'alma',
                fast_len: 9,
                slow_len: 30,
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    const summary = previewValidationExecutionSummaryFromStores()
    expect(summary.moduleSummary).toContain('trade bootstrap iid-only')
    expect(summary.categorySummary).toContain('GPU main + CPU tail: perturb + trade_bootstrap')
  })

  it('auto-exports XLSX after successful validation when enabled and persists the report path', async () => {
    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '12',
      seed: '1',
      minPsr: '',
      minDsr: '',
      autoExportXlsx: true,
      autoRevealReport: false,
      autoExportTopN: '7',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-auto-export',
          backend: { backend: 'GpuKernel', device_id: 0 },
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
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    expect(invokeCommandMock).toHaveBeenCalledWith('save_report_xlsx', expect.objectContaining({
      max_rows: 7,
      report: expect.any(Object),
    }))
    expect(invokeCommandMock).toHaveBeenCalledWith('open_path', {
      path: 'runs/vectorgrid_report_validation_test.xlsx',
    })
    expect(invokeCommandMock).not.toHaveBeenCalledWith('reveal_path', expect.anything())

    const workflowStore = useWorkflowArtifactsStore(pinia)
    expect(workflowStore.currentRun?.reportXlsxPath).toBe('runs/vectorgrid_report_validation_test.xlsx')

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(validationState.statusLine).toBe('Saved XLSX report: runs/vectorgrid_report_validation_test.xlsx')
  })

  it('rejects validation when the current dataset does not match the optimization artifact', async () => {
    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '8',
      seed: '1',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }
    ;(pinia.state.value as Record<string, unknown>).data = {
      csvPath: 'C:/data/MSFT.csv',
      dataTimeframe: '15m',
      baseDataMeta: { timeframe: '15m' },
      datasetLabel: 'MSFT',
      datasetContext: 'source=csv',
      dataFields: ['open', 'high', 'low', 'close', 'volume'],
      dataLen: '1000',
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-08T18:00:00Z',
        csv_path: 'C:/data/AAPL.csv',
        dataset_meta: { csv_path: 'C:/data/AAPL.csv', timeframe: '15m', label: 'AAPL', symbol: 'AAPL' },
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-aapl',
          ma_source: 'close',
          fast_ma_types: ['sma'],
          slow_ma_types: ['ema'],
          strategy: { fee_bps: 1.5 },
        },
        res: {
          top: [{ params: { fast_ma_type: 'sma', slow_ma_type: 'ema', fast_len: 10, slow_len: 40 } }],
        },
      },
      currentValidation: null,
    })

    await expect(runValidation({ switchTab: false })).rejects.toThrow('Validation requires the optimization dataset')
    expect(invokeCommandMock).not.toHaveBeenCalledWith('run_double_ma_validation', expect.anything())
  })

  it('routes ASR validation through composite validation command', async () => {
    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '8',
      seed: '3',
      minPsr: '',
      minDsr: '',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-01T00:00:00Z',
        req: {
          strategy_kind: 'Asr',
          data_id: 'asr-data',
          ma_source: 'close',
          fast_ma_types: ['alma'],
          slow_ma_types: ['alma'],
          strategy: { fee_bps: 1.0 },
          asr: { angle_deg_min: 10, z_entry: 2.0 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'alma',
                slow_ma_type: 'alma',
                fast_len: 12,
                slow_len: 48,
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })
    const [command, args] = firstValidationRunCall()
    expect(command).toBe('run_composite_validation')
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    const candidates = Array.isArray(req.candidates) ? req.candidates as Array<Record<string, unknown>> : []
    expect(candidates[0]?.asr).toMatchObject({ angle_deg_min: 10, z_entry: 2.0 })
  })

  it('routes Mkcs validation through composite validation command', async () => {
    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '8',
      seed: '5',
      minPsr: '',
      minDsr: '',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-11T00:00:00Z',
        req: {
          strategy_kind: 'Mkcs',
          data_id: 'mkcs-data',
          backend: { backend: 'GpuKernel', device_id: 0 },
          ma_source: 'close',
          fast_ma_types: ['ema'],
          slow_ma_types: ['sma'],
          strategy: { fee_bps: 1.25 },
          mkcs: { compression_threshold: 0.01, reexpansion_threshold: 0.03 },
        },
        res: {
          top: [
            {
              params: {
                fast_ma_type: 'ema',
                slow_ma_type: 'sma',
                fast_len: 12,
                slow_len: 48,
                mkcs: { stddev_period: 20, breakout_lookback: 30 },
              },
            },
          ],
        },
      },
      currentValidation: null,
    })

    await runValidation({ switchTab: false })

    const [command, args] = firstValidationRunCall()
    expect(command).toBe('run_composite_validation')
    const req = (args.req && typeof args.req === 'object') ? args.req as Record<string, unknown> : {}
    const candidates = Array.isArray(req.candidates) ? req.candidates as Array<Record<string, unknown>> : []
    expect(req.backend).toEqual({ backend: 'Auto', device_id: 0 })
    expect(candidates[0]?.mkcs).toEqual({ stddev_period: 20, breakout_lookback: 30 })
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
      mode: 'preserved_finalists',
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

  it('keeps validation successful when auto-export XLSX fails', async () => {
    ;(pinia.state.value as Record<string, unknown>).validation = {
      candidates: '10',
      seed: '2',
      minPsr: '',
      minDsr: '',
      autoExportXlsx: true,
      autoRevealReport: false,
      autoExportTopN: '5',
      wfEnable: false,
      perturbEnable: false,
      mcEnable: false,
      cscvEnable: false,
      crossEnable: false,
    }

    useWorkflowArtifactsStore(pinia).applyVueRuntimePayload({
      currentRun: {
        ts: '2026-02-25T12:00:00Z',
        req: {
          strategy_kind: 'DoubleMa',
          data_id: 'data-auto-export-fail',
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

    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'save_report_xlsx') throw new Error('save failed')
      if (command === 'build_report_xlsx') throw new Error('fallback failed')
      if (command === 'run_double_ma_validation' || command === 'run_composite_validation') {
        return {
          backend_used: 'Cpu',
          candidates: [{ score: 1 }],
          runtime_ms: 123,
        }
      }
      if (command === 'save_run_artifact') {
        return {
          path: 'runs/vectorgrid_run_validation_test.json',
          bytes: 256,
        }
      }
      if (command === 'open_path' || command === 'reveal_path') return true
      if (command === 'polygon_get_cache_info') return { datasets: [] }
      return {}
    })

    await expect(runValidation({ switchTab: false })).resolves.toBeUndefined()

    const workflowStore = useWorkflowArtifactsStore(pinia)
    expect(workflowStore.currentValidation?.res).toBeTruthy()
    expect(workflowStore.currentRun?.reportXlsxPath).toBe('')

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(String(validationState.statusLine || '')).toContain('auto-export XLSX failed')
    expect(String(validationState.statusLine || '')).toContain('save failed')
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
        artifact_path: 'runs/vectorgrid_run_validation_test.json',
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

  it('preserves Stage 3 meta XLSX fallback when no validation artifact exists', async () => {
    ;(pinia.state.value as Record<string, unknown>).validation = {
      autoRevealReport: true,
    }
    ;(pinia.state.value as Record<string, unknown>).commandActions = {
      latestMetaReportPath: 'C:/reports/meta_latest.xlsx',
    }

    await exportReportXlsx()

    expect(invokeCommandMock).not.toHaveBeenCalledWith('save_report_xlsx', expect.anything())
    expect(invokeCommandMock).toHaveBeenCalledWith('open_path', {
      path: 'C:/reports/meta_latest.xlsx',
    })
    expect(invokeCommandMock).toHaveBeenCalledWith('reveal_path', {
      path: 'C:/reports/meta_latest.xlsx',
    })

    const validationState = (pinia.state.value as Record<string, unknown>).validation as Record<string, unknown>
    expect(validationState.statusLine).toBe('Opened Stage 3 XLSX report: C:/reports/meta_latest.xlsx')
  })
})
