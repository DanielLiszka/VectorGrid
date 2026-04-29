import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useAnalysisStore } from "../stores/analysisStore";
import { useDataStore } from "../stores/dataStore";
import { useHistoryStore } from "../stores/historyStore";
import { useValidationStore } from "../stores/validationStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import {
  latestStageCompletionForStage,
  stageCompletionIsComplete,
} from "../utils/core/stageCompletion";

export type UiReadinessStage =
  | "no_data"
  | "data_loaded_no_run"
  | "results_available_no_selection"
  | "ready";
export type UiReadinessSurface =
  | "overview"
  | "top"
  | "pareto"
  | "validation"
  | "history"
  | "meta";

export interface UiReadinessDecision {
  stage: UiReadinessStage;
  title: string;
  body: string;
  primaryActionLabel: string;
  blockedReason: string;
}

const PLACEHOLDER_VALUES = new Set(["", "-", "—"]);

function hasMeaningfulText(value: unknown): boolean {
  const next = String(value || "").trim();
  return !PLACEHOLDER_VALUES.has(next);
}

function hasArrayValues<T>(
  value: T[] | readonly T[] | undefined | null,
): boolean {
  return Array.isArray(value) && value.length > 0;
}

function surfaceLabel(surface: UiReadinessSurface): string {
  if (surface === "top") return "Top results";
  if (surface === "pareto") return "Pareto";
  if (surface === "validation") return "Validation";
  if (surface === "history") return "Library";
  if (surface === "meta") return "Aggregate";
  return "Overview";
}

export function describeUiReadinessStage(
  surface: UiReadinessSurface,
  stage: UiReadinessStage,
): UiReadinessDecision {
  const label = surfaceLabel(surface);
  if (stage === "ready") {
    return {
      stage,
      title: `${label} ready.`,
      body: `The ${label.toLowerCase()} surface can use the current workflow state.`,
      primaryActionLabel:
        surface === "validation" ? "Review validation" : "Review results",
      blockedReason: "",
    };
  }
  if (stage === "no_data") {
    return {
      stage,
      title:
        surface === "pareto"
          ? "Load data before opening the Pareto view."
          : `Load data before opening ${label}.`,
      body:
        surface === "pareto"
          ? "The Pareto frontier appears after a dataset is loaded and scored."
          : `${label} becomes useful after a dataset is loaded into the research workflow.`,
      primaryActionLabel: "Load data",
      blockedReason: "A dataset is required first.",
    };
  }
  if (stage === "data_loaded_no_run") {
    return {
      stage,
      title:
        surface === "validation"
          ? "Run optimization before validation."
          : surface === "pareto"
            ? "Run an optimization to generate Pareto points."
            : `Run an optimization to generate ${label}.`,
      body:
        surface === "validation"
          ? "Validation needs an optimization shortlist or selected candidate before it can run."
          : surface === "pareto"
            ? "This view compares tradeoffs between competing candidates once optimization results are available."
            : `${label} appears after the loaded dataset has scored optimization results.`,
      primaryActionLabel: "Run optimization",
      blockedReason: "Optimization results are required first.",
    };
  }
  return {
    stage,
    title:
      surface === "history"
        ? "Select a saved run."
        : "Select or review a candidate.",
    body:
      surface === "history"
        ? "Choose a saved run from the library to enable restore, compare, and report actions."
        : `${label} needs a selected result context before this action is available.`,
    primaryActionLabel:
      surface === "history" ? "Select run" : "Review Top results",
    blockedReason:
      surface === "history"
        ? "No saved run is selected."
        : "A candidate review context is required first.",
  };
}

