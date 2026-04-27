export interface MockTauriScenario {
  name: string
  description: string
  urlSearch?: string
  localStorage?: Record<string, string>
  sessionStorage?: Record<string, string>
  commandResults: Record<string, unknown>
  expectedInvokes?: string[]
}

export const baseCommandResults: Record<string, unknown> = {
  get_gpu_status: {
    built_with_cuda: false,
    built_with_cuda_backtest_kernel: false,
    cuda_available: false,
    auto_device_name: '',
    auto_device_memory_bytes: null,
    auto_device_error: 'CUDA disabled in browser QA fixture.',
  },
  set_shell_menu_command_states: null,
  list_moving_averages: [],
  list_gpu_speedups: [],
  list_dynamic_indicators: [],
  list_ml_signal_model_bundles: [],
  ml_signal_model_status: {
    state: 'unavailable',
    message: 'No ML signal bundle selected in browser QA fixture.',
  },
  list_sample_csvs: [
    {
      label: 'AAPL fixture',
      path: 'C:/fixtures/aapl_1m.csv',
      rows: 5000,
      timeframe: '1m',
      disabled: false,
    },
  ],
  get_market_data_dir_info: {
    root: 'C:/VectorGridData',
    exists: true,
    source: 'mock',
  },
  polygon_get_cache_info: {
    datasets: [],
  },
  get_artifact_retention_info: {
    defaults: {
      run_keep_latest: 200,
      meta_keep_latest: 100,
      stage5_keep_latest: 100,
      ml_policy_eval_keep_latest: 100,
      batch_summary_keep_latest: 100,
      campaign_summary_keep_latest: 100,
      options_expression_keep_latest: 100,
    },
    configured: {},
    env_overrides: {},
    effective: {
      run_keep_latest: 200,
      meta_keep_latest: 100,
      stage5_keep_latest: 100,
      ml_policy_eval_keep_latest: 100,
      batch_summary_keep_latest: 100,
      campaign_summary_keep_latest: 100,
      options_expression_keep_latest: 100,
    },
  },
  get_execution_risk_policy: {
    policy: {
      max_order_notional: null,
      max_position_notional: null,
      allow_short: null,
      require_bracket: null,
    },
  },
  list_artifact_library: [],
  list_repo_data_csvs: [],
  alpaca_get_credential_status: {
    remember_available: true,
    has_api_key: false,
    has_api_secret: false,
    has_credentials: false,
    state: 'missing',
    storage_backend: 'keyring',
    message: '',
  },
  live_execution_runner_status: {
    active: false,
    inflight: false,
    config: null,
  },
  polygon_live_candle_stream_status: {
    active: false,
    stream: null,
  },
}

export const emptyAppScenario: MockTauriScenario = {
  name: 'empty-app',
  description: 'Fresh app with no dataset, latest run, or saved live sessions.',
  localStorage: {
    vta_ui_mode_v1: 'advanced',
  },
  commandResults: {
    ...baseCommandResults,
    load_latest_run_artifact_hydration: null,
    load_latest_run_artifact_summary: null,
    list_live_session_artifacts: [],
    load_live_session_artifact_detail: null,
    list_run_artifacts: [],
  },
}

export const dataLoadFailureScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'data-load-failure',
  description: 'Sample CSV loading fails after the user activates the sample-data action.',
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_price_data: {
      __mockFailure: 'Fixture CSV read failed.',
    },
  },
  expectedInvokes: ['load_price_data'],
}

export const gpuStatusFailureScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'gpu-status-failure',
  description: 'GPU status probing fails after the user requests a refresh.',
  commandResults: {
    ...emptyAppScenario.commandResults,
    get_gpu_status: {
      __mockFailure: 'GPU probe failed in browser QA fixture.',
    },
  },
  expectedInvokes: ['get_gpu_status'],
}

