import type { MockTauriScenario } from "./types";
import { emptyAppScenario } from "./scenarios";

export const vcetTemporalSourceSummaryPath =
  "C:/campaigns/vcet-candidate-universe.json";
export const vcetTemporalPreparationPath = "C:/temporal/vcet-preparation.json";
export const vcetTemporalSelectionReductionPath =
  "C:/temporal/vcet-selection-reduction.json";
export const vcetTemporalSelectionReportPath =
  "C:/reports/vectorgrid_vcet_temporal_selection_qa.xlsx";
export const vcetTemporalFinalReductionPath =
  "C:/temporal/vcet-final-reduction.json";
export const vcetTemporalFinalReportPath =
  "C:/reports/vectorgrid_vcet_temporal_final_qa.xlsx";

const hash = (digit: string): string => `sha256:${digit.repeat(64)}`;

export const vcetTemporalDefaultPlanDraft = {
  startUtc: "2018-01-01T00:00",
  endUtc: "2025-12-31T23:59",
  foldMode: "anchored",
  trainBars: "2048",
  validationBars: "512",
  finalTestBars: "512",
  stepBars: "544",
  purgeBars: "32",
  embargoBars: "32",
  warmupBars: "256",
  positionBoundaryPolicy: "reset_flat_discard_open",
  minFolds: "3",
  minTrainBars: "2048",
  minValidationBars: "512",
  minFinalTestBars: "512",
  minTrainTradesPerFold: "10",
  minValidationTradesPerFold: "5",
  minFinalTestTrades: "5",
  reportLabel: "VCET temporal research",
} as const;

export const vcetTemporalSourceSummary = {
  kind: "vectorgrid_campaign_summary",
  version: 2,
  state: "completed",
  strategy_kind: "Vcet",
  counts: { cancelled: false },
  request: {
    strategy_kind: "Vcet",
    execution_mode: "evaluate_candidates",
    datasets: [
      { csv_path: "C:/fixtures/aapl_1m.csv" },
      { csv_path: "C:/fixtures/msft_1m.csv" },
    ],
  },
  manifest: {
    strategy_kind: "Vcet",
    execution_mode: "evaluate_candidates",
    manifest_hash: hash("1"),
    candidate_manifest_hash: hash("2"),
  },
  fixed_candidate_evaluation: {
    schema_version: 4,
    candidate_manifest_hash: hash("2"),
    admitted_candidate_count: 2,
    admitted_candidate_ids: ["vcet-candidate-a", "vcet-candidate-b"],
  },
  items: [
    {
      item_id: "aapl-selection",
      is_holdout: false,
      status: "ok",
      artifact_path: "C:/runs/aapl-vcet.json",
      artifact_sha256: hash("3"),
    },
    {
      item_id: "msft-final",
      is_holdout: true,
      status: "ok",
      artifact_path: "C:/runs/msft-vcet.json",
      artifact_sha256: hash("4"),
    },
  ],
};

export const vcetTemporalPreparedStatus = {
  schema_version: 1,
  campaign_id: "vcet-temporal-qa-1",
  state: "prepared",
  preparation_artifact_path: vcetTemporalPreparationPath,
  preparation_artifact_sha256: hash("a"),
  source_campaign_summary_path: vcetTemporalSourceSummaryPath,
  source_campaign_summary_sha256: hash("b"),
  universe_sha256: hash("c"),
  candidate_count: 2,
  dataset_count: 2,
  selection_dataset_count: 1,
  final_test_dataset_count: 1,
  selection_shard_count: 1,
  completed_selection_shard_count: 0,
  final_test_shard_count: 1,
  completed_final_test_shard_count: 0,
  winner_candidate_id: null,
  winner_lock_sha256: null,
  winner_runner_up_margin: null,
  pre_final_reduction_artifact_path: null,
  pre_final_reduction_artifact_sha256: null,
  final_reduction_artifact_path: null,
  final_reduction_artifact_sha256: null,
  selection_report_xlsx_path: null,
  final_report_xlsx_path: null,
  execution_blocked: false,
  next_required_action: "run_selection",
  limitations: ["product_v1_is_bounded_to_64_datasets"],
};

