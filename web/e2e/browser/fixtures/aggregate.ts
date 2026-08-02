import type { FixtureFamilyModule, MockTauriScenario } from "./types";
import { emptyAppScenario } from "./scenarios";
import {
  vcetTemporalBlockedRecoveryScenario,
  vcetTemporalEarlyFinalReportPoisonedScenario,
  vcetTemporalEarlySelectionReportPoisonedScenario,
  vcetTemporalFinalReportBlockedScenario,
  vcetTemporalFinalReportResumeScenario,
  vcetTemporalFinalShardResumeScenario,
  vcetTemporalLateFinalBlockedScenario,
  vcetTemporalLatePreFinalBlockedScenario,
  vcetTemporalLifecycleReadyScenario,
  vcetTemporalMutationRecoveryScenario,
  vcetTemporalSelectionReportBlockedScenario,
  vcetTemporalSelectionReportResumeScenario,
  vcetTemporalSelectionShardResumeScenario,
  vcetTemporalStaleRecoveryScenario,
} from "./vcetTemporal";

const metaReportPath = "C:/reports/vectorgrid_meta_latest.xlsx";
const metaJsonPath = "C:/runs/vectorgrid_meta_latest.json";

export const aggregateMetaSelectionResponse = {
  ts: "2026-05-04T12:10:00Z",
  meta_json_path: metaJsonPath,
  meta_json_bytes: 4096,
  report_xlsx_path: metaReportPath,
  report_xlsx_exists: true,
  result: {
    strategy_kind: "CompositeMo",
    timeframe: "1m",
    artifacts_used: 1,
    artifacts_considered: 1,
    candidates_considered: 4,
    runtime_ms: 12,
    holdout: {
      datasets: 1,
      skipped: 0,
    },
    holdout_coverage: {
      groups_evaluated: 1,
      groups_total: 1,
      coverage_mode: "fixture",
      selection_policy: "top",
    },
    groups: [
      {
        samples: 4,
        pass_rate: 0.75,
        meta_score: 1.23,
        final_score: 1.25,
        ml_confidence: 0.7,
        robust_score_p10: 0.8,
        robust_score_median: 1.1,
        max_dd_p90: -0.04,
        psr_median: 0.9,
        dsr_median: 0.8,
        params: {
          fast_len: 8,
          slow_len: 32,
          fast_ma_type: "sma",
          slow_ma_type: "sma",
        },
        holdout: {
          pass_rate: 1,
          score_p10: 0.7,
          score_median: 0.9,
          max_dd_p90: -0.05,
        },
        holdout_coverage: {
          evaluated: true,
          dataset_rows: 5000,
        },
      },
    ],
    warnings: [],
  },
};

const aggregateSavedMetaEntry = {
  ts: "2026-05-04T12:05:00Z",
  path: metaJsonPath,
  bytes: 4096,
  kind: "vectorgrid_meta_selection",
  strategy_kind: "CompositeMo",
  timeframe: "1m",
  groups: 1,
  artifacts_used: 1,
  holdout_datasets: 1,
  report_xlsx_path: metaReportPath,
  report_xlsx_exists: true,
  tags: ["qa"],
};

export const aggregateMetaReadyScenario: MockTauriScenario = {
  ...emptyAppScenario,
  name: "aggregate-meta-ready",
  family: "aggregate",
  description:
    "Saved runs are available and Meta Selection can run, open its report, reveal files, and replay a saved meta artifact.",
  commandResults: {
    ...emptyAppScenario.commandResults,
    list_run_artifacts: [
      {
        ts: "2026-05-04T12:00:00Z",
        path: "C:/runs/vectorgrid_run_a.json",
        bytes: 32768,
        kind: "vectorgrid_run",
        strategy_kind: "CompositeMo",
        csv_path: "C:/fixtures/aapl_1m.csv",
        timeframe: "1m",
        has_validation: true,
        replay_class: "exact",
        report_xlsx_path: "C:/reports/vectorgrid_run_a.xlsx",
        report_xlsx_exists: true,
        tags: ["qa"],
      },
    ],
    list_artifact_library: {
      meta: [aggregateSavedMetaEntry],
      stage5: [],
      policyEvals: [],
    },
    run_meta_selection: aggregateMetaSelectionResponse,
    ml_confidence_model_status: {
      exists: true,
      state: "ready",
      model_healthy: true,
      stale: false,
      retrain_recommended: false,
      available_meta_artifacts: 1,
      available_labeled_rows: 64,
    },
    load_meta_artifact_replay: aggregateMetaSelectionResponse,
    update_artifact_tags: null,
    open_path: null,
    reveal_path: null,
  },
  expectedInvokes: [
    "list_run_artifacts",
    "run_meta_selection",
    "open_path",
    "reveal_path",
  ],
};

export const aggregateFixtureFamily: FixtureFamilyModule = {
  family: "aggregate",
  description: "Aggregate, Meta, and Stage workflow browser QA scenarios.",
  scenarios: [
    aggregateMetaReadyScenario,
    vcetTemporalLifecycleReadyScenario,
    vcetTemporalStaleRecoveryScenario,
    vcetTemporalBlockedRecoveryScenario,
    vcetTemporalSelectionShardResumeScenario,
    vcetTemporalSelectionReportResumeScenario,
    vcetTemporalFinalShardResumeScenario,
    vcetTemporalFinalReportResumeScenario,
    vcetTemporalMutationRecoveryScenario,
    vcetTemporalLateFinalBlockedScenario,
    vcetTemporalLatePreFinalBlockedScenario,
    vcetTemporalSelectionReportBlockedScenario,
    vcetTemporalFinalReportBlockedScenario,
    vcetTemporalEarlySelectionReportPoisonedScenario,
    vcetTemporalEarlyFinalReportPoisonedScenario,
  ],
};