export const validationDatasetMismatchScenario: MockTauriScenario = {
  name: 'validation-dataset-mismatch',
  description: 'A saved optimization artifact is loaded without its matching dataset.',
  localStorage: {
    vta_ui_mode_v1: 'advanced',
  },
  commandResults: {
    ...baseCommandResults,
    load_latest_run_artifact_summary: {
      ts: '2026-04-11T12:00:00Z',
      path: 'C:/runs/vectorgrid_run_mismatch.json',
    },
    load_latest_run_artifact_hydration: {
      summary: {
        ts: '2026-04-11T12:00:00Z',
        path: 'C:/runs/vectorgrid_run_mismatch.json',
        csv_path: 'C:/fixtures/aapl_1m.csv',
        tags: ['qa'],
      },
      hydration: {
        currentRun: {
          ts: '2026-04-11T12:00:00Z',
          artifact_path: 'C:/runs/vectorgrid_run_mismatch.json',
          csv_path: 'C:/fixtures/aapl_1m.csv',
          dataset_meta: {
            csv_path: 'C:/fixtures/aapl_1m.csv',
            timeframe: '1m',
            len: 5000,
          },
          tags: ['qa'],
          req: {
            strategy_kind: 'DoubleMa',
            data_id: 'fixture_aapl_1m',
            fast_start: 8,
            fast_end: 10,
            fast_step: 1,
            slow_start: 32,
            slow_end: 36,
            slow_step: 2,
          },
          res: {
            best_params: {
              fast_len: 8,
              slow_len: 32,
              fast_ma_type: 'sma',
              slow_ma_type: 'sma',
            },
            best_metrics: {
              pnl: 0.12,
              sharpe: 1.4,
              max_dd: -0.05,
              trades: 18,
            },
            top: [
              [
                {
                  fast_len: 8,
                  slow_len: 32,
                  fast_ma_type: 'sma',
                  slow_ma_type: 'sma',
                },
                {
                  pnl: 0.12,
                  sharpe: 1.4,
                  max_dd: -0.05,
                  trades: 18,
                },
              ],
            ],
            runtime_ms: 84,
            backend_used: 'cpu',
            backend_route: 'CpuOnly',
          },
        },
        currentValidation: null,
      },
    },
    list_run_artifacts: [],
    list_live_session_artifacts: [],
    load_live_session_artifact_detail: null,
  },
}

const validationSuccessRunPath = 'C:/runs/vectorgrid_run_validation_success.json'
const validationSuccessCsvPath = 'C:/fixtures/aapl_1m.csv'
const validationSuccessDatasetMeta = {
  csv_path: validationSuccessCsvPath,
  timeframe: '1m',
  len: 5000,
  symbol: 'AAPL',
  label: 'AAPL fixture',
  has_open: true,
  has_high: true,
  has_low: true,
  has_close: true,
  has_volume: true,
  min_ts: 1904164200000,
  max_ts: 1904250600000,
}

const validationSuccessRunReq = {
  strategy_kind: 'DoubleMa',
  data_id: 'fixture_aapl_1m',
  fast_start: 8,
  fast_end: 10,
  fast_step: 1,
  slow_start: 32,
  slow_end: 36,
  slow_step: 2,
  fast_ma_types: ['sma'],
  slow_ma_types: ['sma'],
}

const validationSuccessRunRes = {
  best_params: {
    fast_len: 8,
    slow_len: 32,
    fast_ma_type: 'sma',
    slow_ma_type: 'sma',
  },
  best_metrics: {
    pnl: 0.12,
    sharpe: 1.4,
    max_dd: -0.05,
    trades: 18,
  },
  top: [
    [
      {
        fast_len: 8,
        slow_len: 32,
        fast_ma_type: 'sma',
        slow_ma_type: 'sma',
      },
      {
        pnl: 0.12,
        sharpe: 1.4,
        max_dd: -0.05,
        trades: 18,
      },
    ],
  ],
  runtime_ms: 84,
  backend_used: 'cpu',
  backend_route: 'CpuOnly',
}

const validationSuccessRunArtifact = {
  ts: '2026-04-11T12:00:00Z',
  artifact_path: validationSuccessRunPath,
  report_xlsx_path: '',
  report_xlsx_exists: false,
  csv_path: validationSuccessCsvPath,
  dataset_meta: validationSuccessDatasetMeta,
  tags: ['qa', 'validation'],
  req: validationSuccessRunReq,
  res: validationSuccessRunRes,
}