export const vcetTemporalSelectionCompleteStatus = {
  ...vcetTemporalPreparedStatus,
  state: "selection_complete",
  completed_selection_shard_count: 1,
  winner_candidate_id: "vcet-candidate-a",
  winner_lock_sha256: hash("d"),
  winner_runner_up_margin: 0.1875,
  pre_final_reduction_artifact_path: vcetTemporalSelectionReductionPath,
  pre_final_reduction_artifact_sha256: hash("e"),
  selection_report_xlsx_path: vcetTemporalSelectionReportPath,
  next_required_action: "confirm_and_open_final_test",
};

export const vcetTemporalFinalCompleteStatus = {
  ...vcetTemporalSelectionCompleteStatus,
  state: "final_test_complete",
  completed_final_test_shard_count: 1,
  final_reduction_artifact_path: vcetTemporalFinalReductionPath,
  final_reduction_artifact_sha256: hash("f"),
  final_report_xlsx_path: vcetTemporalFinalReportPath,
  next_required_action: "none",
};

export const vcetTemporalPreFinalBlockedStatus = {
  ...vcetTemporalPreparedStatus,
  completed_selection_shard_count: 1,
  execution_blocked: true,
  blocked_stage: "pre_final_reduction",
  blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
  next_required_action: "start_new_campaign",
};

export const vcetTemporalSelectionShardResumeStatus = {
  ...vcetTemporalPreparedStatus,
  dataset_count: 3,
  selection_dataset_count: 2,
  final_test_dataset_count: 1,
  selection_shard_count: 2,
  completed_selection_shard_count: 1,
  winner_candidate_id: "vcet-candidate-a",
  winner_lock_sha256: hash("d"),
  winner_runner_up_margin: 0.1875,
  pre_final_reduction_artifact_path: vcetTemporalSelectionReductionPath,
  pre_final_reduction_artifact_sha256: hash("e"),
  next_required_action: "resume_selection",
};

export const vcetTemporalSelectionReportResumeStatus = {
  ...vcetTemporalSelectionShardResumeStatus,
  completed_selection_shard_count: 2,
  next_required_action: "resume_selection_report",
};

export const vcetTemporalFinalShardResumeStatus = {
  ...vcetTemporalSelectionCompleteStatus,
  dataset_count: 3,
  selection_dataset_count: 1,
  final_test_dataset_count: 2,
  selection_shard_count: 1,
  completed_selection_shard_count: 1,
  final_test_shard_count: 2,
  completed_final_test_shard_count: 1,
  next_required_action: "confirm_and_resume_final_test",
};

export const vcetTemporalFinalReportResumeStatus = {
  ...vcetTemporalFinalShardResumeStatus,
  completed_final_test_shard_count: 2,
  final_reduction_artifact_path: vcetTemporalFinalReductionPath,
  final_reduction_artifact_sha256: hash("f"),
  next_required_action: "confirm_and_resume_final_report",
};

export const vcetTemporalLateFinalBlockedStatus = {
  ...vcetTemporalFinalCompleteStatus,
  execution_blocked: true,
  blocked_stage: "final_test_shard",
  blocked_shard_ordinal: 0,
  blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
  next_required_action: "start_new_campaign",
};

export const vcetTemporalLatePreFinalBlockedStatus = {
  ...vcetTemporalFinalCompleteStatus,
  execution_blocked: true,
  blocked_stage: "pre_final_reduction",
  blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
  next_required_action: "start_new_campaign",
};

export const vcetTemporalSelectionReportBlockedStatus = {
  ...vcetTemporalFinalCompleteStatus,
  selection_report_xlsx_path: null,
  execution_blocked: true,
  blocked_stage: "selection_report",
  blocked_reason_code: "report_receipt_invalid_or_foreign",
  next_required_action: "start_new_campaign",
};

export const vcetTemporalFinalReportBlockedStatus = {
  ...vcetTemporalFinalCompleteStatus,
  state: "selection_complete",
  final_report_xlsx_path: null,
  execution_blocked: true,
  blocked_stage: "final_report",
  blocked_reason_code: "report_xlsx_not_recoverable",
  next_required_action: "start_new_campaign",
};

export const vcetTemporalEarlySelectionReportPoisonedStatus = {
  ...vcetTemporalPreparedStatus,
  execution_blocked: true,
  blocked_stage: "selection_report",
  blocked_reason_code: "report_receipt_invalid_or_foreign",
  next_required_action: "start_new_campaign",
};

