import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  createDefaultVcetTemporalPlanDraft,
  getVcetTemporalCampaignStatus,
  openVcetTemporalCampaignFinalTest,
  prepareVcetTemporalCampaign,
  runVcetTemporalCampaignSelection,
  type VcetTemporalCampaignStatus,
  vcetTemporalNativePathKey,
} from "../services/vcetTemporalCampaignService";
import {
  useVcetTemporalCampaignStore,
  VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY,
} from "./vcetTemporalCampaignStore";

vi.mock("../services/vcetTemporalCampaignService", async (importOriginal) => {
  const original =
    await importOriginal<
      typeof import("../services/vcetTemporalCampaignService")
    >();
  return {
    ...original,
    prepareVcetTemporalCampaign: vi.fn(),
    runVcetTemporalCampaignSelection: vi.fn(),
    openVcetTemporalCampaignFinalTest: vi.fn(),
    getVcetTemporalCampaignStatus: vi.fn(),
    revealVcetTemporalCampaignPath: vi.fn(),
    openVcetTemporalCampaignWorkbook: vi.fn(),
  };
});

const hash = (digit: string) => `sha256:${digit.repeat(64)}`;
const sourceIdentity = (path: string, digit = "1") => ({
  nativePathKey: vcetTemporalNativePathKey(path),
  summaryContentFingerprint: `campaign-summary-v2-fnv1a64-${digit.repeat(16)}`,
});

function admitSource(
  store: ReturnType<typeof useVcetTemporalCampaignStore>,
  path = "C:/campaign/summary.json",
  digit = "1",
): void {
  store.setEligibleSource(path, "", sourceIdentity(path, digit));
}

function status(
  overrides: Partial<VcetTemporalCampaignStatus> = {},
): VcetTemporalCampaignStatus {
  return {
    schemaVersion: 1,
    campaignId: "temporal-1",
    state: "prepared",
    preparationArtifactPath: "C:/temporal/preparation.json",
    preparationArtifactSha256: hash("a"),
    sourceCampaignSummaryPath: "C:/campaign/summary.json",
    sourceCampaignSummarySha256: hash("b"),
    universeSha256: hash("c"),
    candidateCount: 8,
    datasetCount: 3,
    selectionDatasetCount: 2,
    finalTestDatasetCount: 1,
    selectionShardCount: 2,
    completedSelectionShardCount: 0,
    finalTestShardCount: 1,
    completedFinalTestShardCount: 0,
    winnerCandidateId: null,
    winnerLockSha256: null,
    winnerRunnerUpMargin: null,
    preFinalReductionArtifactPath: null,
    preFinalReductionArtifactSha256: null,
    finalReductionArtifactPath: null,
    finalReductionArtifactSha256: null,
    selectionReportXlsxPath: null,
    finalReportXlsxPath: null,
    executionBlocked: false,
    blockedStage: null,
    blockedShardOrdinal: null,
    blockedReasonCode: null,
    nextRequiredAction: "run_selection",
    limitations: ["product_v1_is_bounded_to_64_datasets"],
    ...overrides,
  };
}

function installStorageHarness(): void {
  const values = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, String(value)),
    removeItem: (key: string) => values.delete(key),
    clear: () => values.clear(),
  });
}

