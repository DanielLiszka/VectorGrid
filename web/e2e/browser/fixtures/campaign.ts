import type { FixtureFamilyModule, MockTauriScenario } from './types'
import { emptyAppScenario } from './scenarios'
import { dataLoadSuccessScenario } from './dataset'

const campaignSummaryPath = 'C:/runs/vectorgrid_campaign_summary.json'
const campaignDatasetA = 'C:/fixtures/aapl_1m.csv'
const campaignDatasetB = 'C:/fixtures/msft_1m.csv'

export const campaignRunResponse = {
  summary_json_path: campaignSummaryPath,
  summary: {
    ts: '2026-05-04T13:00:00Z',
    kind: 'vectorgrid_campaign_summary',
    state: 'completed_with_failures',
    resume_state: 'resumable',
    batch_tag: 'qa-campaign',
    tags: ['qa-campaign', 'holdout'],
    holdout_pct: 15,
    holdout_tag: 'holdout',
    selected_paths: [campaignDatasetA, campaignDatasetB],
    request: {
      strategy_kind: 'DoubleMa',
      selected_paths: [campaignDatasetA, campaignDatasetB],
      datasets: [
        {
          csv_path: campaignDatasetA,
          label: 'AAPL 1m',
          timeframe: '1m',
        },
        {
          csv_path: campaignDatasetB,
          label: 'MSFT 1m',
          timeframe: '1m',
        },
      ],
      tags: ['qa-campaign', 'holdout'],
      holdout_pct: 15,
      holdout_tag: 'holdout',
      skip_existing: true,
      execution_mode: 'full',
      auto_meta: false,
      auto_stage5: false,
      preload_policy: 'host_only',
    },
    counts: {
      total: 2,
      validated: 1,
      skipped: 0,
      failed: 1,
      cancelled: false,
    },
    items: [
      {
        csv_path: campaignDatasetA,
        label: 'AAPL 1m',
        status: 'validated',
        validation_report_path: 'C:/runs/aapl_validation.json',
      },
      {
        csv_path: campaignDatasetB,
        label: 'MSFT 1m',
        status: 'failed',
        error: 'fixture validation failure',
      },
    ],
  },
}

export const campaignBatchReadyScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: 'campaign-batch-ready',
  family: 'campaign',
  description:
    'Aggregate Batch can refresh datasets, select rows, run a delayed campaign, stop it, reveal the summary, retry failures, and resume incomplete work.',
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_price_data: dataLoadSuccessScenario.commandResults.load_price_data,
    list_run_artifacts: [
      {
        ts: '2026-05-04T12:45:00Z',
        path: 'C:/runs/vectorgrid_campaign_seed_run.json',
        bytes: 32768,
        kind: 'vectorgrid_run',
        strategy_kind: 'DoubleMa',
        csv_path: campaignDatasetA,
        timeframe: '1m',
        has_validation: true,
        replay_class: 'exact',
        report_xlsx_path: 'C:/reports/vectorgrid_campaign_seed_run.xlsx',
        report_xlsx_exists: true,
        tags: ['qa-campaign'],
      },
    ],
    polygon_get_cache_info: {
      datasets: [
        {
          path: campaignDatasetA,
          label: 'AAPL 1m',
          market: 'stocks',
          timeframe: '1m',
          adjusted: true,
          min_ts: 1_760_000_000_000,
          max_ts: 1_760_018_000_000,
          row_count: 5000,
          last_updated_utc_ms: 1_760_020_000_000,
          source_size_bytes: 240000,
          source_mtime_utc_ms: 1_760_020_000_000,
          source_content_fingerprint: 'fixture-aapl-1m',
        },
      ],
    },
    list_repo_data_csvs: {
      dir: 'C:/fixtures',
      truncated: false,
      csv_entries: [
        {
          path: campaignDatasetB,
          label: 'MSFT 1m',
          market: 'stocks',
          timeframe: '1m',
          adjusted: true,
          min_ts: 1_760_000_000_000,
          max_ts: 1_760_018_000_000,
          row_count: 4800,
          last_updated_utc_ms: 1_760_021_000_000,
          source_size_bytes: 220000,
          source_mtime_utc_ms: 1_760_021_000_000,
          source_content_fingerprint: 'fixture-msft-1m',
        },
      ],
    },
    run_research_campaign: campaignRunResponse,
    cancel_current_run: null,
    reveal_path: null,
  },
  commandDelays: {
    run_research_campaign: 2500,
  },
  commandEvents: {
    run_research_campaign: [
      {
        event: 'research_campaign_progress',
        delayMs: 100,
        payload: {
          processed: 0,
          total: 2,
          phase: 'starting',
          summary_json_path: campaignSummaryPath,
        },
      },
      {
        event: 'research_campaign_progress',
        delayMs: 420,
        payload: {
          processed: 1,
          total: 2,
          phase: 'validating',
          summary_json_path: campaignSummaryPath,
        },
      },
    ],
  },
  expectedInvokes: [
    'load_price_data',
    'list_run_artifacts',
    'polygon_get_cache_info',
    'list_repo_data_csvs',
    'run_research_campaign',
    'cancel_current_run',
    'reveal_path',
  ],
}

export const campaignFixtureFamily: FixtureFamilyModule = {
  family: 'campaign',
  description: 'Research campaign unsupported, failure, and resumable-state browser QA scenarios.',
  scenarios: [campaignBatchReadyScenario],
}
