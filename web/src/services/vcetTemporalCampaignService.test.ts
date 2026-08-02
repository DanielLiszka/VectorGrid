import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand } from "../tauri/invoke";
import {
  buildVcetTemporalValidationPlan,
  createDefaultVcetTemporalPlanDraft,
  getVcetTemporalCampaignStatus,
  normalizeVcetTemporalCampaignStatus,
  openVcetTemporalCampaignFinalTest,
  openVcetTemporalCampaignWorkbook,
  prepareVcetTemporalCampaign,
  revealVcetTemporalCampaignPath,
  runVcetTemporalCampaignSelection,
  vcetTemporalNativePathKey,
} from "./vcetTemporalCampaignService";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(),
}));

const invokeCommandMock = vi.mocked(invokeCommand);
const hash = (digit: string) => `sha256:${digit.repeat(64)}`;

function preparedStatus(overrides: Record<string, unknown> = {}) {
  return {
    schema_version: 1,
    campaign_id: "vcet-temporal-1",
    state: "prepared",
    preparation_artifact_path: "C:\\campaign\\preparation.json",
    preparation_artifact_sha256: hash("a"),
    source_campaign_summary_path: "C:\\picked\\summary.json",
    source_campaign_summary_sha256: hash("b"),
    universe_sha256: hash("c"),
    candidate_count: 12,
    dataset_count: 3,
    selection_dataset_count: 2,
    final_test_dataset_count: 1,
    selection_shard_count: 2,
    completed_selection_shard_count: 0,
    final_test_shard_count: 1,
    completed_final_test_shard_count: 0,
    execution_blocked: false,
    next_required_action: "run_selection",
    limitations: [
      "product_v1_is_bounded_to_64_datasets",
      "one_dataset_is_loaded_per_execution_shard",
    ],
    ...overrides,
  };
}