describe("VCET temporal campaign store", () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
    installStorageHarness();
    localStorage.clear();
    setActivePinia(createPinia());
    vi.mocked(prepareVcetTemporalCampaign).mockReset();
    vi.mocked(getVcetTemporalCampaignStatus).mockReset();
    vi.mocked(openVcetTemporalCampaignFinalTest).mockReset();
    vi.mocked(runVcetTemporalCampaignSelection).mockReset();
  });

  it("hydrates only a safe plan draft and compact preparation identity", () => {
    localStorage.setItem(
      VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY,
      JSON.stringify({
        schema_version: 1,
        plan_draft: {
          ...createDefaultVcetTemporalPlanDraft(),
          trainBars: 4096,
          positionBoundaryPolicy: "reset_flat_force_liquidate",
        },
        current_preparation: {
          source_campaign_summary_path: "C:/campaign/summary.json",
          preparation_artifact_path: "C:/temporal/preparation.json",
          preparation_artifact_sha256: hash("a"),
          prepared_plan_draft_snapshot: createDefaultVcetTemporalPlanDraft(),
        },
        status: { ignored_artifact: "not persisted by this store" },
        timestamps_utc_ms: Array.from({ length: 100 }, (_, index) => index),
      }),
    );

    const store = useVcetTemporalCampaignStore();
    store.hydrate();

    expect(store.planDraft.trainBars).toBe("2048");
    expect(store.planDraft.positionBoundaryPolicy).toBe(
      "reset_flat_discard_open",
    );
    expect(store.currentPreparationArtifactPath).toBe(
      "C:/temporal/preparation.json",
    );
    expect(store.currentPreparationArtifactSha256).toBe(hash("a"));
    expect(store.currentPreparationPlanDraft?.trainBars).toBe("2048");
    expect(store.status).toBeNull();
    expect(store).not.toHaveProperty("timestamps_utc_ms");
  });

  it("rejects an oversized persisted snapshot before JSON hydration", () => {
    localStorage.setItem(
      VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY,
      JSON.stringify({
        schema_version: 1,
        plan_draft: {
          ...createDefaultVcetTemporalPlanDraft(),
          trainBars: "4096",
        },
        oversized_artifact: "x".repeat(40_000),
      }),
    );

    const store = useVcetTemporalCampaignStore();
    store.hydrate();

    expect(store.planDraft.trainBars).toBe("2048");
    expect(store.hasCurrentPreparation).toBe(false);
  });

  it("drops a malformed persisted preparation identity", () => {
    localStorage.setItem(
      VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY,
      JSON.stringify({
        schema_version: 1,
        plan_draft: createDefaultVcetTemporalPlanDraft(),
        current_preparation: {
          source_campaign_summary_path: "C:/campaign/summary.json",
          preparation_artifact_path: "C:/temporal/preparation.json",
          preparation_artifact_sha256: "not-a-hash",
        },
      }),
    );

    const store = useVcetTemporalCampaignStore();
    store.hydrate();

    expect(store.hasCurrentPreparation).toBe(false);
    expect(store.currentSourceCampaignSummaryPath).toBe("");
  });

  it("persists no status payload after preparation and resumes by compact identity", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValue(status());
    vi.mocked(getVcetTemporalCampaignStatus).mockResolvedValue(
      status({ completedSelectionShardCount: 1 }),
    );
    const first = useVcetTemporalCampaignStore();
    first.hydrate();
    first.setEligibleSource(
      "C:/campaign/summary.json",
      "Structurally eligible terminal source.",
      sourceIdentity("C:/campaign/summary.json"),
    );

    expect(await first.prepare()).toBe(true);
    const persisted = JSON.parse(
      localStorage.getItem(VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY) || "{}",
    ) as Record<string, unknown>;
    expect(persisted).toEqual(
      expect.objectContaining({
        schema_version: 1,
        current_preparation: {
          source_campaign_summary_path: "C:/campaign/summary.json",
          source_summary_content_fingerprint:
            "campaign-summary-v2-fnv1a64-1111111111111111",
          preparation_artifact_path: "C:/temporal/preparation.json",
          preparation_artifact_sha256: hash("a"),
          prepared_plan_draft_snapshot: createDefaultVcetTemporalPlanDraft(),
        },
      }),
    );
    expect(persisted).not.toHaveProperty("status");
    expect(JSON.stringify(persisted)).not.toContain("limitations");

    setActivePinia(createPinia());
    const reloaded = useVcetTemporalCampaignStore();
    expect(await reloaded.resumeAfterHydration()).toBe(true);
    expect(getVcetTemporalCampaignStatus).toHaveBeenCalledWith({
      preparationArtifactPath: "C:/temporal/preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
    });
    expect(reloaded.status?.completedSelectionShardCount).toBe(1);
    expect(reloaded.currentPreparationPlanDraft?.trainBars).toBe("2048");
  });

  it("retains source and plan identity when resume canonicalizes a preparation alias", async () => {
    const preparationAlias = "C:/temporal-alias/preparation.json";
    const canonicalPreparationPath = "C:/temporal/preparation.json";
    const sealedPlan = {
      ...createDefaultVcetTemporalPlanDraft(),
      trainBars: "4096",
    };
    localStorage.setItem(
      VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY,
      JSON.stringify({
        schema_version: 1,
        plan_draft: sealedPlan,
        current_preparation: {
          source_campaign_summary_path: "C:/campaign/summary.json",
          source_summary_content_fingerprint:
            "campaign-summary-v2-fnv1a64-1111111111111111",
          preparation_artifact_path: preparationAlias,
          preparation_artifact_sha256: hash("a"),
          prepared_plan_draft_snapshot: sealedPlan,
        },
      }),
    );
    vi.mocked(getVcetTemporalCampaignStatus).mockResolvedValueOnce(
      status({
        preparationArtifactPath: canonicalPreparationPath,
        preparationArtifactSha256: hash("a"),
        completedSelectionShardCount: 1,
      }),
    );

    const store = useVcetTemporalCampaignStore();
    expect(await store.resumeAfterHydration()).toBe(true);

    expect(getVcetTemporalCampaignStatus).toHaveBeenCalledWith({
      preparationArtifactPath: preparationAlias,
      expectedPreparationArtifactSha256: hash("a"),
    });
    expect(store.currentPreparationArtifactPath).toBe(canonicalPreparationPath);
    expect(store.currentPreparationArtifactSha256).toBe(hash("a"));
    expect(store.currentSourceSummaryContentFingerprint).toBe(
      "campaign-summary-v2-fnv1a64-1111111111111111",
    );
    expect(store.currentPreparationPlanDraft).toEqual(sealedPlan);
    expect(store.status?.completedSelectionShardCount).toBe(1);
  });

  it("does not adopt an initial native preparation when app storage denies its resume identity", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(status());
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    store.setPlanField("reportLabel", "Retry this preparation");
    const retainedSnapshot = localStorage.getItem(
      VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY,
    );
    const deniedSetItem = vi.fn(() => {
      throw new Error("storage write denied");
    });
    vi.stubGlobal("localStorage", {
      getItem: () => retainedSnapshot,
      setItem: deniedSetItem,
      removeItem: vi.fn(),
    });

    expect(await store.prepare()).toBe(false);
    expect(prepareVcetTemporalCampaign).toHaveBeenCalledTimes(1);
    expect(deniedSetItem).toHaveBeenCalledTimes(1);
    expect(store.status).toBeNull();
    expect(store.hasCurrentPreparation).toBe(false);
    expect(store.currentSourceCampaignSummaryPath).toBe("");
    expect(store.currentSourceSummaryContentFingerprint).toBe("");
    expect(store.currentPreparationArtifactPath).toBe("");
    expect(store.currentPreparationArtifactSha256).toBe("");
    expect(store.currentPreparationPlanDraft).toBeNull();
    expect(store.eligibleSourcePath).toBe("C:/campaign/summary.json");
    expect(store.planDraft.reportLabel).toBe("Retry this preparation");
    expect(store.error).toContain("local resume identity could not be saved");
    expect(store.error).toContain("not adopted as the active campaign");
    expect(store.notice).toBe("");
    expect(localStorage.getItem(VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY)).toBe(
      retainedSnapshot,
    );
  });

  it("freezes the submitted plan, flags next-draft/source drift, and requires an explicit replacement", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(status());
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);

    expect(await store.prepare()).toBe(true);
    expect(store.currentPreparationPlanDraft?.trainBars).toBe("2048");
    expect(store.activePlanComparisonKnown).toBe(true);

    expect(await store.prepare({ replaceActivePreparation: true })).toBe(false);
    expect(store.error).toContain("no confirmed identity drift");
    expect(prepareVcetTemporalCampaign).toHaveBeenCalledTimes(1);

    store.setPlanField("trainBars", "4096");
    admitSource(store, "C:/campaign/new-summary.json", "2");
    expect(store.currentPreparationPlanDraft?.trainBars).toBe("2048");
    expect(store.draftDiffersFromActive).toBe(true);
    expect(store.sourceDiffersFromActive).toBe(true);

    expect(await store.prepare()).toBe(false);
    expect(store.error).toContain("requires explicit confirmation");
    expect(prepareVcetTemporalCampaign).toHaveBeenCalledTimes(1);

    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(
      status({
        campaignId: "temporal-2",
        preparationArtifactPath: "C:/temporal/preparation-2.json",
        preparationArtifactSha256: hash("e"),
        sourceCampaignSummaryPath: "C:/campaign/new-summary.json",
        sourceCampaignSummarySha256: hash("f"),
      }),
    );
    expect(await store.prepare({ replaceActivePreparation: true })).toBe(true);
    expect(store.currentPreparationArtifactPath).toBe(
      "C:/temporal/preparation-2.json",
    );
    expect(store.currentPreparationPlanDraft?.trainBars).toBe("4096");
    expect(store.draftDiffersFromActive).toBe(false);
    expect(store.sourceDiffersFromActive).toBe(false);
  });

  it("compares source drift by native Windows identity and retained summary content", async () => {
    const pickerPath = "C:\\Campaign\\Summary.json";
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(
      status({
        sourceCampaignSummaryPath: "\\\\?\\c:\\campaign\\SUMMARY.json",
      }),
    );
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    store.setEligibleSource(pickerPath, "", sourceIdentity(pickerPath));

    expect(await store.prepare()).toBe(true);
    expect(store.activeSourceComparisonKnown).toBe(true);
    expect(store.sourceDiffersFromActive).toBe(false);

    store.setEligibleSource(
      "c:/campaign/summary.json",
      "",
      sourceIdentity("c:/campaign/summary.json"),
    );
    expect(store.sourceDiffersFromActive).toBe(false);

    store.setEligibleSource(
      "c:/campaign/summary.json",
      "",
      sourceIdentity("c:/campaign/summary.json", "2"),
    );
    expect(store.sourceDiffersFromActive).toBe(true);
  });

  it("keeps the active campaign intact when an explicit replacement fails", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(status());
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    expect(await store.prepare()).toBe(true);
    const activeStatus = store.status;

    store.setPlanField("trainBars", "4096");
    vi.mocked(prepareVcetTemporalCampaign).mockRejectedValueOnce(
      new Error("replacement preparation failed"),
    );
    expect(await store.prepare({ replaceActivePreparation: true })).toBe(false);

    expect(store.status).toBe(activeStatus);
    expect(store.currentPreparationArtifactPath).toBe(
      "C:/temporal/preparation.json",
    );
    expect(store.currentPreparationArtifactSha256).toBe(hash("a"));
    expect(store.currentPreparationPlanDraft?.trainBars).toBe("2048");
    expect(store.error).toContain("replacement preparation failed");
    expect(store.notice).toContain("remains unchanged");
  });

  it("rolls back a native replacement when app storage cannot commit the new resume identity", async () => {
    const initialStatus = status();
    const replacementStatus = status({
      campaignId: "temporal-2",
      preparationArtifactPath: "C:/temporal/preparation-2.json",
      preparationArtifactSha256: hash("e"),
      sourceCampaignSummaryPath: "C:/campaign/new-summary.json",
      sourceCampaignSummarySha256: hash("f"),
    });
    vi.mocked(prepareVcetTemporalCampaign)
      .mockResolvedValueOnce(initialStatus)
      .mockResolvedValueOnce(replacementStatus);
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    expect(await store.prepare()).toBe(true);
    const activeStatus = store.status;
    const activeFingerprint = store.currentSourceSummaryContentFingerprint;

    store.setPlanField("trainBars", "4096");
    admitSource(store, "C:/campaign/new-summary.json", "2");
    const retainedSnapshot = localStorage.getItem(
      VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY,
    );
    const deniedSetItem = vi.fn(() => {
      throw new Error("storage write denied");
    });
    vi.stubGlobal("localStorage", {
      getItem: () => retainedSnapshot,
      setItem: deniedSetItem,
      removeItem: vi.fn(),
    });

    expect(await store.prepare({ replaceActivePreparation: true })).toBe(false);
    expect(prepareVcetTemporalCampaign).toHaveBeenCalledTimes(2);
    expect(deniedSetItem).toHaveBeenCalledTimes(1);
    expect(store.status).toBe(activeStatus);
    expect(store.currentSourceCampaignSummaryPath).toBe(
      "C:/campaign/summary.json",
    );
    expect(store.currentSourceSummaryContentFingerprint).toBe(
      activeFingerprint,
    );
    expect(store.currentPreparationArtifactPath).toBe(
      "C:/temporal/preparation.json",
    );
    expect(store.currentPreparationArtifactSha256).toBe(hash("a"));
    expect(store.currentPreparationPlanDraft?.trainBars).toBe("2048");
    expect(store.planDraft.trainBars).toBe("4096");
    expect(store.eligibleSourcePath).toBe("C:/campaign/new-summary.json");
    expect(store.error).toContain("local resume identity could not be saved");
    expect(store.error).toContain("prior active campaign pointer and status");
    expect(store.notice).toBe("");
    expect(localStorage.getItem(VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY)).toBe(
      retainedSnapshot,
    );
  });

  it("exposes unknown plan comparison for a legacy preparation and safely forgets a stale identity", async () => {
    const originalDraft = {
      ...createDefaultVcetTemporalPlanDraft(),
      reportLabel: "Keep this next draft",
    };
    localStorage.setItem(
      VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY,
      JSON.stringify({
        schema_version: 1,
        plan_draft: originalDraft,
        current_preparation: {
          source_campaign_summary_path: "C:/campaign/summary.json",
          preparation_artifact_path: "C:/temporal/missing.json",
          preparation_artifact_sha256: hash("a"),
        },
      }),
    );
    vi.mocked(getVcetTemporalCampaignStatus).mockRejectedValueOnce(
      new Error("preparation artifact is missing"),
    );

    const store = useVcetTemporalCampaignStore();
    expect(await store.resumeAfterHydration()).toBe(false);
    expect(store.activePlanComparisonKnown).toBe(false);
    expect(store.canForgetStalePreparation).toBe(true);
    expect(store.forgetStalePreparation({ confirmed: false })).toBe(false);
    expect(store.hasCurrentPreparation).toBe(true);

    expect(store.forgetStalePreparation({ confirmed: true })).toBe(true);
    expect(store.hasCurrentPreparation).toBe(false);
    expect(store.planDraft.reportLabel).toBe("Keep this next draft");
    expect(store.notice).toContain("Evidence files were not deleted");
    expect(
      JSON.parse(
        localStorage.getItem(VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY) || "{}",
      ).current_preparation,
    ).toBeNull();
  });

  it("rolls back stale-identity forgetting when app storage cannot be updated", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(status());
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    expect(await store.prepare()).toBe(true);
    const retainedSnapshot = localStorage.getItem(
      VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY,
    );
    store.failClosed(new Error("preparation artifact is unreadable"));
    vi.stubGlobal("localStorage", {
      getItem: () => retainedSnapshot,
      setItem: () => {
        throw new Error("storage write denied");
      },
      removeItem: vi.fn(),
    });

    expect(store.forgetStalePreparation({ confirmed: true })).toBe(false);
    expect(store.hasCurrentPreparation).toBe(true);
    expect(store.currentPreparationArtifactPath).toBe(
      "C:/temporal/preparation.json",
    );
    expect(store.currentPreparationPlanDraft?.trainBars).toBe("2048");
    expect(store.error).toContain("could not be cleared from app storage");
  });

  it("retains a blocked identity and permits replacement only after confirmed plan or source drift", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(status());
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    expect(await store.prepare()).toBe(true);

    vi.mocked(runVcetTemporalCampaignSelection).mockResolvedValueOnce(
      status({
        completedSelectionShardCount: 2,
        executionBlocked: true,
        blockedStage: "pre_final_reduction",
        blockedReasonCode: "durable_stage_failed_interrupted_or_invalid",
        nextRequiredAction: "start_new_campaign",
      }),
    );
    expect(await store.runSelection()).toBe(false);
    expect(store.status?.executionBlocked).toBe(true);
    expect(store.canRunSelection).toBe(false);
    expect(store.canOpenFinalTest).toBe(false);
    expect(store.canForgetCurrentPreparation).toBe(false);
    expect(store.notice).toContain("Start a new campaign");

    expect(await store.prepare({ replaceActivePreparation: true })).toBe(false);
    expect(store.error).toContain("same deterministic source");
    expect(prepareVcetTemporalCampaign).toHaveBeenCalledTimes(1);

    store.setPlanField("trainBars", "4096");
    expect(store.draftDiffersFromActive).toBe(true);

    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(
      status({
        campaignId: "temporal-restarted",
        preparationArtifactPath: "C:/temporal/restarted.json",
        preparationArtifactSha256: hash("e"),
      }),
    );
    expect(await store.prepare({ replaceActivePreparation: true })).toBe(true);
    expect(store.status?.executionBlocked).toBe(false);
    expect(store.currentPreparationArtifactPath).toBe(
      "C:/temporal/restarted.json",
    );
  });

  it("retains scientific selection evidence under a terminal selection-report block", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(status());
    vi.mocked(runVcetTemporalCampaignSelection).mockResolvedValueOnce(
      status({
        completedSelectionShardCount: 2,
        winnerCandidateId: "candidate-4",
        winnerLockSha256: hash("d"),
        preFinalReductionArtifactPath: "C:/temporal/pre_final.json",
        preFinalReductionArtifactSha256: hash("e"),
        executionBlocked: true,
        blockedStage: "selection_report",
        blockedReasonCode: "report_receipt_invalid_or_foreign",
        nextRequiredAction: "start_new_campaign",
      }),
    );
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    expect(await store.prepare()).toBe(true);

    expect(await store.runSelection()).toBe(false);
    expect(store.status?.blockedStage).toBe("selection_report");
    expect(store.status?.preFinalReductionArtifactPath).toBe(
      "C:/temporal/pre_final.json",
    );
    expect(store.canRunSelection).toBe(false);
    expect(store.canForgetCurrentPreparation).toBe(false);
    expect(store.notice).toContain("scientific selection evidence");
    expect(store.notice).toContain(
      "report label, source identity, or validation plan",
    );

    store.setPlanField("reportLabel", "Distinct report receipt");
    expect(store.draftDiffersFromActive).toBe(true);
  });

  it("does not claim selection evidence when receipt poison blocks before the reduction", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(status());
    vi.mocked(runVcetTemporalCampaignSelection).mockResolvedValueOnce(
      status({
        completedSelectionShardCount: 1,
        executionBlocked: true,
        blockedStage: "selection_report",
        blockedReasonCode: "report_receipt_invalid_or_foreign",
        nextRequiredAction: "start_new_campaign",
      }),
    );
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    expect(await store.prepare()).toBe(true);

    expect(await store.runSelection()).toBe(false);
    expect(store.status?.blockedStage).toBe("selection_report");
    expect(store.status?.winnerCandidateId).toBeNull();
    expect(store.status?.preFinalReductionArtifactPath).toBeNull();
    expect(store.canRunSelection).toBe(false);
    expect(store.canForgetCurrentPreparation).toBe(false);
    expect(store.notice).toContain(
      "before a locked winner or scientific selection reduction was retained",
    );
    expect(store.notice).not.toContain("evidence remain retained");

    expect(await store.prepare({ replaceActivePreparation: true })).toBe(false);
    expect(store.error).toContain("same deterministic source");
    expect(prepareVcetTemporalCampaign).toHaveBeenCalledTimes(1);
  });

  it("retains the final reduction under a terminal final-report block", async () => {
    const selectionComplete = status({
      state: "selection_complete",
      completedSelectionShardCount: 2,
      winnerCandidateId: "candidate-4",
      winnerLockSha256: hash("d"),
      preFinalReductionArtifactPath: "C:/temporal/pre_final.json",
      preFinalReductionArtifactSha256: hash("e"),
      selectionReportXlsxPath: "C:/temporal/selection.xlsx",
      nextRequiredAction: "confirm_and_open_final_test",
    });
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(status());
    vi.mocked(openVcetTemporalCampaignFinalTest).mockResolvedValueOnce(
      status({
        ...selectionComplete,
        completedFinalTestShardCount: 1,
        finalReductionArtifactPath: "C:/temporal/final.json",
        finalReductionArtifactSha256: hash("f"),
        executionBlocked: true,
        blockedStage: "final_report",
        blockedReasonCode: "report_xlsx_not_recoverable",
        nextRequiredAction: "start_new_campaign",
      }),
    );
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    expect(await store.prepare()).toBe(true);
    store.applyStatus(selectionComplete);

    expect(
      await store.openFinalTest({
        disclosureAccepted: true,
        winnerLockSuffix: "dddddddd",
      }),
    ).toBe(false);
    expect(store.status?.blockedStage).toBe("final_report");
    expect(store.status?.finalReductionArtifactPath).toBe(
      "C:/temporal/final.json",
    );
    expect(store.canOpenFinalTest).toBe(false);
    expect(store.canForgetCurrentPreparation).toBe(false);
    expect(store.notice).toContain("final scientific reduction");
    expect(store.notice).toContain(
      "report label, source identity, or validation plan",
    );
  });

  it("retains only selection evidence when receipt poison blocks before the final reduction", async () => {
    const selectionComplete = status({
      state: "selection_complete",
      completedSelectionShardCount: 2,
      winnerCandidateId: "candidate-4",
      winnerLockSha256: hash("d"),
      preFinalReductionArtifactPath: "C:/temporal/pre_final.json",
      preFinalReductionArtifactSha256: hash("e"),
      selectionReportXlsxPath: "C:/temporal/selection.xlsx",
      nextRequiredAction: "confirm_and_open_final_test",
    });
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(status());
    vi.mocked(openVcetTemporalCampaignFinalTest).mockResolvedValueOnce(
      status({
        ...selectionComplete,
        completedFinalTestShardCount: 1,
        executionBlocked: true,
        blockedStage: "final_report",
        blockedReasonCode: "report_receipt_invalid_or_foreign",
        nextRequiredAction: "start_new_campaign",
      }),
    );
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    expect(await store.prepare()).toBe(true);
    store.applyStatus(selectionComplete);

    expect(
      await store.openFinalTest({
        disclosureAccepted: true,
        winnerLockSuffix: "dddddddd",
      }),
    ).toBe(false);
    expect(store.status?.blockedStage).toBe("final_report");
    expect(store.status?.preFinalReductionArtifactPath).toBe(
      "C:/temporal/pre_final.json",
    );
    expect(store.status?.finalReductionArtifactPath).toBeNull();
    expect(store.canOpenFinalTest).toBe(false);
    expect(store.canForgetCurrentPreparation).toBe(false);
    expect(store.notice).toContain(
      "before a final scientific reduction was retained",
    );
    expect(store.notice).toContain(
      "locked selection evidence remains available",
    );
    expect(store.notice).not.toContain(
      "sealed final scientific reduction remains retained",
    );
  });

  it("fails final access closed until both confirmations match", async () => {
    const selectionComplete = status({
      state: "selection_complete",
      completedSelectionShardCount: 2,
      winnerCandidateId: "candidate-4",
      winnerLockSha256: hash("d"),
      winnerRunnerUpMargin: 0.25,
      preFinalReductionArtifactPath: "C:/temporal/pre_final.json",
      preFinalReductionArtifactSha256: hash("e"),
      selectionReportXlsxPath: "C:/temporal/selection.xlsx",
      nextRequiredAction: "confirm_and_open_final_test",
    });
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValue(selectionComplete);
    vi.mocked(openVcetTemporalCampaignFinalTest).mockResolvedValue(
      status({
        ...selectionComplete,
        state: "final_test_complete",
        completedFinalTestShardCount: 1,
        finalReductionArtifactPath: "C:/temporal/final.json",
        finalReductionArtifactSha256: hash("f"),
        finalReportXlsxPath: "C:/temporal/final.xlsx",
        nextRequiredAction: "none",
      }),
    );
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    await store.prepare();

    expect(
      await store.openFinalTest({
        disclosureAccepted: true,
        winnerLockSuffix: "wrong",
      }),
    ).toBe(false);
    expect(openVcetTemporalCampaignFinalTest).not.toHaveBeenCalled();

    expect(
      await store.openFinalTest({
        disclosureAccepted: false,
        winnerLockSuffix: "dddddddd",
      }),
    ).toBe(false);
    expect(openVcetTemporalCampaignFinalTest).not.toHaveBeenCalled();

    expect(
      await store.openFinalTest({
        disclosureAccepted: true,
        winnerLockSuffix: "dddddddd",
      }),
    ).toBe(true);
    expect(openVcetTemporalCampaignFinalTest).toHaveBeenCalledTimes(1);
  });

  it("preserves and refreshes the active status after selection and final invocation failures", async () => {
    const prepared = status();
    const selectionComplete = status({
      state: "selection_complete",
      completedSelectionShardCount: 2,
      winnerCandidateId: "candidate-4",
      winnerLockSha256: hash("d"),
      preFinalReductionArtifactPath: "C:/temporal/pre_final.json",
      preFinalReductionArtifactSha256: hash("e"),
      selectionReportXlsxPath: "C:/temporal/selection.xlsx",
      nextRequiredAction: "confirm_and_open_final_test",
    });
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(prepared);
    vi.mocked(runVcetTemporalCampaignSelection).mockRejectedValueOnce(
      new Error("selection invoke failed"),
    );
    vi.mocked(getVcetTemporalCampaignStatus)
      .mockResolvedValueOnce(prepared)
      .mockResolvedValueOnce(selectionComplete);
    vi.mocked(openVcetTemporalCampaignFinalTest).mockRejectedValueOnce(
      new Error("final invoke failed"),
    );

    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    expect(await store.prepare()).toBe(true);

    expect(await store.runSelection()).toBe(false);
    expect(store.status).toEqual(prepared);
    expect(store.error).toContain("Native status was refreshed");
    expect(store.canRunSelection).toBe(true);

    store.applyStatus(selectionComplete);
    expect(
      await store.openFinalTest({
        disclosureAccepted: true,
        winnerLockSuffix: "dddddddd",
      }),
    ).toBe(false);
    expect(store.status).toEqual(selectionComplete);
    expect(store.error).toContain("Native status was refreshed");
    expect(store.canOpenFinalTest).toBe(true);
  });

  it("retains the last verified status when mutation and recovery refresh both fail", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(status());
    vi.mocked(runVcetTemporalCampaignSelection).mockRejectedValueOnce(
      new Error("selection invoke failed"),
    );
    vi.mocked(getVcetTemporalCampaignStatus).mockRejectedValueOnce(
      new Error("status refresh failed"),
    );
    const store = useVcetTemporalCampaignStore();
    store.hydrate();
    admitSource(store);
    await store.prepare();
    const previousStatus = store.status;

    expect(await store.runSelection()).toBe(false);
    expect(store.status).toBe(previousStatus);
    expect(store.hasCurrentPreparation).toBe(true);
    expect(store.error).toContain("last verified campaign state was retained");
  });
});