const validationSuccessResult = {
  backend_used: 'Cpu',
  backend_route: 'CpuOnly',
  backend_fallback_reason: '',
  runtime_ms: 42,
  candidates: [
    {
      params: {
        fast_len: 8,
        slow_len: 32,
        fast_ma_type: 'sma',
        slow_ma_type: 'sma',
      },
      in_sample: {
        pnl: 0.12,
        sharpe: 1.4,
        max_dd: -0.05,
        trades: 18,
      },
      robust_score: 1.21,
      psr: 0.91,
      dsr: 0.84,
      robust_pareto: true,
      validation_verdict: {
        passed: true,
        approved: true,
        final_recommendation: 'approved',
      },
    },
  ],
  cross_ticker: { datasets: [] },
  run_verdict: {
    passed: true,
    approved: true,
    candidates_tested: 1,
    candidates_survived_hard_gates: 1,
    approved_candidate_count: 1,
    first_approved_rank: 1,
    final_recommendation: 'approved',
  },
}

export const validationSuccessScenario: MockTauriScenario = {
  name: 'validation-success',
  description: 'A saved DoubleMa run and matching loaded sample dataset can complete validation.',
  localStorage: {
    vta_ui_mode_v1: 'advanced',
  },
  commandResults: {
    ...baseCommandResults,
    load_latest_run_artifact_summary: {
      ts: '2026-04-11T12:00:00Z',
      path: validationSuccessRunPath,
      csv_path: validationSuccessCsvPath,
      tags: ['qa', 'validation'],
    },
    load_latest_run_artifact_hydration: {
      summary: {
        ts: '2026-04-11T12:00:00Z',
        path: validationSuccessRunPath,
        csv_path: validationSuccessCsvPath,
        tags: ['qa', 'validation'],
      },
      hydration: {
        currentRun: validationSuccessRunArtifact,
        currentValidation: null,
      },
    },
    load_price_data: {
      id: 'fixture_aapl_1m',
      meta: validationSuccessDatasetMeta,
      cache_hit: true,
      cache_path: 'C:/VectorGridData/cache/aapl_1m.vta',
      load_source: 'binary_cache',
    },
    run_double_ma_validation: validationSuccessResult,
    save_run_artifact: {
      path: validationSuccessRunPath,
      bytes: 73728,
    },
    list_run_artifacts: [],
    list_live_session_artifacts: [],
    load_live_session_artifact_detail: null,
  },
  expectedInvokes: [
    'load_latest_run_artifact_hydration',
    'load_price_data',
    'run_double_ma_validation',
    'save_run_artifact',
  ],
}

const optionsParentRunPath = 'C:/runs/vectorgrid_options_parent.json'
const optionsParentRunId = 'vectorgrid_options_parent'
const optionsExpressionArtifactPath = 'C:/runs/vectorgrid_options_parent_options_expression.json'
const optionsValidationArtifactPath = 'C:/runs/vectorgrid_options_parent_options_validation.json'
const optionsDatasetId = 'AAPL_OPTIONS_AGG_QA'
const optionsDatasetHash = 'sha256:aapl-options-agg-fixture'

function optionsMetrics(expectancyPerPremium: number, validTrades = 16): Record<string, unknown> {
  return {
    valid_trades: validTrades,
    invalid_events: 1,
    invalid_rate: 0.05,
    expectancy_per_trade: expectancyPerPremium * 1.8,
    expectancy_per_premium: expectancyPerPremium,
    mean_return_on_premium: expectancyPerPremium,
    return_volatility_on_premium: 0.17,
    median_return_on_premium: expectancyPerPremium * 0.8,
    win_rate: 0.61,
    total_pnl_per_contract: expectancyPerPremium * 100,
  }
}

const optionsTupleFixture = {
  structure_family: 'long_single',
  delay_minutes: 1,
  dte_days: 3,
  moneyness_bucket: 'atm',
  hold_minutes: 15,
  target_return: 0.5,
  stop_return: 0.25,
  exit_template: 'fixed_hold_or_target_stop',
  fill_model: 'aggregate_research',
  pre_sealed_rank: 1,
  sealed_finalist: true,
  sealed_final_rank: 1,
  validation_verdict: 'sealed_finalist',
  robust_score: 1.32,
  selection_metrics: optionsMetrics(0.42, 18),
  holdout_metrics: optionsMetrics(0.31, 9),
  sealed_evaluation_metrics: optionsMetrics(0.28, 6),
  stress_metrics: optionsMetrics(0.2, 8),
  harsher_stress_metrics: optionsMetrics(0.16, 7),
  extreme_stress_metrics: optionsMetrics(0.11, 5),
}

