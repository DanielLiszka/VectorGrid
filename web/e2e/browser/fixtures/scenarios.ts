import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import type { MockTauriScenario } from './types'

export type { FixtureFamilyId, MockTauriScenario } from './types'

const fixtureDir = path.dirname(fileURLToPath(import.meta.url))

function loadArtifactFixturePack<T>(fileName: string): T {
  const filePath = path.resolve(fixtureDir, 'artifacts', fileName)
  return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T
}

interface OptionsArtifactFixturePack {
  savedRunSummaries: unknown[]
  repairedRunSummaries: unknown[]
  parentRunDetail: Record<string, unknown>
  repairDryRun: Record<string, unknown>
  repairWrite: Record<string, unknown>
  optionsDatasets: unknown[]
  optionsArtifactSummaries: unknown[]
  optionsExpressionArtifact: Record<string, unknown>
  optionsValidationFeasibility: Record<string, unknown>
  optionsValidationResult: Record<string, unknown>
}

export const savedOptionsArtifactFixturePack =
  loadArtifactFixturePack<OptionsArtifactFixturePack>('saved-options-artifact-pack.json')

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
      run_keep_newest: 200,
      meta_keep_newest: 100,
      stage5_keep_newest: 100,
      ml_policy_eval_keep_newest: 100,
      batch_summary_keep_newest: 100,
      campaign_summary_keep_newest: 100,
      options_expression_keep_newest: 100,
    },
    configured: {},
    env_overrides: {},
    effective: {
      run_keep_newest: 200,
      meta_keep_newest: 100,
      stage5_keep_newest: 100,
      ml_policy_eval_keep_newest: 100,
      batch_summary_keep_newest: 100,
      campaign_summary_keep_newest: 100,
      options_expression_keep_newest: 100,
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
  family: 'empty',
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
  family: 'dataset',
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
  family: 'settings',
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
  family: 'validation',
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
  family: 'validation',
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

export const optionsValidationReadyScenario: MockTauriScenario = {
  name: 'options-validation-ready',
  family: 'options',
  description: 'A saved DoubleMa run has a matching Polygon.io options artifact ready for validation.',
  localStorage: {
    vta_ui_mode_v1: 'advanced',
  },
  commandResults: {
    ...emptyAppScenario.commandResults,
    list_run_artifacts: savedOptionsArtifactFixturePack.savedRunSummaries,
    load_run_artifact_detail: savedOptionsArtifactFixturePack.parentRunDetail,
    repair_run_artifact: savedOptionsArtifactFixturePack.repairDryRun,
    list_options_datasets: savedOptionsArtifactFixturePack.optionsDatasets,
    list_artifact_library: {
      optionsExpression: savedOptionsArtifactFixturePack.optionsArtifactSummaries,
      options_expression: savedOptionsArtifactFixturePack.optionsArtifactSummaries,
    },
    load_options_expression_artifact: savedOptionsArtifactFixturePack.optionsExpressionArtifact,
    analyze_options_expression_validation_feasibility: savedOptionsArtifactFixturePack.optionsValidationFeasibility,
    validate_options_expression_artifact: savedOptionsArtifactFixturePack.optionsValidationResult,
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
  family: 'options',
  description: 'A saved options artifact is loaded, then local validation-plan input fails before Tauri validation.',
  expectedInvokes: optionsValidationSetupInvokes,
}

export const optionsValidationPreviewFailureScenario: MockTauriScenario = {
  ...optionsValidationReadyScenario,
  name: 'options-validation-preview-failure',
  family: 'options',
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
  family: 'liveExecution',
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
    get_local_paper_execution_snapshot: {
      symbol: 'AAPL',
      account_number: 'local-paper',
      account_status: 'simulated',
      currency: 'USD',
      cash: '100000.00',
      buying_power: '100000.00',
      portfolio_value: '100000.00',
      position: null,
      orders: [],
      fetched_at_unix_ms: 1_900_000_000_000,
    },
    list_live_session_artifacts: [],
    load_live_session_artifact_detail: null,
  },
  expectedInvokes: ['get_local_paper_execution_snapshot'],
}

export const liveSafetyReadyScenario: MockTauriScenario = {
  ...liveReadyScenario,
  name: 'live-safety-ready',
  family: 'liveExecution',
  description: 'Local-paper safety controls are confirmable while a live candle stream is active.',
  commandResults: {
    ...liveReadyScenario.commandResults,
    polygon_live_candle_stream_status: {
      active: true,
      stream: {
        symbol: 'AAPL',
        delayed: true,
        market: 'stocks',
        status: 'started',
        last_message: 'started live candle stream task',
        last_status_at_unix_ms: 1_900_000_000_000,
      },
    },
    get_local_paper_execution_snapshot: {
      symbol: 'AAPL',
      account_number: 'local-paper',
      account_status: 'simulated',
      currency: 'USD',
      cash: '100000.00',
      buying_power: '100000.00',
      portfolio_value: '100000.00',
      position: null,
      orders: [],
      fetched_at_unix_ms: 1_900_000_000_000,
    },
    cancel_local_paper_open_orders: {
      cancelled_count: 1,
      open_order_count_before: 1,
    },
    flatten_local_paper_position: {
      position_found: true,
      order_submitted: true,
      action: 'sell',
      order_id: 'lp_flat_1',
      qty: '1',
    },
    polygon_start_live_candle_stream: {
      symbol: 'AAPL',
      channel: 'A.AAPL',
      delayed: true,
    },
    emergency_halt_live_execution: {
      live_stopped: true,
      runner_stopped: false,
      live_symbol: 'AAPL',
      status_message: 'Emergency halt applied.',
      requires_manual_review: true,
    },
  },
  expectedInvokes: [
    'polygon_start_live_candle_stream',
    'cancel_local_paper_open_orders',
    'flatten_local_paper_position',
    'emergency_halt_live_execution',
  ],
}
