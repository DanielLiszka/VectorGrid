import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useAnalysisStore } from "../stores/analysisStore";
import { useDataStore } from "../stores/dataStore";
import { useExecutionStore } from "../stores/executionStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useValidationStore } from "../stores/validationStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import {
  latestStageCompletionForStage,
  stageCompletionIsComplete,
  stageCompletionNeedsReview,
} from "../utils/core/stageCompletion";
import {
  SIMPLE_OPTIMIZATION_STRATEGIES,
  isSimpleOptimizationStrategySupported,
  simpleStrategyLabel,
} from "../utils/simpleWorkflowModel";

export const GUIDED_LIVE_STRATEGIES = SIMPLE_OPTIMIZATION_STRATEGIES;

export type GuidedStatusTone = "blocked" | "ready" | "done" | "warning";

export interface GuidedChecklistItem {
  label: string;
  detail: string;
  complete: boolean;
  tone: GuidedStatusTone;
}

const EMPTY_VALUES = new Set(["", "-", "—"]);

function hasText(value: unknown): boolean {
  const next = String(value ?? "").trim();
  return !EMPTY_VALUES.has(next);
}

function selectedCandidateAvailable(value: unknown): boolean {
  const text = String(value ?? "").trim();
  if (!hasText(text)) return false;
  return text !== "No ranked candidate selected.";
}

function validationTextLooksConcerning(value: unknown): boolean {
  const text = String(value ?? "")
    .trim()
    .toLowerCase();
  if (!text || EMPTY_VALUES.has(text)) return false;
  return /\b(fail|failed|fragile|warning|rejected|blocked|error)\b/.test(text);
}

