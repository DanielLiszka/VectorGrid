// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invokeCommand } from '../tauri/invoke'
import {
  analyzeOptionsExpressionFeasibility,
  downloadPolygonOptionsDataset,
  exportOptionsExpressionCsv,
  importOptionsDataset,
  inspectOptionsExpressionTuple,
  listOptionsDatasets,
  listOptionsExpressionArtifacts,
  loadOptionsExpressionArtifact,
  normalizeOptionsDatasetIds,
  openOrRevealOptionsPath,
  pickOptionsCsvPath,
  runOptionsExpression,
} from './optionsExpressionService'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => ({})),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

describe('optionsExpressionService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('normalizes imported dataset manifests', async () => {
    invokeCommandMock.mockResolvedValueOnce({
      version: 1,
      options_dataset_id: 'spy_options',
      root_symbol: 'SPY',
      provider_name: 'massive',
      provider_source_surface: 'minute_aggregate',
      quote_resolution: '1m',
      price_semantics: 'minute_aggregate',
      calendar_id: 'market_hours_utc',
      timezone: 'UTC',
      coverage_start: 100,
      coverage_end: 200,
      normalization_version: 1,
      quote_semantics_version: 1,
      corporate_action_mapping_version: 1,
      contract_reference_source: 'massive_contract_reference',
      underlier_reference_source: 'underlying_dataset',
      dataset_path: 'C:/data/options.csv',
      row_count: 42,
      contract_count: 3,
      aggregate_price_field: 'close',
    })

    const summary = await importOptionsDataset({ csv_path: 'C:/data/options.csv' })

    expect(summary.optionsDatasetId).toBe('spy_options')
    expect(summary.rootSymbol).toBe('SPY')
    expect(summary.rowCount).toBe(42)
    expect(summary.priceSemantics).toBe('minute_aggregate')
    expect(invokeCommandMock).toHaveBeenCalledWith('import_options_dataset', {
      req: { csv_path: 'C:/data/options.csv' },
    })
  })

  it('normalizes Polygon/Massive download responses', async () => {
    invokeCommandMock.mockResolvedValueOnce({
      manifest: {
        version: 1,
        options_dataset_id: 'spy_options_1m',
        root_symbol: 'SPY',
        provider_name: 'massive',
        provider_source_surface: 'minute_aggregate',
        quote_resolution: '1m',
        price_semantics: 'minute_aggregate',
        calendar_id: 'market_hours_utc',
        timezone: 'UTC',
        coverage_start: 100,
        coverage_end: 200,
        normalization_version: 1,
        quote_semantics_version: 1,
        corporate_action_mapping_version: 1,
        contract_reference_source: 'massive_contract_reference',
        contract_reference_as_of_utc: '2026-04-06T23:59:00Z',
        underlier_reference_source: 'polygon_stocks_1m_raw:SPY',
        dataset_path: 'C:/data/spy_options_rows.csv',
        row_count: 200,
        contract_count: 8,
        aggregate_price_field: 'close',
      },
      contracts_discovered: 22,
      contracts_selected: 8,
      contracts_with_rows: 6,
      downloaded_option_rows: 200,
      skipped_contracts: 2,
      underlying_csv_path: 'C:/cache/polygon_stocks_raw_SPY_1m.csv',
      used_underlying_cache_only: true,
    })

    const summary = await downloadPolygonOptionsDataset({
      rootSymbol: 'SPY',
      from: '2026-04-01',
      to: '2026-04-02',
      contractType: 'call',
    })

    expect(summary.manifest.optionsDatasetId).toBe('spy_options_1m')
    expect(summary.contractsSelected).toBe(8)
    expect(summary.downloadedOptionRows).toBe(200)
    expect(summary.usedUnderlyingCacheOnly).toBe(true)
    expect(invokeCommandMock).toHaveBeenCalledWith('download_polygon_options_dataset', {
      req: {
        root_symbol: 'SPY',
        from: '2026-04-01',
        to: '2026-04-02',
        contract_type: 'call',
      },
    })
  })

  it('normalizes run responses and artifact summaries', async () => {
    invokeCommandMock
      .mockResolvedValueOnce({
        artifact_path: 'C:/runs/vectorgrid_options_expression_1.json',
        artifact: {
          ts: '2026-04-05T12:00:00Z',
          parent_artifact_id: 'vectorgrid_run_1',
          options_dataset_hash: 'hash-1',
          request: {
            parent_artifact_path: 'C:/runs/vectorgrid_run_1.json',
            options_dataset_id: 'spy_options',
            split_assignment_mode: 'session_chronological',
            delay_minutes: [0, 1],
            dte_buckets: [1, 3],
            moneyness_buckets: ['atm', 'otm_1'],
            hold_minutes: [15, 30],
            target_return_buckets: [0.25],
            stop_return_buckets: [0.25],
            exit_template: 'fixed_hold_or_target_stop',
            fill_model: 'aggregate_research',
            filters: {
              max_entry_lag_minutes: 5,
            },
            top_k: 25,
            split_ratio_search: 0.6,
            split_ratio_holdout: 0.2,
          },
        },
        result: {
          backend_used: 'cpu_reference',
          backend_fallback_reason: 'GPU planner probe failed; using CPU.',
          price_semantics: 'minute_aggregate',
          tuple_count: 2,
          staging: {
            candidate_slot_count: 12,
            compiled_candidate_count: 9,
            baseline_exit_summary_count: 24,
            conservative_exit_summary_count: 24,
            harsher_exit_summary_count: 24,
          },
          diagnostics: {
            parent_artifact_path: 'C:/runs/vectorgrid_run_1.json',
            options_dataset_id: 'spy_options',
            root_symbol: 'SPY',
            total_events: 10,
            bullish_events: 6,
            bearish_events: 4,
            search_events: 6,
            holdout_events: 2,
            sealed_test_events: 2,
            dataset_contracts: 3,
            dataset_rows: 42,
            coverage_start: 100,
            coverage_end: 200,
          },
          ranked_results: [
            {
              tuple: {
                delay_minutes: 0,
                dte_days: 1,
                moneyness_bucket: 'atm',
                hold_minutes: 15,
                target_return: 0.25,
                stop_return: 0.25,
                exit_template: 'fixed_hold_or_target_stop',
                fill_model: 'aggregate_research',
              },
              pre_sealed_rank: 0,
              sealed_finalist: false,
              sealed_final_rank: 0,
              selection_metrics: {
                valid_trades: 5,
                invalid_events: 1,
                invalid_rate: 0.1667,
                expectancy_per_trade: 12,
                expectancy_per_premium: 0.2,
                median_return_on_premium: 0.15,
                win_rate: 0.6,
                total_pnl_per_contract: 60,
              },
              robust_score: 0.18,
              sealed_stress_metrics: {
                valid_trades: 2,
                invalid_events: 0,
                invalid_rate: 0,
                expectancy_per_trade: 2,
                expectancy_per_premium: 0.03,
                median_return_on_premium: 0.02,
                win_rate: 0.5,
                total_pnl_per_contract: 4,
              },
              harsher_stress_metrics: {
                valid_trades: 2,
                invalid_events: 0,
                invalid_rate: 0,
                expectancy_per_trade: 2,
                expectancy_per_premium: 0.02,
                median_return_on_premium: 0.01,
                win_rate: 0.5,
                total_pnl_per_contract: 3,
              },
              sealed_harsher_stress_metrics: {
                valid_trades: 2,
                invalid_events: 0,
                invalid_rate: 0,
                expectancy_per_trade: 1,
                expectancy_per_premium: 0.01,
                median_return_on_premium: 0.01,
                win_rate: 0.5,
                total_pnl_per_contract: 2,
              },
            },
          ],
        },
      })
      .mockResolvedValueOnce({
        optionsExpression: [
          {
            ts: '2026-04-05T12:00:00Z',
            path: 'C:/runs/vectorgrid_options_expression_1.json',
            bytes: 2048,
            parent_artifact_id: 'vectorgrid_run_1',
            root_symbol: 'SPY',
            options_dataset_id: 'spy_options',
            backend_used: 'cpu_reference',
            backend_fallback_reason: 'GPU planner probe failed; using CPU.',
            price_semantics: 'minute_aggregate',
            tuple_count: 2,
            best_robust_score: 0.18,
            best_selection_expectancy_per_premium: 0.2,
            best_holdout_expectancy_per_premium: 0.1,
            best_sealed_expectancy_per_premium: 0.07,
            best_stress_expectancy_per_premium: 0.04,
            best_sealed_stress_expectancy_per_premium: 0.03,
            best_harsher_stress_expectancy_per_premium: 0.02,
            best_sealed_harsher_stress_expectancy_per_premium: 0.01,
            has_holdout_metrics: true,
            has_sealed_evaluation_metrics: true,
            has_stress_metrics: true,
            has_harsher_stress_metrics: true,
          },
        ],
      })

    const run = await runOptionsExpression({ parent_artifact_path: 'C:/runs/vectorgrid_run_1.json' })
    const artifacts = await listOptionsExpressionArtifacts()

    expect(run.artifactPath).toBe('C:/runs/vectorgrid_options_expression_1.json')
    expect(run.artifactTs).toBe('2026-04-05T12:00:00Z')
    expect(run.optionsDatasetHash).toBe('hash-1')
    expect(run.backendUsed).toBe('cpu_reference')
    expect(run.backendFallbackReason).toBe('GPU planner probe failed; using CPU.')
    expect(run.staging?.compiledCandidateCount).toBe(9)
    expect(run.rankedResults[0]?.selectionMetrics.validTrades).toBe(5)
    expect(run.request?.splitAssignmentMode).toBe('session_chronological')
    expect(run.request?.delayMinutes).toEqual([0, 1])
    expect(run.request?.sealedFinalistCount).toBe(5)
    expect(run.rankedResults[0]?.preSealedRank).toBe(0)
    expect(run.rankedResults[0]?.sealedFinalist).toBe(false)
    expect(run.rankedResults[0]?.sealedStressMetrics?.expectancyPerPremium).toBe(0.03)
    expect(run.rankedResults[0]?.harsherStressMetrics?.expectancyPerPremium).toBe(0.02)
    expect(run.rankedResults[0]?.sealedHarsherStressMetrics?.expectancyPerPremium).toBe(0.01)
    expect(artifacts[0]?.optionsDatasetId).toBe('spy_options')
    expect(artifacts[0]?.tupleCount).toBe(2)
    expect(artifacts[0]?.bestRobustScore).toBe(0.18)
    expect(artifacts[0]?.bestHoldoutExpectancyPerPremium).toBe(0.1)
    expect(artifacts[0]?.bestSealedExpectancyPerPremium).toBe(0.07)
    expect(artifacts[0]?.bestHarsherStressExpectancyPerPremium).toBe(0.02)
    expect(artifacts[0]?.bestSealedHarsherStressExpectancyPerPremium).toBe(0.01)
    expect(artifacts[0]?.sealedFinalistBudget).toBe(0)
    expect(artifacts[0]?.sealedEvaluatedTupleCount).toBe(0)
    expect(artifacts[0]?.hasSealedEvaluationMetrics).toBe(true)
    expect(artifacts[0]?.hasHarsherStressMetrics).toBe(true)
    expect(artifacts[0]?.backendFallbackReason).toBe('GPU planner probe failed; using CPU.')
    expect(invokeCommandMock).toHaveBeenCalledWith('list_artifact_library')
  })

  it('normalizes feasibility preview responses', async () => {
    invokeCommandMock.mockResolvedValueOnce({
      replay_build_ms: 18,
      total_preview_ms: 41,
      report: {
        diagnostics: {
          parent_artifact_path: 'C:/runs/vectorgrid_run_1.json',
          options_dataset_id: 'spy_options',
          root_symbol: 'SPY',
          total_events: 10,
          bullish_events: 6,
          bearish_events: 4,
          search_events: 6,
          holdout_events: 2,
          sealed_test_events: 2,
          dataset_contracts: 3,
          dataset_rows: 42,
        },
        price_semantics: 'minute_aggregate',
        fill_model: 'aggregate_research',
        best_available_backend: 'gpu_compiled_tensor_hybrid',
        auto_backend_decision: {
          recommended_backend: 'cpu_reference',
          gpu_eligible: false,
          reasons: ['Measured GPU hybrid speedup 2.00x is below the current promotion gate of 3.0x.'],
        },
        compiled_tuple_count: 12,
        per_event_candidate_slots: 9,
        per_candidate_exit_summaries: 6,
        candidate_slot_count: 90,
        compiled_candidate_count: 54,
        candidate_density: 0.6,
        invalid_slot_count: 36,
        invalid_slot_rate: 0.4,
        baseline_exit_summary_count: 324,
        baseline_compiled_exit_summary_count: 240,
        baseline_exit_summary_density: 0.7407,
        conservative_exit_summary_count: 324,
        conservative_compiled_exit_summary_count: 228,
        conservative_exit_summary_density: 0.7037,
        harsher_exit_summary_count: 324,
        harsher_compiled_exit_summary_count: 216,
        harsher_exit_summary_density: 0.6667,
        projected_event_tuple_evaluations: 120,
        structural_cache_bytes_estimate: 4096,
        baseline_summary_bytes_estimate: 8192,
        conservative_summary_bytes_estimate: 8192,
        harsher_summary_bytes_estimate: 8192,
        timings: {
          tuple_compile_ms: 1,
          baseline_candidate_compile_ms: 2,
          baseline_exit_summary_compile_ms: 3,
          conservative_candidate_compile_ms: 4,
          conservative_exit_summary_compile_ms: 5,
          harsher_candidate_compile_ms: 6,
          harsher_exit_summary_compile_ms: 7,
          cpu_ranking_ms: 6,
          cpu_ranking_us: 6_400,
          gpu_ranking_ms: 3,
          gpu_ranking_us: 3_100,
          gpu_speedup_vs_cpu: 2,
        },
        event_coverage: {
          total_events_with_candidates: 9,
          search_events_with_candidates: 5,
          holdout_events_with_candidates: 2,
          sealed_test_events_with_candidates: 2,
          total_events_with_baseline_outcomes: 8,
          search_events_with_baseline_outcomes: 4,
          holdout_events_with_baseline_outcomes: 2,
          sealed_test_events_with_baseline_outcomes: 2,
          total_events_with_conservative_outcomes: 7,
          search_events_with_conservative_outcomes: 4,
          holdout_events_with_conservative_outcomes: 2,
          sealed_test_events_with_conservative_outcomes: 1,
          total_events_with_harsher_outcomes: 6,
          search_events_with_harsher_outcomes: 4,
          holdout_events_with_harsher_outcomes: 1,
          sealed_test_events_with_harsher_outcomes: 1,
        },
        cache_stats: {
          structural_cache_hits: 2,
          structural_cache_misses: 2,
          structural_cache_entries: 2,
          summary_cache_hits: 2,
          summary_cache_misses: 2,
          summary_cache_entries: 2,
          gpu_shared_input_cache_hits: 1,
          gpu_shared_input_cache_misses: 1,
          gpu_outcome_input_cache_hits: 1,
          gpu_outcome_input_cache_misses: 1,
          gpu_output_buffer_reuses: 2,
          gpu_output_buffer_allocations: 1,
          gpu_tuple_chunk_count: 3,
          gpu_tuple_chunk_size: 4096,
          gpu_kernel_launch_count: 6,
          gpu_tuple_budget_bytes: 4 * 1024 * 1024,
          gpu_outcome_tensor_bytes: 2 * 1024 * 1024,
          gpu_shared_input_bytes_per_chunk: 64 * 1024,
          gpu_output_bytes_per_chunk: 128 * 1024,
          gpu_total_device_bytes_per_chunk: 3 * 1024 * 1024,
          gpu_device_total_memory_bytes: 8 * 1024 * 1024 * 1024,
          gpu_device_free_memory_bytes: 512 * 1024 * 1024,
          gpu_device_usable_memory_bytes: 256 * 1024 * 1024,
          gpu_memory_budget_source: 'free_memory_headroom',
          gpu_outcome_tensor_fits_in_memory: true,
          gpu_chunk_plan_fits_in_memory: true,
          gpu_plan_notes: ['GPU planner probe failed before ranking: synthetic test note'],
        },
        invalid_reasons: [
          { reason: 'no_expiration_in_bucket', count: 20 },
          { reason: 'premium_too_high', count: 16 },
        ],
        axis_coverage: {
          delay_minutes: [
            {
              delay_minutes: 0,
              slot_count: 30,
              valid_slot_count: 24,
              invalid_slot_count: 6,
              valid_rate: 0.8,
              invalid_reasons: [{ reason: 'premium_too_high', count: 6 }],
            },
          ],
          dte_days: [
            {
              dte_days: 1,
              slot_count: 30,
              valid_slot_count: 24,
              invalid_slot_count: 6,
              valid_rate: 0.8,
              invalid_reasons: [{ reason: 'premium_too_high', count: 6 }],
            },
            {
              dte_days: 5,
              slot_count: 30,
              valid_slot_count: 0,
              invalid_slot_count: 30,
              valid_rate: 0,
              invalid_reasons: [{ reason: 'no_expiration_in_bucket', count: 20 }],
            },
          ],
          moneyness_buckets: [
            {
              moneyness_bucket: 'atm',
              slot_count: 30,
              valid_slot_count: 24,
              invalid_slot_count: 6,
              valid_rate: 0.8,
              invalid_reasons: [{ reason: 'premium_too_high', count: 6 }],
            },
          ],
        },
      },
    })

    const preview = await analyzeOptionsExpressionFeasibility({
      parent_artifact_path: 'C:/runs/vectorgrid_run_1.json',
      options_dataset_id: 'spy_options',
    })

    expect(preview.replayBuildMs).toBe(18)
    expect(preview.totalPreviewMs).toBe(41)
    expect(preview.compiledTupleCount).toBe(12)
    expect(preview.eventCoverage.totalEventsWithCandidates).toBe(9)
    expect(preview.cacheStats.structuralCacheHits).toBe(2)
    expect(preview.cacheStats.summaryCacheMisses).toBe(2)
    expect(preview.cacheStats.gpuMemoryBudgetSource).toBe('free_memory_headroom')
    expect(preview.cacheStats.gpuDeviceUsableMemoryBytes).toBe(256 * 1024 * 1024)
    expect(preview.cacheStats.gpuPlanNotes[0]).toContain('synthetic test note')
    expect(preview.timings.cpuRankingMs).toBe(6)
    expect(preview.timings.cpuRankingUs).toBe(6_400)
    expect(preview.timings.harsherExitSummaryCompileMs).toBe(7)
    expect(preview.timings.gpuRankingMs).toBe(3)
    expect(preview.timings.gpuRankingUs).toBe(3_100)
    expect(preview.timings.gpuSpeedupVsCpu).toBe(2)
    expect(preview.bestAvailableBackend).toBe('gpu_compiled_tensor_hybrid')
    expect(preview.autoBackendDecision.recommendedBackend).toBe('cpu_reference')
    expect(preview.autoBackendDecision.gpuEligible).toBe(false)
    expect(preview.autoBackendDecision.reasons[0]).toContain('Measured GPU hybrid speedup 2.00x')
    expect(preview.harsherExitSummaryDensity).toBe(0.6667)
    expect(preview.eventCoverage.totalEventsWithHarsherOutcomes).toBe(6)
    expect(preview.invalidReasons[0]?.reason).toBe('no_expiration_in_bucket')
    expect(preview.axisCoverage.dteDays[1]?.dteDays).toBe(5)
    expect(preview.axisCoverage.dteDays[1]?.validRate).toBe(0)
    expect(preview.axisCoverage.delayMinutes[0]?.invalidReasons[0]?.reason).toBe('premium_too_high')
    expect(invokeCommandMock).toHaveBeenCalledWith('analyze_options_expression_feasibility', {
      req: {
        parent_artifact_path: 'C:/runs/vectorgrid_run_1.json',
        options_dataset_id: 'spy_options',
      },
    })
  })

  it('normalizes loaded options expression artifacts', async () => {
    invokeCommandMock.mockResolvedValueOnce({
      version: 1,
      kind: 'vectorgrid_options_expression_v1',
      ts: '2026-04-05T12:00:00Z',
      artifact_path: 'C:/runs/vectorgrid_options_expression_1.json',
      parent_artifact_id: 'vectorgrid_run_1',
      root_symbol: 'SPY',
      options_dataset_id: 'spy_options',
      options_dataset_hash: 'hash-1',
      backend_used: 'cpu_reference',
      backend_fallback_reason: 'GPU planner probe failed; using CPU.',
      price_semantics: 'minute_aggregate',
      staging: {
        candidate_slot_count: 12,
        compiled_candidate_count: 9,
        baseline_exit_summary_count: 24,
        conservative_exit_summary_count: 24,
        harsher_exit_summary_count: 24,
      },
      request: {
        parent_artifact_path: 'C:/runs/vectorgrid_run_1.json',
        options_dataset_id: 'spy_options',
        delay_minutes: [0, 1],
        dte_buckets: [1, 3],
        moneyness_buckets: ['atm', 'otm_1'],
        hold_minutes: [15, 30],
        target_return_buckets: [0.25],
        stop_return_buckets: [0.25],
        exit_template: 'fixed_hold_or_target_stop',
        fill_model: 'aggregate_research',
        filters: {
          max_entry_lag_minutes: 5,
        },
        top_k: 25,
        split_ratio_search: 0.6,
        split_ratio_holdout: 0.2,
      },
      diagnostics: {
        parent_artifact_path: 'C:/runs/vectorgrid_run_1.json',
        options_dataset_id: 'spy_options',
        root_symbol: 'SPY',
        total_events: 10,
        bullish_events: 6,
        bearish_events: 4,
        search_events: 6,
        holdout_events: 2,
        sealed_test_events: 2,
        dataset_contracts: 3,
        dataset_rows: 42,
      },
      selection_metrics: [
        {
          tuple: {
            delay_minutes: 0,
            dte_days: 1,
            moneyness_bucket: 'atm',
            hold_minutes: 15,
            target_return: 0.25,
            stop_return: 0.25,
            exit_template: 'fixed_hold_or_target_stop',
            fill_model: 'aggregate_research',
          },
          selection_metrics: {
            valid_trades: 5,
            invalid_events: 1,
            invalid_rate: 0.1667,
            expectancy_per_trade: 12,
            expectancy_per_premium: 0.2,
            median_return_on_premium: 0.15,
            win_rate: 0.6,
            total_pnl_per_contract: 60,
          },
          robust_score: 0.18,
        },
      ],
    })

    const artifact = await loadOptionsExpressionArtifact('C:/runs/vectorgrid_options_expression_1.json')

    expect(artifact.kind).toBe('vectorgrid_options_expression_v1')
    expect(artifact.staging?.candidateSlotCount).toBe(12)
    expect(artifact.backendFallbackReason).toBe('GPU planner probe failed; using CPU.')
    expect(artifact.request.splitAssignmentMode).toBe('event_chronological')
    expect(artifact.request.delayMinutes).toEqual([0, 1])
    expect(artifact.selectionMetrics[0]?.moneynessBucket).toBe('atm')
  })

  it('normalizes tuple inspection responses', async () => {
    invokeCommandMock.mockResolvedValueOnce({
      tuple_index: 1,
      tuple_count: 4,
      total_events: 3,
      executed_events: 2,
      candidate_missing_events: 1,
      outcome_missing_events: 0,
      tuple_result: {
        tuple: {
          delay_minutes: 5,
          dte_days: 3,
          moneyness_bucket: 'otm_1',
          hold_minutes: 30,
          target_return: 0.5,
          stop_return: 0.25,
          exit_template: 'fixed_hold_or_target_stop',
          fill_model: 'aggregate_research',
        },
        selection_metrics: {
          valid_trades: 2,
          invalid_events: 1,
          invalid_rate: 0.3333,
          expectancy_per_trade: 10,
          expectancy_per_premium: 0.11,
          median_return_on_premium: 0.08,
          win_rate: 0.5,
          total_pnl_per_contract: 20,
        },
        robust_score: 0.12,
      },
      rows: [
        {
          event_index: 0,
          event_id: 'event-1',
          split: 'search',
          side: 'bullish',
          decision_timestamp: 100,
          earliest_tradable_timestamp: 160,
          requested_entry_timestamp: 460,
          raw_underlier_price: 501.25,
          base_exit_timestamp: 700,
          status: 'executed',
          contract_id: 'SPY_C_1',
          expiration_date: '2026-04-12',
          strike: 505,
          multiplier: 100,
          entry_timestamp: 460,
          entry_option_price: 1.4,
          entry_underlier_price: 501.5,
          exit_timestamp: 520,
          exit_option_price: 1.8,
          premium_paid: 140,
          pnl_per_contract: 40,
          return_on_premium: 0.2857,
          entry_lag_minutes: 5,
          minutes_to_expiry_at_entry: 1440,
          realized_hold_minutes: 60,
          path_option_high: 1.95,
          path_option_low: 1.35,
          path_underlier_high: 503.1,
          path_underlier_low: 500.8,
          exit_reason: 'target_hit',
        },
        {
          event_index: 1,
          event_id: 'event-2',
          split: 'holdout',
          side: 'bearish',
          decision_timestamp: 200,
          earliest_tradable_timestamp: 260,
          requested_entry_timestamp: 560,
          raw_underlier_price: 498.75,
          status: 'candidate_missing',
          candidate_invalid_reason: 'no_expiration_in_bucket',
        },
      ],
    })

    const inspection = await inspectOptionsExpressionTuple({
      artifactPath: 'C:/runs/vectorgrid_options_expression_1.json',
      tupleIndex: 1,
    })

    expect(inspection.tupleIndex).toBe(1)
    expect(inspection.executedEvents).toBe(2)
    expect(inspection.rows[0]?.contractId).toBe('SPY_C_1')
    expect(inspection.rows[0]?.exitReason).toBe('target_hit')
    expect(inspection.rows[0]?.entryLagMinutes).toBe(5)
    expect(inspection.rows[0]?.minutesToExpiryAtEntry).toBe(1440)
    expect(inspection.rows[0]?.pathOptionHigh).toBe(1.95)
    expect(inspection.rows[1]?.status).toBe('candidate_missing')
    expect(inspection.rows[1]?.candidateInvalidReason).toBe('no_expiration_in_bucket')
    expect(invokeCommandMock).toHaveBeenCalledWith('inspect_options_expression_tuple', {
      req: {
        artifact_path: 'C:/runs/vectorgrid_options_expression_1.json',
        tuple_index: 1,
      },
    })
  })

  it('normalizes dataset id lists', () => {
    expect(normalizeOptionsDatasetIds([' spy ', '', 42, null])).toEqual(['spy', '42'])
  })

  it('lists dataset manifests', async () => {
    invokeCommandMock.mockResolvedValueOnce([
      {
        version: 1,
        options_dataset_id: 'spy_options',
        root_symbol: 'SPY',
      },
    ])

    const rows = await listOptionsDatasets()
    expect(rows).toHaveLength(1)
    expect(rows[0]?.optionsDatasetId).toBe('spy_options')
  })

  it('uses the CSV picker and open-path fallback helpers', async () => {
    invokeCommandMock
      .mockResolvedValueOnce(' C:/data/options.csv ')
      .mockRejectedValueOnce(new Error('open failed'))
      .mockResolvedValueOnce({})

    const picked = await pickOptionsCsvPath()
    const opened = await openOrRevealOptionsPath('C:/runs/vectorgrid_options_expression_1.json')

    expect(picked).toBe('C:/data/options.csv')
    expect(opened).toEqual({
      path: 'C:/runs/vectorgrid_options_expression_1.json',
      openedWith: 'reveal_path',
    })
  })

  it('exports ranked tuples to CSV', async () => {
    const createObjectURL = vi
      .spyOn(URL, 'createObjectURL')
      .mockImplementation(() => 'blob:options-expression')
    const revokeObjectURL = vi
      .spyOn(URL, 'revokeObjectURL')
      .mockImplementation(() => undefined)

    let clicked = 0
    const createElement = vi.spyOn(document, 'createElement')
    createElement.mockImplementation(((tagName: string) => {
      const element = document.createElementNS('http://www.w3.org/1999/xhtml', tagName)
      if (tagName.toLowerCase() === 'a') {
        ;(element as HTMLAnchorElement).click = () => {
          clicked += 1
        }
      }
      return element
    }) as typeof document.createElement)

    const exported = exportOptionsExpressionCsv({
      filenameStem: 'vectorgrid_options_expression_spy',
      rootSymbol: 'SPY',
      backendUsed: 'cpu_reference',
      priceSemantics: 'minute_aggregate',
      tuples: [
        {
          delayMinutes: 1,
          dteDays: 3,
          moneynessBucket: 'otm_1',
          holdMinutes: 15,
          targetReturn: 0.5,
          stopReturn: 0.25,
          exitTemplate: 'fixed_hold_or_target_stop',
          fillModel: 'aggregate_research',
          robustScore: 0.42,
          selectionMetrics: {
            validTrades: 12,
            invalidEvents: 2,
            invalidRate: 0.14,
            expectancyPerTrade: 8.5,
            expectancyPerPremium: 0.18,
            medianReturnOnPremium: 0.12,
            winRate: 0.58,
            totalPnlPerContract: 102,
          },
          holdoutMetrics: {
            validTrades: 4,
            invalidEvents: 0,
            invalidRate: 0,
            expectancyPerTrade: 5.5,
            expectancyPerPremium: 0.11,
            medianReturnOnPremium: 0.08,
            winRate: 0.5,
            totalPnlPerContract: 22,
          },
          sealedEvaluationMetrics: null,
          stressMetrics: null,
        },
      ],
    })

    expect(exported).toBe(true)
    expect(clicked).toBe(1)
    expect(createObjectURL).toHaveBeenCalledTimes(1)
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:options-expression')

    const blobArg = createObjectURL.mock.calls[0]?.[0]
    expect(blobArg).toBeInstanceOf(Blob)
    const csvBlob = blobArg instanceof Blob ? blobArg : null
    const csvText = await csvBlob?.text()
    expect(csvText).toContain('delay_minutes,dte_days,moneyness_bucket')
    expect(csvText).toContain('cpu_reference,minute_aggregate')
    expect(csvText).toContain('1,3,otm_1,15,0.5,0.25')
  })
})