describe("VCET temporal campaign service", () => {
  beforeEach(() => {
    invokeCommandMock.mockReset();
  });

  it("normalizes the reviewed plan draft into the exact snake-case prepare request", async () => {
    const draft = createDefaultVcetTemporalPlanDraft();
    draft.startUtc = "2024-01-02T03:04";
    draft.endUtc = "2025-02-03T04:05";
    draft.reportLabel = "  Reviewed temporal run  ";
    invokeCommandMock.mockResolvedValue(
      preparedStatus({
        source_campaign_summary_path: "\\\\?\\C:\\picked\\summary.json",
      }) as never,
    );

    const status = await prepareVcetTemporalCampaign({
      sourceCampaignSummaryPath: "C:\\picked\\summary.json",
      planDraft: draft,
    });

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "prepare_vcet_temporal_campaign",
      {
        req: {
          source_campaign_summary_path: "C:\\picked\\summary.json",
          report_label: "Reviewed temporal run",
          plan: expect.objectContaining({
            schema_version: 1,
            requested_bounds: {
              utc: {
                start_utc_ms_inclusive: Date.UTC(2024, 0, 2, 3, 4),
                end_utc_ms_inclusive: Date.UTC(2025, 1, 3, 4, 5),
              },
            },
            fold_mode: "anchored",
            train_bars: 2048,
            validation_bars: 512,
            final_test_bars: 512,
            step_bars: 544,
            purge_bars: 32,
            embargo_bars: 32,
            warmup_bars: 256,
            position_boundary_policy: "reset_flat_discard_open",
            minimum_support: {
              min_folds: 3,
              min_train_bars: 2048,
              min_validation_bars: 512,
              min_final_test_bars: 512,
              min_train_trades_per_fold: 10,
              min_validation_trades_per_fold: 5,
              min_final_test_trades: 5,
            },
          }),
        },
      },
    );
    expect(status.sourceCampaignSummaryPath).toBe(
      "\\\\?\\C:\\picked\\summary.json",
    );
    expect(status.preparationArtifactSha256).toBe(hash("a"));
  });

  it("validates integer and interval bounds before invoking", async () => {
    const draft = createDefaultVcetTemporalPlanDraft();
    draft.stepBars = "512";

    expect(() => buildVcetTemporalValidationPlan(draft)).toThrow(
      "Step bars must be at least validation bars plus embargo bars.",
    );
    await expect(
      prepareVcetTemporalCampaign({
        sourceCampaignSummaryPath: "C:\\picked\\summary.json",
        planDraft: draft,
      }),
    ).rejects.toThrow("Step bars");
    expect(invokeCommandMock).not.toHaveBeenCalled();
  });

  it("matches the per-dataset product cap and UTF-8 report-label contract", () => {
    const oversizedLayout = createDefaultVcetTemporalPlanDraft();
    oversizedLayout.trainBars = "2000000";
    oversizedLayout.minTrainBars = "2000000";
    expect(() => buildVcetTemporalValidationPlan(oversizedLayout)).toThrow(
      "2,000,000-bar per-dataset product limit",
    );

    const oversizedLabel = createDefaultVcetTemporalPlanDraft();
    oversizedLabel.reportLabel = "🧪".repeat(40);
    expect(() => buildVcetTemporalValidationPlan(oversizedLabel)).toThrow(
      "128 UTF-8 bytes",
    );

    const controlLabel = createDefaultVcetTemporalPlanDraft();
    controlLabel.reportLabel = `research\u0085label`;
    expect(() => buildVcetTemporalValidationPlan(controlLabel)).toThrow(
      "control characters",
    );
  });

  it("canonicalizes equivalent Windows path spellings for native identity checks", () => {
    expect(
      vcetTemporalNativePathKey("C:\\Runs\\.\\VCET\\..\\summary.json"),
    ).toBe("c:/runs/summary.json");
    expect(vcetTemporalNativePathKey("\\\\?\\c:\\RUNS\\summary.json")).toBe(
      "c:/runs/summary.json",
    );
    expect(
      vcetTemporalNativePathKey("\\\\?\\UNC\\Server\\Share\\summary.json"),
    ).toBe("//server/share/summary.json");
  });

  it("accepts a canonical preparation response for a pinned alias with the same hash", () => {
    const normalized = normalizeVcetTemporalCampaignStatus(preparedStatus(), {
      preparationArtifactPath: "C:\\campaign-alias\\preparation.json",
      preparationArtifactSha256: hash("a"),
    });

    expect(normalized.preparationArtifactPath).toBe(
      "C:\\campaign\\preparation.json",
    );
    expect(normalized.preparationArtifactSha256).toBe(hash("a"));
  });

  it("rejects a canonical preparation response when its pinned hash differs", () => {
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({ preparation_artifact_sha256: hash("f") }),
        {
          preparationArtifactPath: "C:\\campaign-alias\\preparation.json",
          preparationArtifactSha256: hash("a"),
        },
      ),
    ).toThrow("changed the preparation hash");
  });

  it("retains path matching as the fallback when no preparation hash is pinned", () => {
    expect(() =>
      normalizeVcetTemporalCampaignStatus(preparedStatus(), {
        preparationArtifactPath: "C:\\campaign-alias\\preparation.json",
      }),
    ).toThrow("changed the preparation path");
  });

  it("wraps selection, resumable status, reveal, and workbook commands with compact identities", async () => {
    invokeCommandMock
      .mockResolvedValueOnce(
        preparedStatus({
          state: "selection_complete",
          completed_selection_shard_count: 2,
          winner_candidate_id: "candidate-7",
          winner_lock_sha256: hash("d"),
          pre_final_reduction_artifact_path:
            "C:\\campaign\\pre_final_reduction.json",
          pre_final_reduction_artifact_sha256: hash("e"),
          selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
          next_required_action: "confirm_and_open_final_test",
        }) as never,
      )
      .mockResolvedValueOnce(preparedStatus() as never)
      .mockResolvedValueOnce(undefined as never)
      .mockResolvedValueOnce(undefined as never);

    await runVcetTemporalCampaignSelection({
      preparationArtifactPath: "C:\\campaign\\preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
    });
    await getVcetTemporalCampaignStatus({
      preparationArtifactPath: "C:\\campaign\\preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
    });
    await revealVcetTemporalCampaignPath(
      "C:\\campaign\\pre_final_reduction.json",
    );
    await openVcetTemporalCampaignWorkbook("C:\\campaign\\selection.xlsx");

    expect(invokeCommandMock.mock.calls).toEqual([
      [
        "run_vcet_temporal_campaign_selection",
        {
          req: {
            preparation_artifact_path: "C:\\campaign\\preparation.json",
            expected_preparation_artifact_sha256: hash("a"),
          },
        },
      ],
      [
        "get_vcet_temporal_campaign_status",
        {
          req: {
            preparation_artifact_path: "C:\\campaign\\preparation.json",
            expected_preparation_artifact_sha256: hash("a"),
          },
        },
      ],
      ["reveal_path", { path: "C:\\campaign\\pre_final_reduction.json" }],
      ["open_path", { path: "C:\\campaign\\selection.xlsx" }],
    ]);
  });

  it("rejects malformed or internally inconsistent compact status", () => {
    expect(() => normalizeVcetTemporalCampaignStatus(null)).toThrow(
      "unsupported schema",
    );
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          state: "selection_complete",
          completed_selection_shard_count: 2,
        }),
      ),
    ).toThrow("completed selection evidence");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(preparedStatus({ dataset_count: 4 })),
    ).toThrow("inconsistent counts");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({ schema_version: "1" }),
      ),
    ).toThrow("unsupported schema");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({ candidate_count: "12" }),
      ),
    ).toThrow("candidate count");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          completed_selection_shard_count: 1,
          winner_candidate_id: "candidate-7",
          winner_lock_sha256: hash("d"),
          pre_final_reduction_artifact_path:
            "C:\\campaign\\pre_final_reduction.json",
          pre_final_reduction_artifact_sha256: hash("e"),
          next_required_action: "resume_selection_report",
        }),
      ),
    ).toThrow("next required action");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({ execution_blocked: "false" }),
      ),
    ).toThrow("execution-blocked state");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          blocked_stage: "pre_final_reduction",
          blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
        }),
      ),
    ).toThrow("while execution is resumable");
  });

  it("strictly normalizes terminal durable-stage blocking and rejects malformed blocked evidence", () => {
    const preFinalBlocked = normalizeVcetTemporalCampaignStatus(
      preparedStatus({
        completed_selection_shard_count: 2,
        execution_blocked: true,
        blocked_stage: "pre_final_reduction",
        blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
        next_required_action: "start_new_campaign",
      }),
    );
    expect(preFinalBlocked).toEqual(
      expect.objectContaining({
        executionBlocked: true,
        blockedStage: "pre_final_reduction",
        blockedShardOrdinal: null,
        blockedReasonCode: "durable_stage_failed_interrupted_or_invalid",
        nextRequiredAction: "start_new_campaign",
      }),
    );

    const latePreFinalBlocked = normalizeVcetTemporalCampaignStatus(
      preparedStatus({
        state: "selection_complete",
        completed_selection_shard_count: 1,
        winner_candidate_id: "candidate-7",
        winner_lock_sha256: hash("d"),
        pre_final_reduction_artifact_path:
          "C:\\campaign\\pre_final_reduction.json",
        pre_final_reduction_artifact_sha256: hash("e"),
        selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
        execution_blocked: true,
        blocked_stage: "pre_final_reduction",
        blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
        next_required_action: "start_new_campaign",
      }),
    );
    expect(latePreFinalBlocked.state).toBe("selection_complete");
    expect(latePreFinalBlocked.completedSelectionShardCount).toBe(1);

    const finalBlocked = normalizeVcetTemporalCampaignStatus(
      preparedStatus({
        state: "selection_complete",
        completed_selection_shard_count: 2,
        winner_candidate_id: "candidate-7",
        winner_lock_sha256: hash("d"),
        pre_final_reduction_artifact_path:
          "C:\\campaign\\pre_final_reduction.json",
        pre_final_reduction_artifact_sha256: hash("e"),
        selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
        execution_blocked: true,
        blocked_stage: "final_test_shard",
        blocked_shard_ordinal: 0,
        blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
        next_required_action: "start_new_campaign",
      }),
    );
    expect(finalBlocked.blockedShardOrdinal).toBe(0);

    const lateFinalBlocked = normalizeVcetTemporalCampaignStatus(
      preparedStatus({
        state: "final_test_complete",
        completed_selection_shard_count: 2,
        completed_final_test_shard_count: 1,
        winner_candidate_id: "candidate-7",
        winner_lock_sha256: hash("d"),
        pre_final_reduction_artifact_path:
          "C:\\campaign\\pre_final_reduction.json",
        pre_final_reduction_artifact_sha256: hash("e"),
        selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
        final_reduction_artifact_path: "C:\\campaign\\final.json",
        final_reduction_artifact_sha256: hash("f"),
        final_report_xlsx_path: "C:\\campaign\\final.xlsx",
        execution_blocked: true,
        blocked_stage: "final_test_shard",
        blocked_shard_ordinal: 0,
        blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
        next_required_action: "start_new_campaign",
      }),
    );
    expect(lateFinalBlocked.state).toBe("final_test_complete");
    expect(lateFinalBlocked.blockedShardOrdinal).toBe(0);

    const latePreFinalBlockWithFinalArtifacts =
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          state: "final_test_complete",
          completed_selection_shard_count: 2,
          completed_final_test_shard_count: 1,
          winner_candidate_id: "candidate-7",
          winner_lock_sha256: hash("d"),
          pre_final_reduction_artifact_path:
            "C:\\campaign\\pre_final_reduction.json",
          pre_final_reduction_artifact_sha256: hash("e"),
          selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
          final_reduction_artifact_path: "C:\\campaign\\final.json",
          final_reduction_artifact_sha256: hash("f"),
          final_report_xlsx_path: "C:\\campaign\\final.xlsx",
          execution_blocked: true,
          blocked_stage: "pre_final_reduction",
          blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
          next_required_action: "start_new_campaign",
        }),
      );
    expect(latePreFinalBlockWithFinalArtifacts.state).toBe(
      "final_test_complete",
    );
    expect(latePreFinalBlockWithFinalArtifacts.blockedStage).toBe(
      "pre_final_reduction",
    );

    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          state: "selection_complete",
          completed_selection_shard_count: 2,
          completed_final_test_shard_count: 1,
          winner_candidate_id: "candidate-7",
          winner_lock_sha256: hash("d"),
          pre_final_reduction_artifact_path:
            "C:\\campaign\\pre_final_reduction.json",
          pre_final_reduction_artifact_sha256: hash("e"),
          selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
          final_reduction_artifact_path: "C:\\campaign\\final.json",
          final_reduction_artifact_sha256: hash("f"),
          final_report_xlsx_path: "C:\\campaign\\final.xlsx",
          next_required_action: "none",
        }),
      ),
    ).toThrow("inconsistent staged provenance");

    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          execution_blocked: true,
          blocked_stage: "final_test_shard",
          blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
          next_required_action: "start_new_campaign",
        }),
      ),
    ).toThrow("missing the blocked final shard ordinal");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          execution_blocked: true,
          blocked_stage: "pre_final_reduction",
          blocked_shard_ordinal: 0,
          blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
          next_required_action: "start_new_campaign",
        }),
      ),
    ).toThrow("blocked pre-final evidence");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          execution_blocked: true,
          blocked_stage: "pre_final_reduction",
          blocked_reason_code: "raw_backend_failure",
          next_required_action: "start_new_campaign",
        }),
      ),
    ).toThrow("blocked-reason evidence");
  });

  it("normalizes report blocks without downgrading retained scientific artifacts", () => {
    const selectionReportBlocked = normalizeVcetTemporalCampaignStatus(
      preparedStatus({
        state: "final_test_complete",
        completed_selection_shard_count: 2,
        completed_final_test_shard_count: 1,
        winner_candidate_id: "candidate-7",
        winner_lock_sha256: hash("d"),
        pre_final_reduction_artifact_path:
          "C:\\campaign\\pre_final_reduction.json",
        pre_final_reduction_artifact_sha256: hash("e"),
        selection_report_xlsx_path: null,
        final_reduction_artifact_path: "C:\\campaign\\final.json",
        final_reduction_artifact_sha256: hash("f"),
        final_report_xlsx_path: "C:\\campaign\\final.xlsx",
        execution_blocked: true,
        blocked_stage: "selection_report",
        blocked_reason_code: "report_receipt_invalid_or_foreign",
        next_required_action: "start_new_campaign",
      }),
    );
    expect(selectionReportBlocked).toEqual(
      expect.objectContaining({
        state: "final_test_complete",
        selectionReportXlsxPath: null,
        finalReductionArtifactPath: "C:\\campaign\\final.json",
        finalReportXlsxPath: "C:\\campaign\\final.xlsx",
        blockedStage: "selection_report",
        blockedShardOrdinal: null,
        blockedReasonCode: "report_receipt_invalid_or_foreign",
      }),
    );

    const finalReportBlocked = normalizeVcetTemporalCampaignStatus(
      preparedStatus({
        state: "selection_complete",
        completed_selection_shard_count: 2,
        completed_final_test_shard_count: 1,
        winner_candidate_id: "candidate-7",
        winner_lock_sha256: hash("d"),
        pre_final_reduction_artifact_path:
          "C:\\campaign\\pre_final_reduction.json",
        pre_final_reduction_artifact_sha256: hash("e"),
        selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
        final_reduction_artifact_path: "C:\\campaign\\final.json",
        final_reduction_artifact_sha256: hash("f"),
        final_report_xlsx_path: null,
        execution_blocked: true,
        blocked_stage: "final_report",
        blocked_reason_code: "report_xlsx_not_recoverable",
        next_required_action: "start_new_campaign",
      }),
    );
    expect(finalReportBlocked).toEqual(
      expect.objectContaining({
        state: "selection_complete",
        selectionReportXlsxPath: "C:\\campaign\\selection.xlsx",
        finalReductionArtifactPath: "C:\\campaign\\final.json",
        finalReportXlsxPath: null,
        blockedStage: "final_report",
        blockedShardOrdinal: null,
        blockedReasonCode: "report_xlsx_not_recoverable",
      }),
    );
  });

  it("normalizes receipt poison discovered before the corresponding scientific reduction", () => {
    const earlySelectionReportPoison = normalizeVcetTemporalCampaignStatus(
      preparedStatus({
        completed_selection_shard_count: 1,
        execution_blocked: true,
        blocked_stage: "selection_report",
        blocked_reason_code: "report_receipt_invalid_or_foreign",
        next_required_action: "start_new_campaign",
      }),
    );
    expect(earlySelectionReportPoison).toEqual(
      expect.objectContaining({
        state: "prepared",
        completedSelectionShardCount: 1,
        winnerCandidateId: null,
        preFinalReductionArtifactPath: null,
        selectionReportXlsxPath: null,
        finalReductionArtifactPath: null,
        finalReportXlsxPath: null,
        blockedStage: "selection_report",
        blockedShardOrdinal: null,
        blockedReasonCode: "report_receipt_invalid_or_foreign",
        nextRequiredAction: "start_new_campaign",
      }),
    );

    const earlyFinalReportPoison = normalizeVcetTemporalCampaignStatus(
      preparedStatus({
        state: "selection_complete",
        completed_selection_shard_count: 2,
        completed_final_test_shard_count: 0,
        winner_candidate_id: "candidate-7",
        winner_lock_sha256: hash("d"),
        pre_final_reduction_artifact_path:
          "C:\\campaign\\pre_final_reduction.json",
        pre_final_reduction_artifact_sha256: hash("e"),
        selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
        execution_blocked: true,
        blocked_stage: "final_report",
        blocked_reason_code: "report_receipt_invalid_or_foreign",
        next_required_action: "start_new_campaign",
      }),
    );
    expect(earlyFinalReportPoison).toEqual(
      expect.objectContaining({
        state: "selection_complete",
        completedFinalTestShardCount: 0,
        preFinalReductionArtifactPath: "C:\\campaign\\pre_final_reduction.json",
        selectionReportXlsxPath: "C:\\campaign\\selection.xlsx",
        finalReductionArtifactPath: null,
        finalReportXlsxPath: null,
        blockedStage: "final_report",
        blockedShardOrdinal: null,
        blockedReasonCode: "report_receipt_invalid_or_foreign",
        nextRequiredAction: "start_new_campaign",
      }),
    );
  });

  it("rejects report blocks with mismatched reasons, ordinals, or artifact state", () => {
    const selectionReportBlock = {
      state: "prepared",
      completed_selection_shard_count: 2,
      winner_candidate_id: "candidate-7",
      winner_lock_sha256: hash("d"),
      pre_final_reduction_artifact_path:
        "C:\\campaign\\pre_final_reduction.json",
      pre_final_reduction_artifact_sha256: hash("e"),
      execution_blocked: true,
      blocked_stage: "selection_report",
      blocked_reason_code: "report_receipt_invalid_or_foreign",
      next_required_action: "start_new_campaign",
    };
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          ...selectionReportBlock,
          blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
        }),
      ),
    ).toThrow("blocked-reason evidence");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          ...selectionReportBlock,
          blocked_shard_ordinal: 0,
        }),
      ),
    ).toThrow("blocked selection-report evidence");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          ...selectionReportBlock,
          state: "selection_complete",
          selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
        }),
      ),
    ).toThrow("blocked selection-report evidence");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          completed_selection_shard_count: 2,
          execution_blocked: true,
          blocked_stage: "pre_final_reduction",
          blocked_reason_code: "report_xlsx_not_recoverable",
          next_required_action: "start_new_campaign",
        }),
      ),
    ).toThrow("blocked-reason evidence");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          completed_selection_shard_count: 1,
          execution_blocked: true,
          blocked_stage: "selection_report",
          blocked_reason_code: "report_xlsx_not_recoverable",
          next_required_action: "start_new_campaign",
        }),
      ),
    ).toThrow("blocked selection-report evidence");
    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          state: "selection_complete",
          completed_selection_shard_count: 2,
          winner_candidate_id: "candidate-7",
          winner_lock_sha256: hash("d"),
          pre_final_reduction_artifact_path:
            "C:\\campaign\\pre_final_reduction.json",
          pre_final_reduction_artifact_sha256: hash("e"),
          selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
          execution_blocked: true,
          blocked_stage: "final_report",
          blocked_reason_code: "report_xlsx_not_recoverable",
          next_required_action: "start_new_campaign",
        }),
      ),
    ).toThrow("blocked final-report evidence");
  });

  it("prioritizes incomplete selection-projection repair over selection-report recovery", () => {
    const repairable = normalizeVcetTemporalCampaignStatus(
      preparedStatus({
        completed_selection_shard_count: 1,
        winner_candidate_id: "candidate-7",
        winner_lock_sha256: hash("d"),
        pre_final_reduction_artifact_path:
          "C:\\campaign\\pre_final_reduction.json",
        pre_final_reduction_artifact_sha256: hash("e"),
        next_required_action: "resume_selection",
      }),
    );
    expect(repairable.state).toBe("prepared");
    expect(repairable.nextRequiredAction).toBe("resume_selection");

    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          completed_selection_shard_count: 1,
          winner_candidate_id: "candidate-7",
          winner_lock_sha256: hash("d"),
          pre_final_reduction_artifact_path:
            "C:\\campaign\\pre_final_reduction.json",
          pre_final_reduction_artifact_sha256: hash("e"),
          next_required_action: "resume_selection_report",
        }),
      ),
    ).toThrow("next required action");

    expect(() =>
      normalizeVcetTemporalCampaignStatus(
        preparedStatus({
          state: "selection_complete",
          completed_selection_shard_count: 1,
          winner_candidate_id: "candidate-7",
          winner_lock_sha256: hash("d"),
          pre_final_reduction_artifact_path:
            "C:\\campaign\\pre_final_reduction.json",
          pre_final_reduction_artifact_sha256: hash("e"),
          selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
          next_required_action: "confirm_and_open_final_test",
        }),
      ),
    ).toThrow("staged provenance");
  });

  it("rejects a valid but stale stage response from a mutating action", async () => {
    invokeCommandMock.mockResolvedValue(
      preparedStatus({
        state: "selection_complete",
        completed_selection_shard_count: 2,
        winner_candidate_id: "candidate-7",
        winner_lock_sha256: hash("d"),
        pre_final_reduction_artifact_path:
          "C:\\campaign\\pre_final_reduction.json",
        pre_final_reduction_artifact_sha256: hash("e"),
        selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
        next_required_action: "confirm_and_open_final_test",
      }) as never,
    );

    await expect(
      openVcetTemporalCampaignFinalTest({
        preparationArtifactPath: "C:\\campaign\\preparation.json",
        expectedPreparationArtifactSha256: hash("a"),
        expectedWinnerLockSha256: hash("d"),
      }),
    ).rejects.toThrow("expected final_test_complete");
  });

  it("returns a strict blocked status from the mutating stage that discovered it", async () => {
    invokeCommandMock.mockResolvedValue(
      preparedStatus({
        completed_selection_shard_count: 2,
        execution_blocked: true,
        blocked_stage: "pre_final_reduction",
        blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
        next_required_action: "start_new_campaign",
      }) as never,
    );

    const blocked = await runVcetTemporalCampaignSelection({
      preparationArtifactPath: "C:\\campaign\\preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
    });

    expect(blocked.executionBlocked).toBe(true);
    expect(blocked.blockedStage).toBe("pre_final_reduction");
    expect(blocked.nextRequiredAction).toBe("start_new_campaign");
  });

  it("returns report blocks from their owning mutating actions", async () => {
    invokeCommandMock
      .mockResolvedValueOnce(
        preparedStatus({
          completed_selection_shard_count: 2,
          winner_candidate_id: "candidate-7",
          winner_lock_sha256: hash("d"),
          pre_final_reduction_artifact_path:
            "C:\\campaign\\pre_final_reduction.json",
          pre_final_reduction_artifact_sha256: hash("e"),
          execution_blocked: true,
          blocked_stage: "selection_report",
          blocked_reason_code: "report_xlsx_not_recoverable",
          next_required_action: "start_new_campaign",
        }) as never,
      )
      .mockResolvedValueOnce(
        preparedStatus({
          state: "selection_complete",
          completed_selection_shard_count: 2,
          completed_final_test_shard_count: 1,
          winner_candidate_id: "candidate-7",
          winner_lock_sha256: hash("d"),
          pre_final_reduction_artifact_path:
            "C:\\campaign\\pre_final_reduction.json",
          pre_final_reduction_artifact_sha256: hash("e"),
          selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
          final_reduction_artifact_path: "C:\\campaign\\final.json",
          final_reduction_artifact_sha256: hash("f"),
          execution_blocked: true,
          blocked_stage: "final_report",
          blocked_reason_code: "report_receipt_invalid_or_foreign",
          next_required_action: "start_new_campaign",
        }) as never,
      );

    const selectionBlocked = await runVcetTemporalCampaignSelection({
      preparationArtifactPath: "C:\\campaign\\preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
    });
    expect(selectionBlocked.blockedStage).toBe("selection_report");

    const finalBlocked = await openVcetTemporalCampaignFinalTest({
      preparationArtifactPath: "C:\\campaign\\preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
      expectedWinnerLockSha256: hash("d"),
    });
    expect(finalBlocked.blockedStage).toBe("final_report");
  });

  it("returns early receipt-poison blocks from their owning mutating actions", async () => {
    invokeCommandMock
      .mockResolvedValueOnce(
        preparedStatus({
          completed_selection_shard_count: 1,
          execution_blocked: true,
          blocked_stage: "selection_report",
          blocked_reason_code: "report_receipt_invalid_or_foreign",
          next_required_action: "start_new_campaign",
        }) as never,
      )
      .mockResolvedValueOnce(
        preparedStatus({
          state: "selection_complete",
          completed_selection_shard_count: 2,
          winner_candidate_id: "candidate-7",
          winner_lock_sha256: hash("d"),
          pre_final_reduction_artifact_path:
            "C:\\campaign\\pre_final_reduction.json",
          pre_final_reduction_artifact_sha256: hash("e"),
          selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
          execution_blocked: true,
          blocked_stage: "final_report",
          blocked_reason_code: "report_receipt_invalid_or_foreign",
          next_required_action: "start_new_campaign",
        }) as never,
      );

    const selectionBlocked = await runVcetTemporalCampaignSelection({
      preparationArtifactPath: "C:\\campaign\\preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
    });
    expect(selectionBlocked.preFinalReductionArtifactPath).toBeNull();
    expect(selectionBlocked.blockedStage).toBe("selection_report");

    const finalBlocked = await openVcetTemporalCampaignFinalTest({
      preparationArtifactPath: "C:\\campaign\\preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
      expectedWinnerLockSha256: hash("d"),
    });
    expect(finalBlocked.finalReductionArtifactPath).toBeNull();
    expect(finalBlocked.blockedStage).toBe("final_report");
  });

  it("rejects a terminal block from the wrong mutating stage or a new preparation", async () => {
    const finalBlocked = preparedStatus({
      state: "selection_complete",
      completed_selection_shard_count: 2,
      winner_candidate_id: "candidate-7",
      winner_lock_sha256: hash("d"),
      pre_final_reduction_artifact_path:
        "C:\\campaign\\pre_final_reduction.json",
      pre_final_reduction_artifact_sha256: hash("e"),
      selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
      execution_blocked: true,
      blocked_stage: "final_test_shard",
      blocked_shard_ordinal: 0,
      blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
      next_required_action: "start_new_campaign",
    });
    invokeCommandMock.mockResolvedValueOnce(finalBlocked as never);
    await expect(
      runVcetTemporalCampaignSelection({
        preparationArtifactPath: "C:\\campaign\\preparation.json",
        expectedPreparationArtifactSha256: hash("a"),
      }),
    ).rejects.toThrow("terminal execution block");

    invokeCommandMock.mockResolvedValueOnce(
      preparedStatus({
        completed_selection_shard_count: 2,
        execution_blocked: true,
        blocked_stage: "pre_final_reduction",
        blocked_reason_code: "durable_stage_failed_interrupted_or_invalid",
        next_required_action: "start_new_campaign",
      }) as never,
    );
    await expect(
      prepareVcetTemporalCampaign({
        sourceCampaignSummaryPath: "C:\\picked\\summary.json",
        planDraft: createDefaultVcetTemporalPlanDraft(),
      }),
    ).rejects.toThrow("terminal execution block");
  });

  it("sends the full locked hash and explicit final-test confirmation", async () => {
    invokeCommandMock.mockResolvedValue(
      preparedStatus({
        state: "final_test_complete",
        completed_selection_shard_count: 2,
        completed_final_test_shard_count: 1,
        winner_candidate_id: "candidate-7",
        winner_lock_sha256: hash("d"),
        pre_final_reduction_artifact_path:
          "C:\\campaign\\pre_final_reduction.json",
        pre_final_reduction_artifact_sha256: hash("e"),
        selection_report_xlsx_path: "C:\\campaign\\selection.xlsx",
        final_reduction_artifact_path: "C:\\campaign\\final_reduction.json",
        final_reduction_artifact_sha256: hash("f"),
        final_report_xlsx_path: "C:\\campaign\\final.xlsx",
        next_required_action: "none",
        winner_runner_up_margin: 0.125,
      }) as never,
    );

    const status = await openVcetTemporalCampaignFinalTest({
      preparationArtifactPath: "C:\\campaign\\preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
      expectedWinnerLockSha256: hash("d"),
    });

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "open_vcet_temporal_campaign_final_test",
      {
        req: {
          preparation_artifact_path: "C:\\campaign\\preparation.json",
          expected_preparation_artifact_sha256: hash("a"),
          expected_winner_lock_sha256: hash("d"),
          confirm_final_test_access: true,
        },
      },
    );
    expect(status.winnerRunnerUpMargin).toBe(0.125);
  });
});