const optionsDatasetFixture = {
  version: 1,
  options_dataset_id: optionsDatasetId,
  root_symbol: 'AAPL',
  provider_name: 'polygon.io',
  provider_source_surface: 'polygon_options_download',
  provider_entitlement_profile: 'starter',
  quote_resolution: 'minute',
  price_semantics: 'minute_aggregate',
  calendar_id: 'XNYS',
  timezone: 'America/New_York',
  coverage_start: 1904164200000,
  coverage_end: 1904250600000,
  normalization_version: 1,
  quote_semantics_version: 1,
  corporate_action_mapping_version: 1,
  contract_reference_source: 'polygon_contract_reference',
  contract_reference_as_of_utc: '2026-04-24T00:00:00Z',
  underlier_reference_source: 'polygon_underlying_aggregates',
  dataset_path: 'C:/VectorGridData/options/aapl_options_agg_fixture.csv',
  row_count: 2400,
  contract_count: 48,
  aggregate_price_field: 'close',
}

const optionsArtifactSummaryFixture = {
  ts: '2026-04-24T14:45:00Z',
  path: optionsExpressionArtifactPath,
  bytes: 65536,
  parent_artifact_id: optionsParentRunId,
  parent_strategy_kind: 'DoubleMa',
  root_symbol: 'AAPL',
  options_dataset_id: optionsDatasetId,
  backend_used: 'cpu',
  backend_fallback_reason: '',
  price_semantics: 'minute_aggregate',
  request_backend_preference: 'auto',
  request_fill_model: 'aggregate_research',
  request_structure_family: 'long_single',
  request_split_assignment_mode: 'event_chronological',
  coverage_adequacy_verdict: 'adequate',
  coverage_non_promotable: false,
  search_overlap_rate: 1,
  holdout_overlap_rate: 1,
  sealed_test_overlap_rate: 1,
  search_candidate_coverage_rate: 0.96,
  holdout_candidate_coverage_rate: 0.91,
  sealed_test_candidate_coverage_rate: 0.88,
  search_baseline_outcome_coverage_rate: 0.95,
  holdout_baseline_outcome_coverage_rate: 0.9,
  sealed_test_baseline_outcome_coverage_rate: 0.87,
  tuple_count: 1,
  best_robust_score: 1.32,
  best_selection_expectancy_per_premium: 0.42,
  best_holdout_expectancy_per_premium: 0.31,
  best_sealed_expectancy_per_premium: 0.28,
  best_stress_expectancy_per_premium: 0.2,
  best_harsher_stress_expectancy_per_premium: 0.16,
  best_extreme_stress_expectancy_per_premium: 0.11,
  leader_summary: {
    search_leader_tuple_index: 0,
    robust_leader_tuple_index: 0,
    holdout_leader_tuple_index: 0,
    sealed_leader_tuple_index: 0,
    stress_leader_tuple_index: 0,
    distinct_leader_count: 1,
    search_vs_robust_disagree: false,
    robust_vs_holdout_disagree: false,
    robust_vs_sealed_disagree: false,
    robust_leader_reached_sealed_finalist: true,
    robust_leader_sealed_final_rank: 1,
  },
  top_validation_verdict: 'sealed_finalist',
  sealed_finalist_budget: 1,
  sealed_evaluated_tuple_count: 1,
  validated_tuple_count: 1,
  stress_fragile_tuple_count: 0,
  supported_holdout_tuple_count: 1,
  supported_sealed_tuple_count: 1,
  supported_stress_tuple_count: 1,
  has_holdout_metrics: true,
  has_sealed_evaluation_metrics: true,
  has_stress_metrics: true,
  has_harsher_stress_metrics: true,
  has_extreme_stress_metrics: true,
}

