import { expect } from '@playwright/test'
import fs from 'node:fs/promises'
import path from 'node:path'
import {
  optionsValidationReadyScenario,
  savedOptionsArtifactFixturePack,
  type MockTauriScenario,
} from '../fixtures'
import {
  defineWorkflow,
  readTauriInvokeLog,
  type WorkflowContext,
} from '../support/workflowRunner'
import {
  expectNoHorizontalDocumentOverflow,
  expectNoStuckBusyState,
} from '../support/qaAssertions'

type ActionLookup = WorkflowContext['action']

const optionsArtifact = objectValue(
  savedOptionsArtifactFixturePack.optionsExpressionArtifact,
)
const rankedTuples = arrayValue(optionsArtifact.selection_metrics)
const artifactPath = textValue(optionsArtifact.artifact_path)

const tupleInspectionRows = [
  {
    event_index: 0,
    event_id: 'AAPL-20260424-001',
    split: 'search',
    side: 'long',
    decision_timestamp: 1904164200000,
    earliest_tradable_timestamp: 1904164260000,
    requested_entry_timestamp: 1904164500000,
    raw_underlier_price: 182.42,
    base_exit_timestamp: 1904165400000,
    status: 'executed',
    candidate_invalid_reason: '',
    contract_id: 'AAPL260424C00182500',
    companion_contract_id: '',
    expiration_date: '2026-04-24',
    strike: 182.5,
    companion_strike: null,
    multiplier: 100,
    entry_timestamp: 1904164560000,
    entry_option_price: 2.45,
    companion_entry_option_price: null,
    entry_underlier_price: 182.47,
    exit_timestamp: 1904165400000,
    exit_option_price: 3.12,
    companion_exit_option_price: null,
    premium_paid: 245,
    pnl_per_contract: 67,
    return_on_premium: 0.2735,
    entry_lag_minutes: 1,
    minutes_to_expiry_at_entry: 390,
    realized_hold_minutes: 14,
    path_option_high: 3.22,
    path_option_low: 2.31,
    path_underlier_high: 183.15,
    path_underlier_low: 182.11,
    exit_reason: 'target_return',
  },
  {
    event_index: 1,
    event_id: 'AAPL-20260424-002',
    split: 'holdout',
    side: 'long',
    decision_timestamp: 1904171400000,
    earliest_tradable_timestamp: 1904171460000,
    requested_entry_timestamp: 1904171700000,
    raw_underlier_price: 181.9,
    base_exit_timestamp: 1904172600000,
    status: 'candidate_missing',
    candidate_invalid_reason: 'strike_window_empty',
    contract_id: '',
    companion_contract_id: '',
    expiration_date: '',
    strike: null,
    companion_strike: null,
    multiplier: null,
    entry_timestamp: null,
    entry_option_price: null,
    companion_entry_option_price: null,
    entry_underlier_price: null,
    exit_timestamp: null,
    exit_option_price: null,
    companion_exit_option_price: null,
    premium_paid: null,
    pnl_per_contract: null,
    return_on_premium: null,
    entry_lag_minutes: null,
    minutes_to_expiry_at_entry: null,
    realized_hold_minutes: null,
    path_option_high: null,
    path_option_low: null,
    path_underlier_high: null,
    path_underlier_low: null,
    exit_reason: '',
  },
]

const optionsBacktestValidationScenario: MockTauriScenario = {
  ...optionsValidationReadyScenario,
  name: 'options-backtest-validation-full',
  description:
    'Saved parent run supports a full options-expression optimization, artifact replay, tuple inspection, event-path replay, and validation pass.',
  commandResults: {
    ...optionsValidationReadyScenario.commandResults,
    analyze_options_expression_feasibility: makeFeasibilityResponse(),
    run_options_expression: makeRunOptionsExpressionResponse(),
    inspect_options_expression_tuple: makeTupleInspectionResponse(),
    inspect_options_expression_event_path: makeEventPathInspectionResponse(),
    pick_save_csv: 'C:/exports/vectorgrid_options_expression_qa.csv',
    save_options_csv: {
      path: 'C:/exports/vectorgrid_options_expression_qa.csv',
      bytes: 4096,
    },
    reveal_path: null,
  },
  expectedInvokes: [
    'list_run_artifacts',
    'load_run_artifact_detail',
    'list_options_datasets',
    'analyze_options_expression_feasibility',
    'run_options_expression',
    'list_artifact_library',
    'pick_save_csv',
    'save_options_csv',
    'reveal_path',
    'load_options_expression_artifact',
    'inspect_options_expression_tuple',
    'inspect_options_expression_event_path',
    'analyze_options_expression_validation_feasibility',
    'validate_options_expression_artifact',
  ],
}

