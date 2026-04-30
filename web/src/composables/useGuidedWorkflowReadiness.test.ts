import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAnalysisStore } from "../stores/analysisStore";
import { useDataStore } from "../stores/dataStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useValidationStore } from "../stores/validationStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { useGuidedWorkflowReadiness } from "./useGuidedWorkflowReadiness";

describe("useGuidedWorkflowReadiness", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("starts with data and optimization blocked", () => {
    const readiness = useGuidedWorkflowReadiness();

    expect(readiness.datasetReady.value).toBe(false);
    expect(readiness.optimizationReady.value).toBe(false);
    expect(readiness.validationReady.value).toBe(false);
    expect(readiness.guidedChecklist.value.map((item) => item.tone)).toEqual([
      "blocked",
      "done",
      "blocked",
      "blocked",
    ]);
  });

  it("separates Simple optimization support from unsupported strategy names", () => {
    const optimizeStore = useOptimizeStore();
    const readiness = useGuidedWorkflowReadiness();

    optimizeStore.strategyKind = "MlSignal";
    expect(readiness.simpleOptimizationSupported.value).toBe(false);
    expect(readiness.guidedChecklist.value[1]).toMatchObject({
      label: "Strategy selected",
      tone: "warning",
    });

    optimizeStore.strategyKind = "CompositeCpu";
    expect(readiness.simpleOptimizationSupported.value).toBe(true);
    expect(readiness.guidedChecklist.value[1]).toMatchObject({
      detail: "Composite signal · Sharpe · Auto",
      tone: "done",
    });
  });

  it("marks optimization ready from existing result state", () => {
    const dataStore = useDataStore();
    const analysisStore = useAnalysisStore();
    const readiness = useGuidedWorkflowReadiness();

    dataStore.$patch({ dataId: "dataset-1", datasetLabel: "AAPL sample" });
    analysisStore.$patch({
      topRows: 2,
      selectedTopTitle: "Rank 1",
      selectedTopSummary: "Return 10%",
    });

    expect(readiness.datasetReady.value).toBe(true);
    expect(readiness.optimizationReady.value).toBe(true);
    expect(readiness.candidateReady.value).toBe(true);
    expect(readiness.guidedChecklist.value[2]).toMatchObject({
      label: "Optimization run",
      complete: true,
      tone: "done",
    });
  });

  it("maps validation warnings into a review tone", () => {
    const dataStore = useDataStore();
    const analysisStore = useAnalysisStore();
    const validationStore = useValidationStore();
    const readiness = useGuidedWorkflowReadiness();

    dataStore.$patch({ dataId: "dataset-1", datasetLabel: "AAPL sample" });
    analysisStore.$patch({ topRows: 1, bestTitle: "Rank 1" });
    validationStore.$patch({
      tableVisible: true,
      tableRowCount: 3,
      validationWarningSummary: "Fragile candidate warning.",
    });

    expect(readiness.validationReady.value).toBe(true);
    expect(readiness.validationVerdictTone.value).toBe("warning");
    expect(readiness.validationVerdictTitle.value).toBe("Validation needs review.");
  });

  it("uses structured validation stage completions when available", () => {
    const workflowArtifactsStore = useWorkflowArtifactsStore();
    const readiness = useGuidedWorkflowReadiness();

    workflowArtifactsStore.applyVueRuntimePayload({
      currentRun: {
        stageCompletions: [
          {
            schemaVersion: 1,
            stage: "optimization",
            status: "success",
            verdict: "search_complete_requires_validation",
            completedAt: "2026-04-29T12:00:00.000Z",
          },
          {
            schemaVersion: 1,
            stage: "validation",
            status: "success",
            verdict: "approved_with_warnings",
            completedAt: "2026-04-29T12:01:00.000Z",
            recommendation: "Approved, but review the warning.",
            warnings: ["Low trade count."],
          },
        ],
      },
    });

    expect(readiness.optimizationReady.value).toBe(true);
    expect(readiness.validationReady.value).toBe(true);
    expect(readiness.validationVerdictTone.value).toBe("warning");
    expect(readiness.validationVerdictTitle.value).toBe(
      "Validation approved with warnings.",
    );
    expect(readiness.validationVerdictBody.value).toContain("Approved");
  });
});