const optionsArtifactRecordFixture = {
  version: 1,
  kind: 'options_expression_artifact',
  ts: '2026-04-24T14:45:00Z',
  artifact_path: optionsExpressionArtifactPath,
  parent_artifact_id: optionsParentRunId,
  root_symbol: 'AAPL',
  options_dataset_id: optionsDatasetId,
  options_dataset_hash: optionsDatasetHash,
  backend_used: 'cpu',
  backend_fallback_reason: '',
  price_semantics: 'minute_aggregate',
  request: {
    parent_artifact_path: optionsParentRunPath,
    options_dataset_id: optionsDatasetId,
    structure_family: 'long_single',
    backend_preference: 'auto',
    split_assignment_mode: 'event_chronological',
    delay_minutes: [0, 1, 5],
    dte_buckets: [1, 3, 5],
    moneyness_buckets: ['atm'],
    hold_minutes: [15],
    target_return_buckets: [0.5],
    stop_return_buckets: [0.25],
    exit_template: 'fixed_hold_or_target_stop',
    fill_model: 'aggregate_research',
    filters: {
      max_entry_lag_minutes: 5,
      max_premium: null,
      min_premium: null,
      min_volume: null,
      min_open_interest: null,
    },
    top_k: 25,
    min_valid_trades: 5,
    split_ratio_search: 0.6,
    split_ratio_holdout: 0.2,
    sealed_finalist_count: 1,
  },
  diagnostics: {
    parent_artifact_path: optionsParentRunPath,
    options_dataset_id: optionsDatasetId,
    root_symbol: 'AAPL',
    total_events: 34,
    bullish_events: 20,
    bearish_events: 14,
    search_events: 20,
    holdout_events: 8,
    sealed_test_events: 6,
  },
  staging: null,
  overlap_window: {
    overlap_start: 1904164200000,
    overlap_end: 1904250600000,
    search_overlap_rate: 1,
    holdout_overlap_rate: 1,
    sealed_test_overlap_rate: 1,
  },
  coverage_adequacy: {
    verdict: 'adequate',
    non_promotable: false,
    notes: ['Fixture coverage is adequate for browser QA.'],
  },
  split_invalid_reasons: {},
  selection_metrics: [optionsTupleFixture],
}

const optionsValidationFeasibilityFixture = {
  replay_build_ms: 4,
  artifact_path: optionsExpressionArtifactPath,
  source_artifact_hash: 'sha256:options-expression-artifact',
  options_dataset_id: optionsDatasetId,
  options_dataset_hash: optionsDatasetHash,
  dataset_policy: 'require_exact_replay',
  replacement_dataset: false,
  validation_plan: {
    mode: 'date_range_holdout',
    start_timestamp_ms: Date.parse('2030-04-25T00:00:00.000Z'),
    end_timestamp_ms: Date.parse('2030-04-26T00:00:00.000Z'),
    train_session_count: null,
    validation_session_count: null,
    step_session_count: null,
  },
  tuple_selection: {
    include_sealed_finalists: true,
    include_robust_leader: true,
    include_holdout_leader: true,
    tuple_indexes: [0],
  },
  selected_tuple_indexes: [0],
  selected_tuple_count: 1,
  price_semantics: 'minute_aggregate',
  dataset_provenance: {
    provider_name: 'polygon.io',
    provider_source_surface: 'polygon_options_download',
    quote_resolution: 'minute',
    aggregate_price_field: 'close',
    contract_reference_as_of_utc: '2026-04-24T00:00:00Z',
  },
  notes: ['Selected sealed finalist and robust leader tuple.', 'Validation will assign 8 events in the end-exclusive date range to holdout.'],
}