export function useUiReadiness() {
  const analysisStore = useAnalysisStore();
  const dataStore = useDataStore();
  const historyStore = useHistoryStore();
  const validationStore = useValidationStore();
  const workflowArtifactsStore = useWorkflowArtifactsStore();

  const {
    bestMeta,
    bestStats,
    bestTitle,
    metaCompareSelected,
    metaOutputPreview,
    metaTableRows,
    topRows,
  } = storeToRefs(analysisStore);
  const { dataId, datasetLabel } = storeToRefs(dataStore);
  const { savedRunCount, savedSelectedValue } = storeToRefs(historyStore);
  const { tableRowCount, tableVisible, validationOutputPreview } =
    storeToRefs(validationStore);
  const { currentRun, currentValidation } = storeToRefs(workflowArtifactsStore);
  const optimizationCompletion = computed(() =>
    latestStageCompletionForStage(
      currentRun.value?.stageCompletions,
      "optimization",
    ),
  );
  const validationCompletion = computed(() =>
    latestStageCompletionForStage(
      currentRun.value?.stageCompletions,
      "validation",
    ),
  );

  const datasetReady = computed(() => {
    const id = String(dataId.value || "").trim();
    const label = String(datasetLabel.value || "").trim();
    return hasMeaningfulText(id) || hasMeaningfulText(label);
  });

  const hasOptimizationResults = computed(() => {
    if (stageCompletionIsComplete(optimizationCompletion.value)) return true;
    if (currentRun.value?.res) return true;
    if (Number(topRows.value || 0) > 0) return true;
    if (hasArrayValues(bestStats.value)) return true;
    return (
      hasMeaningfulText(bestTitle.value) || hasMeaningfulText(bestMeta.value)
    );
  });

  const hasValidationResults = computed(() => {
    if (stageCompletionIsComplete(validationCompletion.value)) return true;
    if (currentValidation.value?.res) return true;
    if (Boolean(tableVisible.value)) return true;
    if (Number(tableRowCount.value || 0) > 0) return true;
    return hasMeaningfulText(validationOutputPreview.value);
  });

  const hasSavedRuns = computed(() => Number(savedRunCount.value || 0) > 0);
  const hasSavedRunSelection = computed(() =>
    hasMeaningfulText(savedSelectedValue.value),
  );
  const hasMetaArtifacts = computed(
    () =>
      hasArrayValues(metaTableRows.value) ||
      hasMeaningfulText(metaOutputPreview.value),
  );
  const hasMetaComparisonSelection = computed(() =>
    hasMeaningfulText(metaCompareSelected.value),
  );

  const overviewStage = computed<UiReadinessStage>(() => {
    if (!datasetReady.value) return "no_data";
    if (!hasOptimizationResults.value) return "data_loaded_no_run";
    return "ready";
  });

  const topStage = computed<UiReadinessStage>(() => {
    if (!datasetReady.value) return "no_data";
    if (!hasOptimizationResults.value) return "data_loaded_no_run";
    return "ready";
  });

  const paretoStage = computed<UiReadinessStage>(() => {
    if (!datasetReady.value) return "no_data";
    if (!hasOptimizationResults.value) return "data_loaded_no_run";
    return "ready";
  });

  const validationStage = computed<UiReadinessStage>(() => {
    if (!datasetReady.value) return "no_data";
    if (!hasOptimizationResults.value) return "data_loaded_no_run";
    if (!hasValidationResults.value) return "results_available_no_selection";
    return "ready";
  });

  const historyStage = computed<UiReadinessStage>(() => {
    if (!hasSavedRuns.value) return "no_data";
    if (!hasSavedRunSelection.value) return "results_available_no_selection";
    return "ready";
  });

  const metaStage = computed<UiReadinessStage>(() => {
    if (!hasSavedRuns.value) return "no_data";
    if (!hasMetaArtifacts.value) return "data_loaded_no_run";
    return "ready";
  });

  const metaCompareStage = computed<UiReadinessStage>(() => {
    if (metaStage.value !== "ready") return metaStage.value;
    if (!hasMetaComparisonSelection.value)
      return "results_available_no_selection";
    return "ready";
  });

  return {
    datasetReady,
    hasOptimizationResults,
    hasValidationResults,
    hasSavedRuns,
    hasSavedRunSelection,
    hasMetaArtifacts,
    hasMetaComparisonSelection,
    overviewStage,
    topStage,
    paretoStage,
    validationStage,
    historyStage,
    metaStage,
    metaCompareStage,
    describeStage: describeUiReadinessStage,
  };
}
