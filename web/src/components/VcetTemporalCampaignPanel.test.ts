import { createPinia, setActivePinia } from "pinia";
import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  getVcetTemporalCampaignStatus,
  openVcetTemporalCampaignFinalTest,
  prepareVcetTemporalCampaign,
  runVcetTemporalCampaignSelection,
  type VcetTemporalCampaignStatus,
  vcetTemporalNativePathKey,
} from "../services/vcetTemporalCampaignService";
import { useVcetTemporalCampaignStore } from "../stores/vcetTemporalCampaignStore";
import VcetTemporalCampaignPanel from "./VcetTemporalCampaignPanel.vue";

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

function temporalStatus(
  overrides: Partial<VcetTemporalCampaignStatus> = {},
): VcetTemporalCampaignStatus {
  return {
    schemaVersion: 1,
    campaignId: "temporal-ui-1",
    state: "prepared",
    preparationArtifactPath: "C:/temporal/preparation.json",
    preparationArtifactSha256: hash("a"),
    sourceCampaignSummaryPath: "C:/picked/summary.json",
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

async function mountPanel(sourceEligible = true) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const wrapper = mount(VcetTemporalCampaignPanel, {
    props: {
      sourceCampaignSummaryPath: sourceEligible ? "C:/picked/summary.json" : "",
      sourceEligible,
      sourceEligibilityIdentity: sourceEligible
        ? sourceIdentity("C:/picked/summary.json")
        : null,
      sourceEligibilityMessage: sourceEligible
        ? "Structurally eligible terminal VCET source; sealing verification is pending."
        : "No compatible latest campaign.",
    },
    global: { plugins: [pinia] },
  });
  await flushPromises();
  return { wrapper, store: useVcetTemporalCampaignStore(pinia) };
}

describe("VcetTemporalCampaignPanel", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.mocked(prepareVcetTemporalCampaign).mockReset();
    vi.mocked(getVcetTemporalCampaignStatus).mockReset();
    vi.mocked(runVcetTemporalCampaignSelection).mockReset();
    vi.mocked(openVcetTemporalCampaignFinalTest).mockReset();
  });

  it("keeps preparation disabled without the exact latest eligible campaign and states the bounded contract", async () => {
    const { wrapper } = await mountPanel(false);

    expect(wrapper.text()).toContain("at most 64 source datasets");
    expect(wrapper.text()).toContain(
      "one resident dataset per execution shard",
    );
    expect(wrapper.text()).toContain("not the 10,000-stock");
    expect(
      wrapper.findAll(".stageStateLabel").map((label) => label.text()),
    ).toEqual(["ready", "pending", "pending"]);
    expect(
      wrapper.get("[data-source-eligible]").attributes("data-source-eligible"),
    ).toBe("false");
    expect(
      (
        wrapper.get('[data-testid="vcet-temporal-prepare"]')
          .element as HTMLButtonElement
      ).disabled,
    ).toBe(true);

    wrapper.unmount();
  });

  it("prepares from the host-provided source path and renders compact stage progress", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValue(
      temporalStatus({ completedSelectionShardCount: 1 }),
    );
    const { wrapper } = await mountPanel();

    await wrapper.get('[data-testid="vcet-temporal-prepare"]').trigger("click");
    await flushPromises();

    expect(prepareVcetTemporalCampaign).toHaveBeenCalledWith({
      sourceCampaignSummaryPath: "C:/picked/summary.json",
      planDraft: expect.objectContaining({
        foldMode: "anchored",
        positionBoundaryPolicy: "reset_flat_discard_open",
      }),
    });
    expect(wrapper.text()).toContain("1 / 2 selection shards");
    expect(wrapper.text()).toContain("8 candidates");

    wrapper.unmount();
  });

  it("keeps the active plan frozen, flags next-campaign drift, and gates replacement explicitly", async () => {
    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(
      temporalStatus(),
    );
    const { wrapper } = await mountPanel();
    await wrapper.get('[data-testid="vcet-temporal-prepare"]').trigger("click");
    await flushPromises();

    expect(
      wrapper.get('[data-testid="vcet-temporal-active-plan"]').text(),
    ).toContain("2048 train");
    expect(wrapper.text()).toContain("Active campaign already prepared");

    await wrapper.get("#vcetTemporal_trainBars").setValue("4096");
    await wrapper.setProps({
      sourceCampaignSummaryPath: "C:/picked/new-summary.json",
      sourceEligible: true,
      sourceEligibilityIdentity: sourceIdentity(
        "C:/picked/new-summary.json",
        "2",
      ),
      sourceEligibilityMessage: "Eligible replacement source.",
    });
    await nextTick();

    const drift = wrapper.get('[data-testid="vcet-temporal-drift-warning"]');
    expect(drift.text()).toContain("different native path or");
    expect(drift.text()).toContain("research draft differs");
    expect(
      wrapper.get('[data-testid="vcet-temporal-active-plan"]').text(),
    ).toContain("2048 train");
    const prepareButton = wrapper.get('[data-testid="vcet-temporal-prepare"]');
    expect((prepareButton.element as HTMLButtonElement).disabled).toBe(true);

    await wrapper
      .get('[data-testid="vcet-temporal-replacement-confirmation"]')
      .setValue(true);
    expect((prepareButton.element as HTMLButtonElement).disabled).toBe(false);

    vi.mocked(prepareVcetTemporalCampaign).mockResolvedValueOnce(
      temporalStatus({
        campaignId: "temporal-ui-2",
        preparationArtifactPath: "C:/temporal/preparation-2.json",
        preparationArtifactSha256: hash("e"),
        sourceCampaignSummaryPath: "C:/picked/new-summary.json",
        sourceCampaignSummarySha256: hash("f"),
      }),
    );
    await prepareButton.trigger("click");
    await flushPromises();

    expect(prepareVcetTemporalCampaign).toHaveBeenLastCalledWith({
      sourceCampaignSummaryPath: "C:/picked/new-summary.json",
      planDraft: expect.objectContaining({ trainBars: "4096" }),
    });
    expect(
      wrapper.get('[data-testid="vcet-temporal-active-plan"]').text(),
    ).toContain("4096 train");
    expect(
      wrapper.find('[data-testid="vcet-temporal-drift-warning"]').exists(),
    ).toBe(false);

    wrapper.unmount();
  });

  it("recovers from an unverifiable saved preparation only after confirmed local-pointer forgetting", async () => {
    const { wrapper, store } = await mountPanel();
    store.applyStatus(temporalStatus());
    store.failClosed(new Error("preparation artifact was deleted"));
    await nextTick();

    expect(wrapper.text()).toContain("Saved preparation needs attention");
    expect(wrapper.text()).toContain("evidence files are never deleted");
    const forgetButton = wrapper.get(
      '[data-testid="vcet-temporal-forget-stale"]',
    );
    expect((forgetButton.element as HTMLButtonElement).disabled).toBe(true);

    await wrapper
      .get('[data-testid="vcet-temporal-forget-confirmation"]')
      .setValue(true);
    expect((forgetButton.element as HTMLButtonElement).disabled).toBe(false);
    await forgetButton.trigger("click");
    await nextTick();

    expect(store.hasCurrentPreparation).toBe(false);
    expect(wrapper.text()).toContain("Evidence files were not deleted");
    expect(
      wrapper.find('[data-testid="vcet-temporal-forget-stale"]').exists(),
    ).toBe(false);

    wrapper.unmount();
  });

  it("renders terminal durable blocking accessibly and removes resume/final actions", async () => {
    const { wrapper, store } = await mountPanel();
    store.applyStatus(
      temporalStatus({
        completedSelectionShardCount: 2,
        executionBlocked: true,
        blockedStage: "pre_final_reduction",
        blockedReasonCode: "durable_stage_failed_interrupted_or_invalid",
        nextRequiredAction: "start_new_campaign",
      }),
      {
        preparedPlanDraftSnapshot: { ...store.planDraft },
        sourceSummaryContentFingerprint: sourceIdentity(
          "C:/picked/summary.json",
        ).summaryContentFingerprint,
      },
    );
    await nextTick();

    const blocked = wrapper.get(
      '[data-testid="vcet-temporal-execution-blocked"]',
    );
    expect(blocked.attributes("role")).toBe("alert");
    expect(blocked.text()).toContain("This campaign cannot resume");
    expect(blocked.text()).toContain("pre-final reduction");
    expect(blocked.text()).toContain("evidence files are never deleted");
    expect(blocked.text()).toContain("cannot be forgotten");
    expect(
      wrapper.find('[data-testid="vcet-temporal-selection"]').exists(),
    ).toBe(false);
    expect(
      wrapper.find('[data-testid="vcet-temporal-final-open"]').exists(),
    ).toBe(false);
    expect(
      wrapper.findAll(".stageStateLabel").map((label) => label.text()),
    ).toEqual(["complete", "blocked", "sealed"]);
    expect(
      wrapper
        .find('[data-testid="vcet-temporal-replacement-confirmation"]')
        .exists(),
    ).toBe(false);
    expect(
      wrapper.find('[data-testid="vcet-temporal-forget-stale"]').exists(),
    ).toBe(false);
    expect(
      (
        wrapper.get('[data-testid="vcet-temporal-prepare"]')
          .element as HTMLButtonElement
      ).disabled,
    ).toBe(true);

    await wrapper.get("#vcetTemporal_trainBars").setValue("4096");
    await nextTick();
    expect(
      wrapper
        .find('[data-testid="vcet-temporal-replacement-confirmation"]')
        .exists(),
    ).toBe(true);
    expect(store.hasCurrentPreparation).toBe(true);
    expect(wrapper.text()).toContain(
      "editable research draft differs from the active sealed plan",
    );

    wrapper.unmount();
  });

  it("keeps retained final evidence complete when a late pre-final reduction record is blocked", async () => {
    const { wrapper, store } = await mountPanel();
    store.applyStatus(
      temporalStatus({
        state: "final_test_complete",
        completedSelectionShardCount: 2,
        completedFinalTestShardCount: 1,
        winnerCandidateId: "candidate-4",
        winnerLockSha256: hash("d"),
        winnerRunnerUpMargin: 0.25,
        preFinalReductionArtifactPath: "C:/temporal/pre_final.json",
        preFinalReductionArtifactSha256: hash("e"),
        selectionReportXlsxPath: "C:/temporal/selection.xlsx",
        finalReductionArtifactPath: "C:/temporal/final.json",
        finalReductionArtifactSha256: hash("f"),
        finalReportXlsxPath: "C:/temporal/final.xlsx",
        executionBlocked: true,
        blockedStage: "pre_final_reduction",
        blockedReasonCode: "durable_stage_failed_interrupted_or_invalid",
        nextRequiredAction: "start_new_campaign",
      }),
    );
    await nextTick();

    expect(
      wrapper
        .get('[data-testid="vcet-temporal-stage-selection"]')
        .attributes("data-stage-state"),
    ).toBe("blocked");
    expect(
      wrapper
        .get('[data-testid="vcet-temporal-stage-final"]')
        .attributes("data-stage-state"),
    ).toBe("complete");
    expect(wrapper.get(".finalComplete").text()).toContain(
      "Retained final evidence is complete",
    );
    expect(wrapper.get(".finalComplete").text()).toContain(
      "late-stage artifacts remain available",
    );
    expect(
      wrapper.get('[data-testid="vcet-temporal-execution-blocked"]').text(),
    ).toContain("pre-final reduction");
    expect(
      wrapper.find('[data-testid="vcet-temporal-selection"]').exists(),
    ).toBe(false);
    expect(
      wrapper.find('[data-testid="vcet-temporal-final-open"]').exists(),
    ).toBe(false);
    expect(
      wrapper
        .find(
          '[data-qa-action="aggregate.batch.vcetTemporal.revealFinalReduction"]',
        )
        .exists(),
    ).toBe(true);
    expect(
      wrapper
        .find('[data-qa-action="aggregate.batch.vcetTemporal.openFinalXlsx"]')
        .exists(),
    ).toBe(true);

    wrapper.unmount();
  });

  it("preserves late final evidence and distinct-replacement guidance under a selection-report block", async () => {
    const { wrapper, store } = await mountPanel();
    store.applyStatus(
      temporalStatus({
        state: "final_test_complete",
        completedSelectionShardCount: 2,
        completedFinalTestShardCount: 1,
        winnerCandidateId: "candidate-4",
        winnerLockSha256: hash("d"),
        winnerRunnerUpMargin: 0.25,
        preFinalReductionArtifactPath: "C:/temporal/pre_final.json",
        preFinalReductionArtifactSha256: hash("e"),
        selectionReportXlsxPath: null,
        finalReductionArtifactPath: "C:/temporal/final.json",
        finalReductionArtifactSha256: hash("f"),
        finalReportXlsxPath: "C:/temporal/final.xlsx",
        executionBlocked: true,
        blockedStage: "selection_report",
        blockedReasonCode: "report_receipt_invalid_or_foreign",
        nextRequiredAction: "start_new_campaign",
      }),
      {
        preparedPlanDraftSnapshot: { ...store.planDraft },
        sourceSummaryContentFingerprint: sourceIdentity(
          "C:/picked/summary.json",
        ).summaryContentFingerprint,
      },
    );
    await nextTick();

    expect(
      wrapper
        .get('[data-testid="vcet-temporal-stage-selection"]')
        .attributes("data-stage-state"),
    ).toBe("blocked");
    expect(
      wrapper
        .get('[data-testid="vcet-temporal-stage-final"]')
        .attributes("data-stage-state"),
    ).toBe("complete");
    expect(
      wrapper.get('[data-testid="vcet-temporal-execution-blocked"]').text(),
    ).toContain("selection report");
    expect(wrapper.text()).toContain(
      "retained final scientific reduction remain complete",
    );
    const guidance = wrapper.get(
      '[data-testid="vcet-temporal-report-block-guidance"]',
    );
    expect(guidance.text()).toContain(
      "change the report label, structurally eligible source identity, or validation plan",
    );
    expect(guidance.text()).toContain("cannot be forgotten");
    expect(
      wrapper
        .find(
          '[data-qa-action="aggregate.batch.vcetTemporal.openSelectionXlsx"]',
        )
        .exists(),
    ).toBe(false);
    expect(
      wrapper
        .find('[data-qa-action="aggregate.batch.vcetTemporal.openFinalXlsx"]')
        .exists(),
    ).toBe(true);
    expect(
      wrapper.find('[data-testid="vcet-temporal-forget-stale"]').exists(),
    ).toBe(false);
    expect(
      wrapper.find('[data-testid="vcet-temporal-selection"]').exists(),
    ).toBe(false);

    await wrapper
      .get('[data-testid="vcet-temporal-report-label"]')
      .setValue("Distinct replacement report");
    await nextTick();
    expect(
      wrapper
        .find('[data-testid="vcet-temporal-replacement-confirmation"]')
        .exists(),
    ).toBe(true);

    wrapper.unmount();
  });

  it("shows a retained final scientific reduction when only the final report is blocked", async () => {
    const { wrapper, store } = await mountPanel();
    store.applyStatus(
      temporalStatus({
        state: "selection_complete",
        completedSelectionShardCount: 2,
        completedFinalTestShardCount: 1,
        winnerCandidateId: "candidate-4",
        winnerLockSha256: hash("d"),
        winnerRunnerUpMargin: 0.25,
        preFinalReductionArtifactPath: "C:/temporal/pre_final.json",
        preFinalReductionArtifactSha256: hash("e"),
        selectionReportXlsxPath: "C:/temporal/selection.xlsx",
        finalReductionArtifactPath: "C:/temporal/final.json",
        finalReductionArtifactSha256: hash("f"),
        finalReportXlsxPath: null,
        executionBlocked: true,
        blockedStage: "final_report",
        blockedReasonCode: "report_xlsx_not_recoverable",
        nextRequiredAction: "start_new_campaign",
      }),
      {
        preparedPlanDraftSnapshot: { ...store.planDraft },
        sourceSummaryContentFingerprint: sourceIdentity(
          "C:/picked/summary.json",
        ).summaryContentFingerprint,
      },
    );
    await nextTick();

    expect(
      wrapper
        .get('[data-testid="vcet-temporal-stage-selection"]')
        .attributes("data-stage-state"),
    ).toBe("complete");
    expect(
      wrapper
        .get('[data-testid="vcet-temporal-stage-final"]')
        .attributes("data-stage-state"),
    ).toBe("blocked");
    const blocked = wrapper.get(
      '[data-testid="vcet-temporal-execution-blocked"]',
    );
    expect(blocked.text()).toContain("final report");
    expect(blocked.text()).toContain("exact bytes and SHA-256");
    expect(wrapper.get(".finalComplete").text()).toContain(
      "Retained final scientific reduction is complete",
    );
    expect(
      wrapper
        .find(
          '[data-qa-action="aggregate.batch.vcetTemporal.openSelectionXlsx"]',
        )
        .exists(),
    ).toBe(true);
    expect(
      wrapper
        .find(
          '[data-qa-action="aggregate.batch.vcetTemporal.revealFinalReduction"]',
        )
        .exists(),
    ).toBe(true);
    expect(
      wrapper
        .find('[data-qa-action="aggregate.batch.vcetTemporal.openFinalXlsx"]')
        .exists(),
    ).toBe(false);
    expect(
      wrapper.find('[data-testid="vcet-temporal-final-open"]').exists(),
    ).toBe(false);
    expect(
      wrapper.find('[data-testid="vcet-temporal-forget-stale"]').exists(),
    ).toBe(false);

    wrapper.unmount();
  });

  it("does not claim retained science when selection-report poison precedes the reduction", async () => {
    const { wrapper, store } = await mountPanel();
    store.applyStatus(
      temporalStatus({
        completedSelectionShardCount: 1,
        executionBlocked: true,
        blockedStage: "selection_report",
        blockedReasonCode: "report_receipt_invalid_or_foreign",
        nextRequiredAction: "start_new_campaign",
      }),
      {
        preparedPlanDraftSnapshot: { ...store.planDraft },
        sourceSummaryContentFingerprint: sourceIdentity(
          "C:/picked/summary.json",
        ).summaryContentFingerprint,
      },
    );
    await nextTick();

    expect(
      wrapper
        .get('[data-testid="vcet-temporal-stage-selection"]')
        .attributes("data-stage-state"),
    ).toBe("blocked");
    expect(
      wrapper
        .get('[data-testid="vcet-temporal-stage-final"]')
        .attributes("data-stage-state"),
    ).toBe("sealed");
    const blocked = wrapper.get(
      '[data-testid="vcet-temporal-execution-blocked"]',
    );
    expect(blocked.text()).toContain("selection report");
    expect(blocked.text()).toContain(
      "before a locked winner or pre-final scientific reduction was retained",
    );
    expect(blocked.text()).toContain(
      "No completed scientific selection evidence is claimed",
    );
    expect(blocked.text()).not.toContain("retained scientific artifact");
    expect(blocked.text()).not.toContain(
      "pre-final scientific reduction remain complete",
    );
    expect(wrapper.find(".finalComplete").exists()).toBe(false);
    expect(
      wrapper
        .find(
          '[data-qa-action="aggregate.batch.vcetTemporal.revealSelectionReduction"]',
        )
        .exists(),
    ).toBe(false);
    expect(
      wrapper.find('[data-testid="vcet-temporal-forget-stale"]').exists(),
    ).toBe(false);
    expect(
      wrapper
        .find('[data-testid="vcet-temporal-replacement-confirmation"]')
        .exists(),
    ).toBe(false);
    expect(
      (
        wrapper.get('[data-testid="vcet-temporal-prepare"]')
          .element as HTMLButtonElement
      ).disabled,
    ).toBe(true);

    wrapper.unmount();
  });

  it("claims only retained selection science when final-report poison precedes the final reduction", async () => {
    const { wrapper, store } = await mountPanel();
    store.applyStatus(
      temporalStatus({
        state: "selection_complete",
        completedSelectionShardCount: 2,
        completedFinalTestShardCount: 1,
        winnerCandidateId: "candidate-4",
        winnerLockSha256: hash("d"),
        winnerRunnerUpMargin: 0.25,
        preFinalReductionArtifactPath: "C:/temporal/pre_final.json",
        preFinalReductionArtifactSha256: hash("e"),
        selectionReportXlsxPath: "C:/temporal/selection.xlsx",
        executionBlocked: true,
        blockedStage: "final_report",
        blockedReasonCode: "report_receipt_invalid_or_foreign",
        nextRequiredAction: "start_new_campaign",
      }),
      {
        preparedPlanDraftSnapshot: { ...store.planDraft },
        sourceSummaryContentFingerprint: sourceIdentity(
          "C:/picked/summary.json",
        ).summaryContentFingerprint,
      },
    );
    await nextTick();

    expect(
      wrapper
        .get('[data-testid="vcet-temporal-stage-selection"]')
        .attributes("data-stage-state"),
    ).toBe("complete");
    expect(
      wrapper
        .get('[data-testid="vcet-temporal-stage-final"]')
        .attributes("data-stage-state"),
    ).toBe("blocked");
    const blocked = wrapper.get(
      '[data-testid="vcet-temporal-execution-blocked"]',
    );
    expect(blocked.text()).toContain(
      "locked selection winner and pre-final scientific reduction remain complete",
    );
    expect(blocked.text()).toContain(
      "No final scientific reduction was retained",
    );
    expect(blocked.text()).toContain("no completed final evidence is claimed");
    expect(wrapper.find(".finalComplete").exists()).toBe(false);
    expect(
      wrapper
        .find(
          '[data-qa-action="aggregate.batch.vcetTemporal.revealSelectionReduction"]',
        )
        .exists(),
    ).toBe(true);
    expect(
      wrapper
        .find(
          '[data-qa-action="aggregate.batch.vcetTemporal.openSelectionXlsx"]',
        )
        .exists(),
    ).toBe(true);
    expect(
      wrapper
        .find(
          '[data-qa-action="aggregate.batch.vcetTemporal.revealFinalReduction"]',
        )
        .exists(),
    ).toBe(false);
    expect(
      wrapper.find('[data-testid="vcet-temporal-forget-stale"]').exists(),
    ).toBe(false);
    expect(
      wrapper
        .find('[data-testid="vcet-temporal-replacement-confirmation"]')
        .exists(),
    ).toBe(false);

    wrapper.unmount();
  });

  it("requires both explicit disclosure controls before opening the sealed final test", async () => {
    const selectionComplete = temporalStatus({
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
    vi.mocked(runVcetTemporalCampaignSelection).mockResolvedValue(
      selectionComplete,
    );
    vi.mocked(openVcetTemporalCampaignFinalTest).mockResolvedValue(
      temporalStatus({
        ...selectionComplete,
        state: "final_test_complete",
        completedFinalTestShardCount: 1,
        finalReductionArtifactPath: "C:/temporal/final.json",
        finalReductionArtifactSha256: hash("f"),
        finalReportXlsxPath: "C:/temporal/final.xlsx",
        nextRequiredAction: "none",
      }),
    );
    const { wrapper, store } = await mountPanel();
    store.applyStatus(temporalStatus());
    await nextTick();

    await wrapper
      .get('[data-testid="vcet-temporal-selection"]')
      .trigger("click");
    await flushPromises();

    expect(runVcetTemporalCampaignSelection).toHaveBeenCalledWith({
      preparationArtifactPath: "C:/temporal/preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
    });
    expect(wrapper.text()).toContain(
      "A weak final result remains final evidence",
    );
    expect(wrapper.text()).toContain("Winner / runner-up margin");
    expect(openVcetTemporalCampaignFinalTest).not.toHaveBeenCalled();

    const finalButton = wrapper.get('[data-testid="vcet-temporal-final-open"]');
    await wrapper
      .get('[data-testid="vcet-temporal-final-disclosure"]')
      .setValue(true);
    await wrapper
      .get('[data-testid="vcet-temporal-final-confirmation"]')
      .setValue("wrong");
    expect((finalButton.element as HTMLButtonElement).disabled).toBe(true);

    await wrapper
      .get('[data-testid="vcet-temporal-final-confirmation"]')
      .setValue("dddddddd");
    await nextTick();
    expect((finalButton.element as HTMLButtonElement).disabled).toBe(false);
    await finalButton.trigger("click");
    await flushPromises();

    expect(openVcetTemporalCampaignFinalTest).toHaveBeenCalledWith({
      preparationArtifactPath: "C:/temporal/preparation.json",
      expectedPreparationArtifactSha256: hash("a"),
      expectedWinnerLockSha256: hash("d"),
    });
    expect(wrapper.text()).toContain("Final evidence is complete");

    wrapper.unmount();
  });
});