export function useGuidedWorkflowReadiness() {
  const analysisStore = useAnalysisStore();
  const dataStore = useDataStore();
  const executionStore = useExecutionStore();
  const optimizeStore = useOptimizeStore();
  const validationStore = useValidationStore();
  const workflowArtifactsStore = useWorkflowArtifactsStore();

  const {
    bestStats,
    bestTitle,
    selectedTopSummary,
    selectedTopTitle,
    topRows,
  } = storeToRefs(analysisStore);
  const { dataId, dataTimeframe, datasetLabel, datasetRange } =
    storeToRefs(dataStore);
  const { backend, objective, runReadinessHint, strategyKind } =
    storeToRefs(optimizeStore);
  const {
    tableRowCount,
    tableVisible,
    validationOutputPreview,
    validationRouteSummary,
    validationShortlistMeta,
    validationWarningSummary,
  } = storeToRefs(validationStore);
  const { currentRun, currentValidation } = storeToRefs(workflowArtifactsStore);

  const datasetReady = computed(
    () => hasText(dataId.value) || hasText(datasetLabel.value),
  );
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
  const optimizationReady = computed(() => {
    if (stageCompletionIsComplete(optimizationCompletion.value)) return true;
    if (currentRun.value?.res) return true;
    if (Number(topRows.value || 0) > 0) return true;
    if (Array.isArray(bestStats.value) && bestStats.value.length > 0)
      return true;
    return hasText(bestTitle.value);
  });
  const candidateReady = computed(
    () =>
      selectedCandidateAvailable(selectedTopTitle.value) ||
      optimizationReady.value,
  );
  const validationReady = computed(() => {
    if (stageCompletionIsComplete(validationCompletion.value)) return true;
    if (currentValidation.value?.res) return true;
    if (Boolean(tableVisible.value)) return true;
    if (Number(tableRowCount.value || 0) > 0) return true;
    return hasText(validationOutputPreview.value);
  });
  const simpleOptimizationSupported = computed(() =>
    isSimpleOptimizationStrategySupported(strategyKind.value),
  );
  const liveStrategySupported = simpleOptimizationSupported;
  const executionSymbolReady = computed(() => hasText(executionStore.symbol));
  const executionDryRunReady = computed(() => Boolean(executionStore.dryRun));
  const executionCredentialReady = computed(() =>
    Boolean(executionStore.dryRun || executionStore.hasExecutionCredentials),
  );
  const liveFeedReady = computed(() =>
    Boolean(
      executionStore.liveActive && executionStore.liveSymbolMatchesRequested,
    ),
  );
  const runnerReady = computed(() =>
    Boolean(executionStore.dryRun && executionStore.canStartRunner),
  );

  const validationVerdictTone = computed<GuidedStatusTone>(() => {
    if (!validationReady.value) return "blocked";
    if (stageCompletionNeedsReview(validationCompletion.value))
      return "warning";
    if (validationTextLooksConcerning(validationWarningSummary.value))
      return "warning";
    if (validationTextLooksConcerning(validationOutputPreview.value))
      return "warning";
    return "done";
  });

  const validationVerdictTitle = computed(() => {
    if (!validationReady.value) return "Validation has not run yet.";
    if (validationCompletion.value?.verdict === "approved")
      return "Validation approved.";
    if (validationCompletion.value?.verdict === "approved_with_warnings")
      return "Validation approved with warnings.";
    if (validationCompletion.value?.verdict === "rejected")
      return "Validation rejected.";
    if (validationCompletion.value?.verdict === "passed")
      return "Validation passed.";
    if (validationVerdictTone.value === "warning")
      return "Validation needs review.";
    return "Validation results are available.";
  });

  const validationVerdictBody = computed(() => {
    if (!optimizationReady.value)
      return "Run an optimization before validating a candidate.";
    if (!validationReady.value)
      return "Run validation to check whether the candidate survives holdout and robustness checks.";
    const recommendation = validationCompletion.value?.recommendation || "";
    const warning =
      validationCompletion.value?.blockers?.[0] ||
      validationCompletion.value?.warnings?.[0] ||
      "";
    const packetBody = [recommendation, warning]
      .filter((part) => hasText(part))
      .join(" · ");
    if (packetBody) return packetBody;
    const shortlist = String(validationShortlistMeta.value || "").trim();
    const route = String(validationRouteSummary.value || "").trim();
    return (
      [shortlist, route].filter((part) => hasText(part)).join(" ") ||
      "Open Advanced Mode for the full validation table and report exports."
    );
  });

  const guidedChecklist = computed<GuidedChecklistItem[]>(() => [
    {
      label: "Data loaded",
      detail: datasetReady.value
        ? `${datasetLabel.value || dataId.value} · ${dataTimeframe.value || "base timeframe"}`
        : "Load CSV, sample data, or Polygon candles.",
      complete: datasetReady.value,
      tone: datasetReady.value ? "done" : "blocked",
    },
    {
      label: "Strategy selected",
      detail: `${simpleStrategyLabel(strategyKind.value)} · ${objective.value || "Sharpe"} · ${backend.value || "Auto"}`,
      complete: simpleOptimizationSupported.value,
      tone: simpleOptimizationSupported.value ? "done" : "warning",
    },
    {
      label: "Optimization run",
      detail: optimizationReady.value
        ? optimizationCompletion.value?.summary?.rankedCandidates != null
          ? `${optimizationCompletion.value.summary.rankedCandidates} ranked result${Number(optimizationCompletion.value.summary.rankedCandidates) === 1 ? "" : "s"}`
          : `${Number(topRows.value || 0)} ranked result${Number(topRows.value || 0) === 1 ? "" : "s"}`
        : String(
            runReadinessHint.value ||
              "Run optimization to generate candidates.",
          ),
      complete: optimizationReady.value,
      tone: optimizationReady.value ? "done" : "blocked",
    },
    {
      label: "Validation optional",
      detail: validationReady.value
        ? validationCompletion.value?.recommendation ||
          `${Number(tableRowCount.value || 0)} validation row${Number(tableRowCount.value || 0) === 1 ? "" : "s"}`
        : optimizationReady.value
          ? "Run validation when you want robustness evidence."
          : "Available after optimization.",
      complete: validationReady.value,
      tone: validationReady.value
        ? validationVerdictTone.value
        : optimizationReady.value
          ? "ready"
          : "blocked",
    },
  ]);

  const tradeChecklist = computed<GuidedChecklistItem[]>(() => [
    {
      label: "Validated candidate",
      detail: validationReady.value
        ? validationVerdictTitle.value
        : "Validate the candidate before starting trade readiness.",
      complete: validationReady.value,
      tone: validationVerdictTone.value,
    },
    {
      label: "Supported strategy",
      detail: simpleOptimizationSupported.value
        ? `${simpleStrategyLabel(strategyKind.value)} is supported by Simple optimization.`
        : "Simple optimization supports Double MA crossover and Composite signal only.",
      complete: simpleOptimizationSupported.value,
      tone: simpleOptimizationSupported.value ? "done" : "warning",
    },
    {
      label: "Symbol",
      detail: executionSymbolReady.value
        ? executionStore.requestedSymbolLabel
        : "Enter the symbol to trade.",
      complete: executionSymbolReady.value,
      tone: executionSymbolReady.value ? "done" : "blocked",
    },
    {
      label: "Dry-run first",
      detail: executionDryRunReady.value
        ? "Dry-run is enabled."
        : "Open Advanced Mode for dry-run, paper, or broker-armed launch.",
      complete: executionDryRunReady.value,
      tone: executionDryRunReady.value ? "done" : "warning",
    },
    {
      label: "Credentials",
      detail: executionCredentialReady.value
        ? executionStore.keyStateLabel
        : "API key and secret are required when dry-run is off.",
      complete: executionCredentialReady.value,
      tone: executionCredentialReady.value ? "done" : "blocked",
    },
    {
      label: "Live candles",
      detail: liveFeedReady.value
        ? executionStore.feedStateLabel
        : executionStore.liveStatusHint,
      complete: liveFeedReady.value,
      tone: liveFeedReady.value ? "done" : "blocked",
    },
    {
      label: "Auto-runner",
      detail: runnerReady.value
        ? "Ready to start the dry-run runner."
        : executionStore.dryRun
          ? executionStore.runnerStatusHint
          : "Open Advanced Mode to start a broker-armed runner after manual review.",
      complete: runnerReady.value,
      tone: runnerReady.value ? "ready" : "blocked",
    },
  ]);

  return {
    candidateReady,
    datasetReady,
    executionCredentialReady,
    executionDryRunReady,
    executionSymbolReady,
    guidedChecklist,
    liveFeedReady,
    liveStrategySupported,
    optimizationReady,
    runnerReady,
    simpleOptimizationSupported,
    tradeChecklist,
    validationReady,
    validationVerdictBody,
    validationVerdictTitle,
    validationVerdictTone,
    selectedTopSummary,
    selectedTopTitle,
    datasetRange,
  };
}