const optionsValidationResultFixture = {
  artifact_path: optionsValidationArtifactPath,
  replay_build_ms: 4,
  validation_ms: 7,
  result: {
    version: 1,
    kind: 'options_expression_validation_artifact',
    ts: '2026-04-24T15:00:00Z',
    validation_artifact_path: optionsValidationArtifactPath,
    source_artifact_path: optionsExpressionArtifactPath,
    source_artifact_hash: 'sha256:options-expression-artifact',
    parent_artifact_id: optionsParentRunId,
    parent_artifact_hash: 'sha256:parent-run',
    signal_manifest_id: 'signal-aapl-qa',
    root_symbol: 'AAPL',
    options_dataset_id: optionsDatasetId,
    options_dataset_hash: optionsDatasetHash,
    dataset_policy: 'require_exact_replay',
    replacement_dataset: false,
    validation_plan: optionsValidationFeasibilityFixture.validation_plan,
    tuple_selection: optionsValidationFeasibilityFixture.tuple_selection,
    price_semantics: 'minute_aggregate',
    dataset_provenance: optionsValidationFeasibilityFixture.dataset_provenance,
    diagnostics: optionsArtifactRecordFixture.diagnostics,
    overlap_window: optionsArtifactRecordFixture.overlap_window,
    coverage_adequacy: optionsArtifactRecordFixture.coverage_adequacy,
    lane_summaries: [
      {
        lane: 'holdout',
        tuple_count: 1,
        supported_tuple_count: 1,
        support_deficient_tuple_count: 0,
        min_valid_trades: 5,
        best_expectancy_per_premium: 0.3,
        worst_expectancy_per_premium: 0.3,
        median_expectancy_per_premium: 0.3,
      },
      {
        lane: 'stress',
        tuple_count: 1,
        supported_tuple_count: 1,
        support_deficient_tuple_count: 0,
        min_valid_trades: 5,
        best_expectancy_per_premium: 0.19,
        worst_expectancy_per_premium: 0.19,
        median_expectancy_per_premium: 0.19,
      },
    ],
    acceptance_summary: {
      verdict: 'accepted',
      selected_tuple_count: 1,
      validated_tuple_count: 1,
      accepted_tuple_count: 1,
      support_deficient_tuple_count: 0,
      stress_fragile_tuple_count: 0,
      min_valid_trades: 5,
      best_validated_expectancy_per_premium: 0.3,
      worst_stress_expectancy_per_premium: 0.19,
      notes: ['Accepted with supported holdout and stress lanes.'],
    },
    tuple_results: [
      {
        source_tuple_index: 0,
        source_validation_verdict: 'sealed_finalist',
        source_robust_score: 1.32,
        validated_tuple: {
          ...optionsTupleFixture,
          holdout_metrics: optionsMetrics(0.3, 9),
          stress_metrics: optionsMetrics(0.19, 8),
        },
        support_deficient: false,
        drift: {
          selection_expectancy_delta: -0.01,
          holdout_expectancy_delta: -0.01,
          sealed_expectancy_delta: 0,
          stress_expectancy_delta: -0.01,
          extreme_stress_expectancy_delta: 0,
          robust_score_delta: -0.03,
        },
      },
    ],
  },
}