export const vcetTemporalEarlyFinalReportPoisonedStatus = {
  ...vcetTemporalSelectionCompleteStatus,
  execution_blocked: true,
  blocked_stage: "final_report",
  blocked_reason_code: "report_receipt_invalid_or_foreign",
  next_required_action: "start_new_campaign",
};

const temporalSourceStorage = {
  ...emptyAppScenario.localStorage,
  vectorgrid_last_batch_summary_path_v1: vcetTemporalSourceSummaryPath,
};

export const vcetTemporalLifecycleReadyScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: "vcet-temporal-lifecycle-ready",
  family: "aggregate",
  description:
    "A disk-reloaded exact VCET candidate universe drives the mocked temporal preparation, selection, explicit final disclosure, and evidence actions.",
  localStorage: temporalSourceStorage,
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_artifact: vcetTemporalSourceSummary,
    prepare_vcet_temporal_campaign: vcetTemporalPreparedStatus,
    get_vcet_temporal_campaign_status: vcetTemporalPreparedStatus,
    run_vcet_temporal_campaign_selection: vcetTemporalSelectionCompleteStatus,
    open_vcet_temporal_campaign_final_test: vcetTemporalFinalCompleteStatus,
    reveal_path: null,
    open_path: null,
  },
  expectedInvokes: [
    "load_artifact",
    "prepare_vcet_temporal_campaign",
    "get_vcet_temporal_campaign_status",
    "run_vcet_temporal_campaign_selection",
    "open_vcet_temporal_campaign_final_test",
    "reveal_path",
    "open_path",
  ],
};

const persistedTemporalSnapshot = {
  schema_version: 1,
  plan_draft: vcetTemporalDefaultPlanDraft,
  current_preparation: {
    source_campaign_summary_path: vcetTemporalSourceSummaryPath,
    preparation_artifact_path: vcetTemporalPreparationPath,
    preparation_artifact_sha256: hash("a"),
    prepared_plan_draft_snapshot: vcetTemporalDefaultPlanDraft,
  },
};

export const vcetTemporalStaleRecoveryScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: "vcet-temporal-stale-recovery",
  family: "aggregate",
  description:
    "A persisted temporal preparation whose native status refresh fails can forget only its local pointer after explicit confirmation.",
  localStorage: {
    ...temporalSourceStorage,
    vectorgrid_vcet_temporal_campaign_v1: JSON.stringify(
      persistedTemporalSnapshot,
    ),
  },
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_artifact: vcetTemporalSourceSummary,
    get_vcet_temporal_campaign_status: {
      __mockFailure:
        "The retained temporal preparation fixture is unavailable.",
    },
  },
  expectedInvokes: ["load_artifact", "get_vcet_temporal_campaign_status"],
  forbiddenInvokes: [
    "prepare_vcet_temporal_campaign",
    "run_vcet_temporal_campaign_selection",
    "open_vcet_temporal_campaign_final_test",
    "reveal_path",
    "open_path",
  ],
};

export const vcetTemporalBlockedRecoveryScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: "vcet-temporal-blocked-recovery",
  family: "aggregate",
  description:
    "A verified temporal preparation with terminal pre-final durable evidence cannot retry or forget the identical deterministic identity.",
  localStorage: {
    ...temporalSourceStorage,
    vectorgrid_vcet_temporal_campaign_v1: JSON.stringify(
      persistedTemporalSnapshot,
    ),
  },
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_artifact: vcetTemporalSourceSummary,
    get_vcet_temporal_campaign_status: vcetTemporalPreFinalBlockedStatus,
  },
  expectedInvokes: ["load_artifact", "get_vcet_temporal_campaign_status"],
  forbiddenInvokes: [
    "prepare_vcet_temporal_campaign",
    "run_vcet_temporal_campaign_selection",
    "open_vcet_temporal_campaign_final_test",
    "reveal_path",
    "open_path",
  ],
};

function resumeScenario(
  name: string,
  description: string,
  currentStatus: unknown,
  command:
    | "run_vcet_temporal_campaign_selection"
    | "open_vcet_temporal_campaign_final_test",
  completedStatus: unknown,
): MockTauriScenario {
  return {
    ...emptyAppScenario,
    name,
    family: "aggregate",
    description,
    localStorage: {
      ...temporalSourceStorage,
      vectorgrid_vcet_temporal_campaign_v1: JSON.stringify(
        persistedTemporalSnapshot,
      ),
    },
    commandResults: {
      ...emptyAppScenario.commandResults,
      load_artifact: vcetTemporalSourceSummary,
      get_vcet_temporal_campaign_status: currentStatus,
      [command]: completedStatus,
    },
    expectedInvokes: [
      "load_artifact",
      "get_vcet_temporal_campaign_status",
      command,
    ],
  };
}