function objectValue(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' ? (value as Record<string, unknown>) : {}
}

function arrayValue(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function textValue(value: unknown): string {
  return String(value ?? '')
}

function makeRunOptionsExpressionResponse(): Record<string, unknown> {
  return {
    artifact_path: artifactPath,
    result: {
      backend_used: optionsArtifact.backend_used,
      backend_fallback_reason: optionsArtifact.backend_fallback_reason ?? '',
      price_semantics: optionsArtifact.price_semantics,
      tuple_count: rankedTuples.length,
      diagnostics: optionsArtifact.diagnostics,
      staging: optionsArtifact.staging,
      overlap_window: optionsArtifact.overlap_window,
      coverage_adequacy: optionsArtifact.coverage_adequacy,
      split_invalid_reasons: optionsArtifact.split_invalid_reasons,
      ranked_results: rankedTuples,
    },
    artifact: optionsArtifact,
  }
}

function makeFeasibilityResponse(): Record<string, unknown> {
  return {
    replay_build_ms: 4,
    total_preview_ms: 11,
    report: {
      diagnostics: optionsArtifact.diagnostics,
      price_semantics: 'minute_aggregate',
      fill_model: 'aggregate_research',
      best_available_backend: 'cpu_reference',
      auto_backend_decision: {
        recommended_backend: 'cpu_reference',
        gpu_eligible: false,
        reasons: ['Browser QA fixture runs the CPU reference path.'],
      },
      compiled_tuple_count: 1,
      per_event_candidate_slots: 9,
      per_candidate_exit_summaries: 3,
      candidate_slot_count: 306,
      compiled_candidate_count: 244,
      candidate_density: 0.7974,
      invalid_slot_count: 62,
      invalid_slot_rate: 0.2026,
      baseline_exit_summary_count: 244,
      baseline_compiled_exit_summary_count: 244,
      baseline_exit_summary_density: 1,
      conservative_exit_summary_count: 244,
      conservative_compiled_exit_summary_count: 232,
      conservative_exit_summary_density: 0.9508,
      harsher_exit_summary_count: 244,
      harsher_compiled_exit_summary_count: 219,
      harsher_exit_summary_density: 0.8975,
      projected_event_tuple_evaluations: 34,
      structural_cache_bytes_estimate: 65536,
      baseline_summary_bytes_estimate: 32768,
      conservative_summary_bytes_estimate: 32768,
      harsher_summary_bytes_estimate: 32768,
      timings: {
        tuple_compile_ms: 1,
        baseline_candidate_compile_ms: 2,
        baseline_exit_summary_compile_ms: 2,
        conservative_candidate_compile_ms: 1,
        conservative_exit_summary_compile_ms: 1,
        harsher_candidate_compile_ms: 1,
        harsher_exit_summary_compile_ms: 1,
        cpu_ranking_ms: 3,
        cpu_ranking_us: 3000,
        gpu_ranking_ms: null,
        gpu_ranking_us: null,
        gpu_tensor_score_ms: null,
        gpu_tensor_score_us: null,
        gpu_exactify_ms: null,
        gpu_exactify_us: null,
        gpu_sealed_stage_ms: null,
        gpu_sealed_stage_us: null,
        gpu_speedup_vs_cpu: null,
      },
      event_coverage: {
        total_events_with_candidates: 34,
        search_events_with_candidates: 20,
        holdout_events_with_candidates: 8,
        sealed_test_events_with_candidates: 6,
        total_events_with_baseline_outcomes: 33,
        search_events_with_baseline_outcomes: 20,
        holdout_events_with_baseline_outcomes: 8,
        sealed_test_events_with_baseline_outcomes: 5,
        total_events_with_conservative_outcomes: 32,
        search_events_with_conservative_outcomes: 19,
        holdout_events_with_conservative_outcomes: 8,
        sealed_test_events_with_conservative_outcomes: 5,
        total_events_with_harsher_outcomes: 31,
        search_events_with_harsher_outcomes: 18,
        holdout_events_with_harsher_outcomes: 8,
        sealed_test_events_with_harsher_outcomes: 5,
      },
      overlap_window: {
        ...objectValue(optionsArtifact.overlap_window),
        total_events_in_dataset_window: 34,
        search_events_in_dataset_window: 20,
        holdout_events_in_dataset_window: 8,
        sealed_test_events_in_dataset_window: 6,
        total_overlap_rate: 1,
      },
      split_invalid_reasons: {
        search_invalid_reasons: [],
        holdout_invalid_reasons: [
          { reason: 'strike_window_empty', count: 1, share: 0.125 },
        ],
        sealed_test_invalid_reasons: [],
      },
      coverage_adequacy: {
        verdict: 'healthy',
        non_promotable: false,
        reasons: ['Browser QA fixture has full overlap for all splits.'],
        search_candidate_coverage_rate: 1,
        holdout_candidate_coverage_rate: 1,
        sealed_test_candidate_coverage_rate: 1,
        search_baseline_outcome_coverage_rate: 1,
        holdout_baseline_outcome_coverage_rate: 1,
        sealed_test_baseline_outcome_coverage_rate: 0.8333,
        dominant_invalid_reason: 'strike_window_empty',
        dominant_invalid_reason_share: 0.125,
      },
      cache_stats: {
        structural_cache_hits: 3,
        structural_cache_misses: 1,
        structural_cache_entries: 1,
        summary_cache_hits: 2,
        summary_cache_misses: 1,
        summary_cache_entries: 1,
        gpu_shared_input_cache_hits: 0,
        gpu_shared_input_cache_misses: 0,
        gpu_outcome_input_cache_hits: 0,
        gpu_outcome_input_cache_misses: 0,
        gpu_output_buffer_reuses: 0,
        gpu_output_buffer_allocations: 0,
        gpu_tuple_chunk_count: 0,
        gpu_tuple_chunk_size: 0,
        gpu_kernel_launch_count: 0,
        gpu_tuple_budget_bytes: 0,
        gpu_outcome_tensor_bytes: 0,
        gpu_shared_input_bytes_per_chunk: 0,
        gpu_output_bytes_per_chunk: 0,
        gpu_total_device_bytes_per_chunk: 0,
        gpu_device_total_memory_bytes: 0,
        gpu_device_free_memory_bytes: 0,
        gpu_device_usable_memory_bytes: 0,
        gpu_memory_budget_source: '',
        gpu_outcome_tensor_fits_in_memory: false,
        gpu_chunk_plan_fits_in_memory: false,
        gpu_plan_notes: [],
      },
      invalid_reasons: [
        { reason: 'strike_window_empty', count: 1, share: 0.125 },
      ],
      axis_coverage: {
        delay_minutes: [
          {
            delay_minutes: 1,
            slot_count: 102,
            valid_slot_count: 94,
            invalid_slot_count: 8,
            valid_rate: 0.9216,
            invalid_reasons: [],
          },
        ],
        dte_days: [
          {
            dte_days: 3,
            slot_count: 102,
            valid_slot_count: 96,
            invalid_slot_count: 6,
            valid_rate: 0.9412,
            invalid_reasons: [],
          },
        ],
        moneyness_buckets: [
          {
            moneyness_bucket: 'atm',
            slot_count: 102,
            valid_slot_count: 98,
            invalid_slot_count: 4,
            valid_rate: 0.9608,
            invalid_reasons: [],
          },
        ],
      },
    },
  }
}

function makeTupleInspectionResponse(): Record<string, unknown> {
  return {
    tuple_index: 0,
    tuple_count: 1,
    tuple_result: rankedTuples[0],
    total_events: tupleInspectionRows.length,
    executed_events: 1,
    candidate_missing_events: 1,
    outcome_missing_events: 0,
    rows: tupleInspectionRows,
  }
}

function makeEventPathInspectionResponse(): Record<string, unknown> {
  return {
    tuple_index: 0,
    event_index: 0,
    event_id: 'AAPL-20260424-001',
    row: tupleInspectionRows[0],
    point_count: 4,
    points: [
      {
        timestamp_ms: 1904164560000,
        option_open: 2.45,
        option_high: 2.52,
        option_low: 2.41,
        option_close: 2.48,
        underlying_price: 182.47,
      },
      {
        timestamp_ms: 1904164860000,
        option_open: 2.48,
        option_high: 2.76,
        option_low: 2.43,
        option_close: 2.71,
        underlying_price: 182.8,
      },
      {
        timestamp_ms: 1904165160000,
        option_open: 2.71,
        option_high: 3.22,
        option_low: 2.69,
        option_close: 3.08,
        underlying_price: 183.15,
      },
      {
        timestamp_ms: 1904165400000,
        option_open: 3.08,
        option_high: 3.16,
        option_low: 3.02,
        option_close: 3.12,
        underlying_price: 183.02,
      },
    ],
  }
}

async function openSavedOptionsRun(
  page: WorkflowContext['page'],
  action: ActionLookup,
): Promise<void> {
  await action('shell.workspace.library').click()
  await action('library.savedRun.refresh').click()
  await expect(page.getByRole('button', { name: /DoubleMa/i }).first()).toBeVisible()
  await page.getByRole('button', { name: /DoubleMa/i }).first().click()

  await expect(page.locator('[aria-label="Options expression follow-up"]')).toBeVisible()
  await expect(page.locator('[data-qa-control="options.dataset.select"]')).toHaveValue('AAPL_OPTIONS_AGG_QA')
}

defineWorkflow({
  id: 'options.backtestOptimization.validation.full',
  title:
    'runs options backtest optimization, tuple inspection, event-path replay, and validation',
  tags: ['@workflow', '@regression'],
  scenario: optionsBacktestValidationScenario,
  viewport: { width: 1600, height: 1000 },
  run: async ({ page, action }) => {
    const evidenceDir = path.resolve(
      process.cwd(),
      '../output/playwright/options-backtest-validation',
    )
    await fs.mkdir(evidenceDir, { recursive: true })

    await openSavedOptionsRun(page, action)
    await action('options.datasets.refresh').click()
    await action('options.datasets.revealSelected').click()

    await action('options.feasibility.preview').click()
    await expect(page.getByText(/Feasibility preview compiled 1 tuple/i).first()).toBeVisible()
    await expect(page.getByText(/Phase 0\.5 preview/i).first()).toBeVisible()
    await expect(page.getByText(/GPU viability/i).first()).toBeVisible()

    await action('options.expression.run').click()
    await expect(page.getByText(/Options expression run completed with 1 ranked tuple/i).first()).toBeVisible()
    await expect(page.getByText(/Selected tuple/i).first()).toBeVisible()
    await expect(action('options.expression.exportCsv')).toBeEnabled()
    await action('options.expression.exportCsv').click()
    await expect(page.getByText(/Saved and revealed 1 ranked tuple/i).first()).toBeVisible()

    await page
      .locator('[aria-label="Options expression follow-up"]')
      .screenshot({ path: path.join(evidenceDir, 'optimization-result.png') })

    await action('options.artifacts.refresh').click()
    await expect(action('options.artifacts.load')).toBeEnabled()
    await action('options.artifacts.load').click()
    await expect(page.getByText(/Loaded saved options expression artifact/i).first()).toBeVisible()
    await action('options.artifacts.revealActive').click()

    await expect(action('options.tupleInspection.load')).toBeEnabled()
    await action('options.tupleInspection.load').click()
    await expect(page.getByText(/Tuple event detail/i).first()).toBeVisible()
    await expect(page.getByText(/Showing 2 of\s+2 event rows/i).first()).toBeVisible()
    await expect(page.getByText(/Split \/ status rollup/i).first()).toBeVisible()
    await expect(page.getByText(/strike_window_empty/i).first()).toBeVisible()
    await page
      .locator('[data-qa-control="options.tupleInspection.splitFilter"]')
      .selectOption('holdout')
    await expect(page.getByText(/AAPL-20260424-002/i).first()).toBeVisible()
    await page
      .locator('[data-qa-control="options.tupleInspection.splitFilter"]')
      .selectOption('all')
    await action('options.tupleInspection.exportCsv').click()
    await expect(page.getByText(/Saved and revealed 2 inspection rows/i).first()).toBeVisible()

    await page.locator('[aria-label="Select inspection event AAPL-20260424-001"]').click()
    await expect(action('options.eventPath.load')).toBeEnabled()
    await action('options.eventPath.load').click()
    await expect(page.getByText(/Loaded 4 path points for event AAPL-20260424-001/i).first()).toBeVisible()
    await expect(page.getByText(/Event path replay/i).first()).toBeVisible()
    await expect(page.getByText(/Option O\/H\/L\/C/i).first()).toBeVisible()
    await action('options.eventPath.exportCsv').click()
    await expect(page.getByText(/Saved and revealed 4 replay points/i).first()).toBeVisible()

    await page
      .locator('[aria-label="Options expression follow-up"]')
      .screenshot({ path: path.join(evidenceDir, 'tuple-event-path.png') })

    const planSelect = page.locator('[data-qa-control="options.validation.planMode"]')
    await planSelect.selectOption('anchored_walk_forward')
    await expect(page.locator('[data-qa-control="options.validation.trainSessions"]')).toBeVisible()
    await expect(page.locator('[data-qa-control="options.validation.validationSessions"]')).toBeVisible()
    await expect(page.locator('[data-qa-control="options.validation.stepSessions"]')).toBeVisible()
    await planSelect.selectOption('rolling_session_walk_forward')
    await expect(page.locator('[data-qa-control="options.validation.stepSessions"]')).toBeVisible()
    await planSelect.selectOption('session_chronological_replay')
    await expect(page.locator('[data-qa-control="options.validation.startDate"]')).toHaveCount(0)
    await planSelect.selectOption('date_range_holdout')
    await page.locator('[data-qa-control="options.validation.startDate"]').fill('2030-04-24')
    await page.locator('[data-qa-control="options.validation.endDate"]').fill('2030-04-25')
    await page
      .locator('[data-qa-control="options.validation.datasetPolicy"]')
      .selectOption('allow_same_root_replacement')
    await page
      .locator('[data-qa-control="options.validation.datasetPolicy"]')
      .selectOption('require_exact_replay')

    await expect(action('options.validation.preview')).toBeEnabled()
    await action('options.validation.preview').click()
    await expect(page.getByText(/Validation preview selected 1 tuple/i).first()).toBeVisible()
    await expect(action('options.validation.run')).toBeEnabled()
    await action('options.validation.run').click()
    await expect(page.getByText(/Validation accepted: 1\/1 tuple accepted/i).first()).toBeVisible()
    await expect(page.getByText(/Accepted with supported holdout and stress lanes/i).first()).toBeVisible()
    await expect(action('options.validation.reveal')).toBeEnabled()
    await action('options.validation.reveal').click()

    await page
      .locator('[aria-label="Options expression follow-up"]')
      .screenshot({ path: path.join(evidenceDir, 'validation-result.png') })

    const invokes = await readTauriInvokeLog(page)
    const runInvoke = invokes.find((entry) => entry.command === 'run_options_expression')
    const runReq = objectValue(runInvoke?.args.req)
    expect(runReq.options_dataset_id).toBe('AAPL_OPTIONS_AGG_QA')
    expect(runReq.structure_family).toBe('long_single')
    expect(runReq.fill_model).toBe('aggregate_research')

    const validationInvoke = invokes
      .filter((entry) => entry.command === 'validate_options_expression_artifact')
      .at(-1)
    const validationReq = objectValue(validationInvoke?.args.req)
    expect(validationReq.artifact_path).toBe(artifactPath)
    expect(validationReq.options_dataset_id).toBe('AAPL_OPTIONS_AGG_QA')
    expect(objectValue(validationReq.validation_plan).mode).toBe('date_range_holdout')
    expect(objectValue(validationReq.tuple_selection).include_sealed_finalists).toBe(true)

    await expectNoHorizontalDocumentOverflow(page)
    await expectNoStuckBusyState(page)
  },
})