export const optionsValidationReadyScenario: MockTauriScenario = {
  name: 'options-validation-ready',
  description: 'A saved DoubleMa run has a matching Polygon.io options artifact ready for validation.',
  localStorage: {
    vta_ui_mode_v1: 'advanced',
  },
  commandResults: {
    ...emptyAppScenario.commandResults,
    list_run_artifacts: [
      {
        ts: '2026-04-24T14:30:00Z',
        path: optionsParentRunPath,
        bytes: 49152,
        strategy_kind: 'DoubleMa',
        config_fingerprint: 'double-ma-aapl-qa',
        config_fingerprint_version: 1,
        has_validation: true,
        report_xlsx_path: 'C:/runs/vectorgrid_options_parent.xlsx',
        report_xlsx_exists: true,
        csv_path: 'C:/fixtures/aapl_1m.csv',
        timeframe: '1m',
        tags: ['qa', 'options'],
      },
    ],
    load_run_artifact_detail: {
      ts: '2026-04-24T14:30:00Z',
      artifactPath: optionsParentRunPath,
      strategyKind: 'DoubleMa',
      configFingerprint: 'double-ma-aapl-qa',
      configFingerprintVersion: 1,
      timeframe: '1m',
      csvPath: 'C:/fixtures/aapl_1m.csv',
      reportXlsxPath: 'C:/runs/vectorgrid_options_parent.xlsx',
      reportXlsxExists: true,
      hasValidation: true,
      datasetMeta: {
        csv_path: 'C:/fixtures/aapl_1m.csv',
        timeframe: '1m',
        len: 5000,
      },
      runReq: {
        strategy_kind: 'DoubleMa',
        data_id: 'fixture_aapl_1m',
        fast_start: 8,
        fast_end: 10,
        fast_step: 1,
        slow_start: 32,
        slow_end: 36,
        slow_step: 2,
      },
      runRes: {
        best_params: {
          fast_len: 8,
          slow_len: 32,
          fast_ma_type: 'sma',
          slow_ma_type: 'sma',
        },
        best_metrics: {
          pnl: 0.12,
          sharpe: 1.4,
          max_dd: -0.05,
          trades: 18,
        },
        top: [],
        runtime_ms: 84,
        backend_used: 'cpu',
        backend_route: 'CpuOnly',
      },
      paramsText: '{"strategy_kind":"DoubleMa","data_id":"fixture_aapl_1m"}',
      tags: ['qa', 'options'],
    },
    repair_run_artifact: {
      source_path: optionsParentRunPath,
      result_path: '',
      dry_run: true,
      changed: true,
      wrote: false,
      changes: ['normalized saved-run summary fields', 'canonicalized request payload'],
      summary: {
        ts: '2026-04-24T14:30:00Z',
        path: optionsParentRunPath,
        bytes: 49152,
        strategy_kind: 'DoubleMa',
        config_fingerprint: 'double-ma-aapl-qa',
        config_fingerprint_version: 1,
        has_validation: true,
        report_xlsx_path: 'C:/runs/vectorgrid_options_parent.xlsx',
        report_xlsx_exists: true,
        csv_path: 'C:/fixtures/aapl_1m.csv',
        timeframe: '1m',
        tags: ['qa', 'options'],
      },
    },
    list_options_datasets: [optionsDatasetFixture],
    list_artifact_library: {
      optionsExpression: [optionsArtifactSummaryFixture],
      options_expression: [optionsArtifactSummaryFixture],
    },
    load_options_expression_artifact: optionsArtifactRecordFixture,
    analyze_options_expression_validation_feasibility: optionsValidationFeasibilityFixture,
    validate_options_expression_artifact: optionsValidationResultFixture,
    reveal_path: null,
  },
  expectedInvokes: [
    'list_run_artifacts',
    'load_run_artifact_detail',
    'list_options_datasets',
    'list_artifact_library',
    'load_options_expression_artifact',
    'analyze_options_expression_validation_feasibility',
    'validate_options_expression_artifact',
    'reveal_path',
  ],
}

const optionsValidationSetupInvokes = [
  'list_run_artifacts',
  'load_run_artifact_detail',
  'list_options_datasets',
  'list_artifact_library',
  'load_options_expression_artifact',
]

export const optionsValidationInputFailureScenario: MockTauriScenario = {
  ...optionsValidationReadyScenario,
  name: 'options-validation-input-failure',
  description: 'A saved options artifact is loaded, then local validation-plan input fails before Tauri validation.',
  expectedInvokes: optionsValidationSetupInvokes,
}

export const optionsValidationPreviewFailureScenario: MockTauriScenario = {
  ...optionsValidationReadyScenario,
  name: 'options-validation-preview-failure',
  description: 'A saved options artifact is loaded, then validation feasibility preview fails through mocked Tauri.',
  commandResults: {
    ...optionsValidationReadyScenario.commandResults,
    analyze_options_expression_validation_feasibility: {
      __mockFailure: 'Options validation feasibility fixture failed.',
    },
  },
  expectedInvokes: [
    ...optionsValidationSetupInvokes,
    'analyze_options_expression_validation_feasibility',
  ],
}

export const liveReadyScenario: MockTauriScenario = {
  name: 'live-ready',
  description: 'Live workspace has credential status, stream status, and broker review data.',
  localStorage: {
    vta_ui_mode_v1: 'advanced',
  },
  commandResults: {
    ...emptyAppScenario.commandResults,
    alpaca_get_credential_status: {
      remember_available: true,
      has_api_key: false,
      has_api_secret: false,
      has_credentials: false,
      state: 'missing',
      storage_backend: 'keyring',
      message: '',
    },
    get_live_execution_snapshot: {
      symbol: 'AAPL',
      account_number: 'ABC123',
      account_status: 'ACTIVE',
      currency: 'USD',
      buying_power: '100000',
      cash: '25000',
      portfolio_value: '125000',
      fetched_at_unix_ms: 1900000000000,
      position: null,
      orders: [],
    },
    list_live_session_artifacts: [],
    load_live_session_artifact_detail: null,
  },
}