export const vcetTemporalSelectionShardResumeScenario = resumeScenario(
  "vcet-temporal-selection-shard-resume",
  "A retained preparation resumes remaining immutable selection shards.",
  vcetTemporalSelectionShardResumeStatus,
  "run_vcet_temporal_campaign_selection",
  vcetTemporalSelectionCompleteStatus,
);

export const vcetTemporalSelectionReportResumeScenario = resumeScenario(
  "vcet-temporal-selection-report-resume",
  "A retained pre-final reduction resumes only its missing selection workbook receipt.",
  vcetTemporalSelectionReportResumeStatus,
  "run_vcet_temporal_campaign_selection",
  vcetTemporalSelectionCompleteStatus,
);

export const vcetTemporalFinalShardResumeScenario = resumeScenario(
  "vcet-temporal-final-shard-resume",
  "A retained winner lock explicitly resumes remaining final-test shards.",
  vcetTemporalFinalShardResumeStatus,
  "open_vcet_temporal_campaign_final_test",
  vcetTemporalFinalCompleteStatus,
);

export const vcetTemporalFinalReportResumeScenario = resumeScenario(
  "vcet-temporal-final-report-resume",
  "A retained final reduction explicitly resumes only its missing final workbook receipt.",
  vcetTemporalFinalReportResumeStatus,
  "open_vcet_temporal_campaign_final_test",
  vcetTemporalFinalCompleteStatus,
);

export const vcetTemporalLateFinalBlockedScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: "vcet-temporal-late-final-blocked",
  family: "aggregate",
  description:
    "A late artifact-derived final state remains non-resumable when one durable final-shard record is terminally blocked.",
  localStorage: {
    ...temporalSourceStorage,
    vectorgrid_vcet_temporal_campaign_v1: JSON.stringify(
      persistedTemporalSnapshot,
    ),
  },
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_artifact: vcetTemporalSourceSummary,
    get_vcet_temporal_campaign_status: vcetTemporalLateFinalBlockedStatus,
  },
  expectedInvokes: ["load_artifact", "get_vcet_temporal_campaign_status"],
  forbiddenInvokes: [
    "prepare_vcet_temporal_campaign",
    "run_vcet_temporal_campaign_selection",
    "open_vcet_temporal_campaign_final_test",
  ],
};

export const vcetTemporalLatePreFinalBlockedScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: "vcet-temporal-late-pre-final-blocked",
  family: "aggregate",
  description:
    "Retained final evidence stays visibly complete when a late pre-final reduction record is terminally blocked.",
  localStorage: {
    ...temporalSourceStorage,
    vectorgrid_vcet_temporal_campaign_v1: JSON.stringify(
      persistedTemporalSnapshot,
    ),
  },
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_artifact: vcetTemporalSourceSummary,
    get_vcet_temporal_campaign_status: vcetTemporalLatePreFinalBlockedStatus,
  },
  expectedInvokes: ["load_artifact", "get_vcet_temporal_campaign_status"],
  forbiddenInvokes: [
    "prepare_vcet_temporal_campaign",
    "run_vcet_temporal_campaign_selection",
    "open_vcet_temporal_campaign_final_test",
  ],
};

export const vcetTemporalSelectionReportBlockedScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: "vcet-temporal-selection-report-blocked",
  family: "aggregate",
  description:
    "A foreign selection-report receipt blocks replay without erasing the locked winner or retained later final evidence.",
  localStorage: {
    ...temporalSourceStorage,
    vectorgrid_vcet_temporal_campaign_v1: JSON.stringify(
      persistedTemporalSnapshot,
    ),
  },
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_artifact: vcetTemporalSourceSummary,
    get_vcet_temporal_campaign_status: vcetTemporalSelectionReportBlockedStatus,
  },
  expectedInvokes: ["load_artifact", "get_vcet_temporal_campaign_status"],
  forbiddenInvokes: [
    "prepare_vcet_temporal_campaign",
    "run_vcet_temporal_campaign_selection",
    "open_vcet_temporal_campaign_final_test",
  ],
};

export const vcetTemporalFinalReportBlockedScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: "vcet-temporal-final-report-blocked",
  family: "aggregate",
  description:
    "An unrecoverable final workbook blocks replay while retaining the completed final scientific reduction.",
  localStorage: {
    ...temporalSourceStorage,
    vectorgrid_vcet_temporal_campaign_v1: JSON.stringify(
      persistedTemporalSnapshot,
    ),
  },
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_artifact: vcetTemporalSourceSummary,
    get_vcet_temporal_campaign_status: vcetTemporalFinalReportBlockedStatus,
  },
  expectedInvokes: ["load_artifact", "get_vcet_temporal_campaign_status"],
  forbiddenInvokes: [
    "prepare_vcet_temporal_campaign",
    "run_vcet_temporal_campaign_selection",
    "open_vcet_temporal_campaign_final_test",
  ],
};

export const vcetTemporalEarlySelectionReportPoisonedScenario: MockTauriScenario =
  {
    ...emptyAppScenario,
    name: "vcet-temporal-early-selection-report-poisoned",
    family: "aggregate",
    description:
      "A foreign selection-report receipt terminally blocks the preparation before any locked winner or scientific selection reduction exists.",
    localStorage: {
      ...temporalSourceStorage,
      vectorgrid_vcet_temporal_campaign_v1: JSON.stringify(
        persistedTemporalSnapshot,
      ),
    },
    commandResults: {
      ...emptyAppScenario.commandResults,
      load_artifact: vcetTemporalSourceSummary,
      get_vcet_temporal_campaign_status:
        vcetTemporalEarlySelectionReportPoisonedStatus,
    },
    expectedInvokes: ["load_artifact", "get_vcet_temporal_campaign_status"],
    forbiddenInvokes: [
      "prepare_vcet_temporal_campaign",
      "run_vcet_temporal_campaign_selection",
      "open_vcet_temporal_campaign_final_test",
      "reveal_path",
      "open_path",
    ],
  };

export const vcetTemporalEarlyFinalReportPoisonedScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: "vcet-temporal-early-final-report-poisoned",
  family: "aggregate",
  description:
    "A foreign final-report receipt terminally blocks the campaign after selection but before any final scientific reduction exists.",
  localStorage: {
    ...temporalSourceStorage,
    vectorgrid_vcet_temporal_campaign_v1: JSON.stringify(
      persistedTemporalSnapshot,
    ),
  },
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_artifact: vcetTemporalSourceSummary,
    get_vcet_temporal_campaign_status:
      vcetTemporalEarlyFinalReportPoisonedStatus,
  },
  expectedInvokes: ["load_artifact", "get_vcet_temporal_campaign_status"],
  forbiddenInvokes: [
    "prepare_vcet_temporal_campaign",
    "run_vcet_temporal_campaign_selection",
    "open_vcet_temporal_campaign_final_test",
    "reveal_path",
    "open_path",
  ],
};

export const vcetTemporalMutationRecoveryScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: "vcet-temporal-mutation-recovery",
  family: "aggregate",
  description:
    "Mocked prepare, selection, and final invocation failures remain retryable without losing the active temporal identity.",
  localStorage: temporalSourceStorage,
  commandResults: {
    ...emptyAppScenario.commandResults,
    load_artifact: vcetTemporalSourceSummary,
    prepare_vcet_temporal_campaign: {
      __mockSequence: [
        { __mockFailure: "Mocked temporal preparation failure." },
        vcetTemporalPreparedStatus,
      ],
    },
    run_vcet_temporal_campaign_selection: {
      __mockSequence: [
        { __mockFailure: "Mocked temporal selection failure." },
        vcetTemporalSelectionCompleteStatus,
      ],
    },
    get_vcet_temporal_campaign_status: {
      __mockSequence: [
        vcetTemporalPreparedStatus,
        vcetTemporalSelectionCompleteStatus,
      ],
    },
    open_vcet_temporal_campaign_final_test: {
      __mockSequence: [
        { __mockFailure: "Mocked temporal final-test failure." },
        vcetTemporalFinalCompleteStatus,
      ],
    },
  },
  expectedInvokes: [
    "load_artifact",
    "prepare_vcet_temporal_campaign",
    "get_vcet_temporal_campaign_status",
    "run_vcet_temporal_campaign_selection",
    "open_vcet_temporal_campaign_final_test",
  ],
};
