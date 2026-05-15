import { computed } from "vue";
import { storeToRefs } from "pinia";
import { useTaskLifecycleSummary } from "./useTaskLifecycleSummary";
import { useUiReadiness } from "./useUiReadiness";
import { useAnalysisStore } from "../stores/analysisStore";
import { useDataStore } from "../stores/dataStore";
import { useOptimizeStore } from "../stores/optimizeStore";
import { useValidationStore } from "../stores/validationStore";
import { useWorkflowArtifactsStore } from "../stores/workflowArtifactsStore";
import { useWorkflowStore } from "../stores/workflowStore";
import {
  resolveWorkbenchPrimaryAction,
  type WorkbenchPrimaryAction,
} from "../utils/workbenchActionModel";
import { buildWorkbenchNavigationState } from "../utils/workbenchNavigationModel";
import { ML_SIGNAL_WORKFLOW_STEP_LABELS } from "../utils/core/workflowModel";
import {
  latestStageCompletionForStage,
  stageCompletionIsComplete,
  stageCompletionNeedsReview,
} from "../utils/core/stageCompletion";

export type ResearchCockpitTone =
  | "neutral"
  | "accent"
  | "success"
  | "warning"
  | "danger";
export type ResearchCockpitStepState =
  | "active"
  | "complete"
  | "available"
  | "blocked";
export type ResearchCockpitStepId =
  | "data"
  | "configure"
  | "optimize"
  | "validate"
  | "analyze"
  | "finalize";

export interface ResearchCockpitContextItem {
  key: string;
  value: string;
  current: boolean;
  defaultVisible: boolean;
}

export interface ResearchCockpitStep {
  id: ResearchCockpitStepId;
  label: string;
  detail: string;
  state: ResearchCockpitStepState;
  actionId: string;
}

export interface ResearchCockpitCard {
  id: string;
  title: string;
  value: string;
  detail: string;
  tone: ResearchCockpitTone;
}

export interface ResearchCockpitModel {
  title: string;
  summary: string;
  modeLabel: string;
  primaryAction: WorkbenchPrimaryAction;
  primaryDetail: string;
  steps: ResearchCockpitStep[];
  cards: ResearchCockpitCard[];
  contextItems: ResearchCockpitContextItem[];
  secondaryContextItems: ResearchCockpitContextItem[];
  secondaryContextSummary: string;
}

const NO_CANDIDATE_LABEL = "No ranked candidate selected.";

function clean(value: unknown, fallback = "—"): string {
  const text = String(value ?? "").trim();
  if (!text || text === "undefined" || text === "NaN") return fallback;
  return text;
}

function hasUsefulText(value: unknown): boolean {
  const text = clean(value, "");
  return text.length > 0 && text !== "-" && text !== "—";
}

function candidateIsSelected(value: unknown): boolean {
  const text = clean(value, "");
  return hasUsefulText(text) && text !== NO_CANDIDATE_LABEL;
}

function card(
  id: string,
  title: string,
  value: unknown,
  detail: unknown,
  tone: ResearchCockpitTone,
): ResearchCockpitCard {
  return {
    id,
    title,
    value: clean(value),
    detail: clean(detail, ""),
    tone,
  };
}

function packetCountDetail(value: unknown, noun: string): string {
  const parsed = Math.max(0, Math.floor(Number(value ?? 0)));
  return parsed > 0 ? `${parsed} ${noun}${parsed === 1 ? "" : "s"}` : "";
}

function step(
  id: ResearchCockpitStepId,
  label: string,
  detail: string,
  state: ResearchCockpitStepState,
): ResearchCockpitStep {
  return {
    id,
    label,
    detail,
    state,
    actionId: `workbench.cockpit.step.${id}`,
  };
}

export function useResearchCockpitModel() {
  const workflowStore = useWorkflowStore();
  const dataStore = useDataStore();
  const optimizeStore = useOptimizeStore();
  const analysisStore = useAnalysisStore();
  const validationStore = useValidationStore();
  const workflowArtifactsStore = useWorkflowArtifactsStore();

  const { controlsTab, mode, resultsTab, workspace, workflowStep } =
    storeToRefs(workflowStore);
  const { dataId, dataLen, dataTimeframe, datasetLabel, datasetRange } =
    storeToRefs(dataStore);
  const {
    backend,
    canCancel,
    gpuStatus,
    mlSignalDatasetExportPath,
    mlSignalScoreCachePath,
    mlSignalScoreInputPath,
    objective,
    pairsPreview,
    progressMeta,
    runDisabled,
    runReadinessHint,
    strategyKind,
    validateDisabled,
  } = storeToRefs(optimizeStore);
  const { bestMeta, bestTitle, selectedTopTitle, topRows } =
    storeToRefs(analysisStore);
  const {
    statusLine: validationStatusLine,
    tableRowCount,
    validationMeta,
    validationWarningSummary,
  } = storeToRefs(validationStore);
  const { currentRun, currentValidation } = storeToRefs(workflowArtifactsStore);
  const { activeTaskCount, centerStatusLabel, taskHeadline } =
    useTaskLifecycleSummary();
  const {
    datasetReady,
    hasOptimizationResults,
    hasValidationResults,
    validationStage,
  } = useUiReadiness();
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
  const optimizationComplete = computed(
    () =>
      hasOptimizationResults.value ||
      stageCompletionIsComplete(optimizationCompletion.value),
  );
  const validationComplete = computed(
    () =>
      hasValidationResults.value ||
      stageCompletionIsComplete(validationCompletion.value),
  );

  const navState = computed(() =>
    buildWorkbenchNavigationState({
      workspace: workspace.value,
      mode: mode.value,
      workflowStep: workflowStep.value,
      controlsTab: controlsTab.value,
      resultsTab: resultsTab.value,
    }),
  );

  const selectedCandidate = computed(() =>
    clean(selectedTopTitle.value, "No candidate"),
  );
  const selectedCandidateCurrent = computed(() =>
    candidateIsSelected(selectedTopTitle.value),
  );

  const primaryAction = computed(() =>
    resolveWorkbenchPrimaryAction({
      canCancel: canCancel.value,
      mode: navState.value.mode,
      workflowStep: navState.value.activeWorkflowStep,
      dataLoaded: datasetReady.value || hasUsefulText(dataId.value),
      hasOptimizationResults: optimizationComplete.value,
      runDisabled: runDisabled.value,
      validateDisabled: validateDisabled.value,
      runReadinessHint: runReadinessHint.value,
      validationHint: validationStore.validationHint,
      validationStage: validationStage.value,
      mlSignalDatasetExportPath: mlSignalDatasetExportPath.value,
      mlSignalScoreInputPath: mlSignalScoreInputPath.value,
      mlSignalScoreCachePath: mlSignalScoreCachePath.value,
    }),
  );

  const contextItems = computed<ResearchCockpitContextItem[]>(() => {
    const dataset = clean(datasetLabel.value, "");
    const range = clean(datasetRange.value, "");
    const rows = clean(dataLen.value, "");
    const strategy = clean(strategyKind.value, "");
    const objectiveLabel = clean(objective.value, "");
    const backendLabel = clean(backend.value, "");
    const timeframe = clean(dataTimeframe.value, "");
    const status =
      clean(progressMeta.value, "") || clean(runReadinessHint.value, "");
    const loaded = clean(dataId.value, "");
    const modeItems: ResearchCockpitContextItem[] =
      navState.value.mode === "ml-signal"
        ? [
            {
              key: "Workflow",
              value: "ML Signal",
              current: true,
              defaultVisible: true,
            },
            {
              key: "Stage",
              value:
                navState.value.stageItems.find((item) => item.selected)
                  ?.shortLabel || "Data",
              current: true,
              defaultVisible: true,
            },
          ]
        : [
            {
              key: "Strategy",
              value:
                [strategy, objectiveLabel].filter(hasUsefulText).join(" · ") ||
                "—",
              current: hasUsefulText(strategy) || hasUsefulText(objectiveLabel),
              defaultVisible: true,
            },
            {
              key: "Backend",
              value: backendLabel || "—",
              current: false,
              defaultVisible: true,
            },
          ];

    return [
      {
        key: "Dataset",
        value: dataset
          ? `${dataset}${timeframe ? ` · ${timeframe}` : ""}`
          : "No dataset",
        current: !dataset,
        defaultVisible: true,
      },
      {
        key: "Range",
        value: range || "No range",
        current: false,
        defaultVisible: true,
      },
      {
        key: "Rows",
        value: rows || "—",
        current: false,
        defaultVisible: false,
      },
      ...modeItems,
      {
        key: "Candidate",
        value: selectedCandidateCurrent.value
          ? selectedCandidate.value
          : "No candidate",
        current: selectedCandidateCurrent.value,
        defaultVisible:
          navState.value.mode === "conventional" &&
          navState.value.activeWorkflowStep !== "data",
      },
      {
        key: "Status",
        value: status || "Ready.",
        current: canCancel.value || activeTaskCount.value > 0,
        defaultVisible: true,
      },
      {
        key: "Loaded ID",
        value: loaded || "—",
        current: false,
        defaultVisible: false,
      },
    ];
  });

  const steps = computed<ResearchCockpitStep[]>(() => {
    const currentStep = navState.value.activeWorkflowStep;
    const activeControls = navState.value.activeControlsTab;
    if (navState.value.mode === "ml-signal") {
      const scoreCacheReady = hasUsefulText(mlSignalScoreCachePath.value);
      const exportReady = hasUsefulText(mlSignalDatasetExportPath.value);
      return [
        step(
          "data",
          ML_SIGNAL_WORKFLOW_STEP_LABELS.data,
          datasetReady.value ? "Dataset loaded" : "Load dataset",
          currentStep === "data"
            ? "active"
            : datasetReady.value
              ? "complete"
              : "available",
        ),
        step(
          "optimize",
          ML_SIGNAL_WORKFLOW_STEP_LABELS.optimize,
          scoreCacheReady
            ? "Cache selected"
            : exportReady
              ? "Build cache"
              : "Choose output",
          currentStep === "optimize"
            ? "active"
            : scoreCacheReady
              ? "complete"
              : datasetReady.value
                ? "available"
                : "blocked",
        ),
        step(
          "validate",
          ML_SIGNAL_WORKFLOW_STEP_LABELS.validate,
          validationComplete.value ? "Evidence available" : "Run policy",
          currentStep === "validate"
            ? "active"
            : validationComplete.value
              ? "complete"
              : scoreCacheReady
                ? "available"
                : "blocked",
        ),
        step(
          "analyze",
          ML_SIGNAL_WORKFLOW_STEP_LABELS.analyze,
          scoreCacheReady ? "Sweep thresholds" : "Await cache",
          currentStep === "analyze"
            ? "active"
            : scoreCacheReady
              ? "available"
              : "blocked",
        ),
        step(
          "finalize",
          ML_SIGNAL_WORKFLOW_STEP_LABELS.finalize,
          scoreCacheReady ? "Inspect artifacts" : "Await outputs",
          currentStep === "finalize"
            ? "active"
            : scoreCacheReady
              ? "available"
              : "blocked",
        ),
      ];
    }
    return [
      step(
        "data",
        "Data",
        datasetReady.value ? "Loaded" : "Required",
        currentStep === "data"
          ? "active"
          : datasetReady.value
            ? "complete"
            : "available",
      ),
      step(
        "configure",
        "Configure",
        hasUsefulText(strategyKind.value)
          ? clean(strategyKind.value)
          : "Choose strategy",
        currentStep === "optimize" && activeControls === "strategy"
          ? "active"
          : hasUsefulText(strategyKind.value)
            ? "complete"
            : "available",
      ),
      step(
        "optimize",
        "Optimize",
        optimizationComplete.value
          ? `${Math.max(0, Number(topRows.value || optimizationCompletion.value?.summary?.rankedCandidates || 0))} ranked`
          : "Run search",
        currentStep === "optimize" && activeControls !== "strategy"
          ? "active"
          : optimizationComplete.value
            ? "complete"
            : datasetReady.value
              ? "available"
              : "blocked",
      ),
      step(
        "validate",
        "Validate",
        validationComplete.value
          ? validationCompletion.value?.recommendation || "Evidence available"
          : "Prove robustness",
        currentStep === "validate"
          ? "active"
          : validationComplete.value
            ? "complete"
            : optimizationComplete.value
              ? "available"
              : "blocked",
      ),
      step(
        "analyze",
        "Analyze",
        optimizationComplete.value ? "Inspect results" : "Await results",
        currentStep === "analyze" || currentStep === "finalize"
          ? "active"
          : optimizationComplete.value
            ? "available"
            : "blocked",
      ),
    ];
  });

  const cards = computed<ResearchCockpitCard[]>(() => {
    const datasetDetail = [
      clean(dataTimeframe.value, ""),
      clean(dataLen.value, ""),
      clean(datasetRange.value, ""),
    ]
      .filter(hasUsefulText)
      .join(" · ");
    const strategyDetail = [
      clean(backend.value, ""),
      clean(gpuStatus.value, ""),
      clean(pairsPreview.value, ""),
    ]
      .filter(hasUsefulText)
      .join(" · ");
    const packetResultDetail =
      packetCountDetail(
        optimizationCompletion.value?.summary?.rankedCandidates,
        "ranked candidate",
      ) ||
      clean(optimizationCompletion.value?.route, "") ||
      clean(optimizationCompletion.value?.backendUsed, "");
    const resultDetail =
      clean(bestMeta.value, "") ||
      packetResultDetail ||
      clean(currentRun.value?.artifactPath, "") ||
      clean(runReadinessHint.value, "");
    const packetValidationDetail =
      clean(validationCompletion.value?.recommendation, "") ||
      clean(validationCompletion.value?.blockers?.[0], "") ||
      clean(validationCompletion.value?.warnings?.[0], "") ||
      packetCountDetail(
        validationCompletion.value?.summary?.candidatesTested,
        "candidate tested",
      );
    const validationDetail =
      packetValidationDetail ||
      clean(validationWarningSummary.value, "") ||
      clean(validationStatusLine.value, "") ||
      clean(currentValidation.value?.ts, "");
    return [
      card(
        "dataset",
        "Active Dataset",
        datasetReady.value ? datasetLabel.value : "No dataset",
        datasetDetail || "Load CSV or Polygon candles.",
        datasetReady.value ? "success" : "warning",
      ),
      card(
        "strategy",
        "Strategy",
        clean(strategyKind.value, "No strategy"),
        strategyDetail || "Configure optimization inputs.",
        hasUsefulText(strategyKind.value) ? "accent" : "neutral",
      ),
      card(
        "results",
        "Latest Result",
        optimizationComplete.value
          ? clean(bestTitle.value, "Optimization result")
          : "No run yet",
        resultDetail || "Run optimization to rank candidates.",
        optimizationComplete.value ? "success" : "neutral",
      ),
      card(
        "validation",
        "Validation",
        validationComplete.value
          ? clean(validationMeta.value, "Validation ready")
          : validationStage.value,
        validationDetail ||
          `${Math.max(0, Number(tableRowCount.value || 0))} rows`,
        validationComplete.value
          ? stageCompletionNeedsReview(validationCompletion.value)
            ? "warning"
            : "success"
          : optimizationComplete.value
            ? "warning"
            : "neutral",
      ),
      card(
        "tasks",
        "Tasks",
        activeTaskCount.value
          ? `${activeTaskCount.value} active`
          : centerStatusLabel.value,
        taskHeadline.value,
        activeTaskCount.value ? "accent" : "neutral",
      ),
    ];
  });

  const model = computed<ResearchCockpitModel>(() => ({
    title:
      navState.value.mode === "ml-signal"
        ? "ML Signal Research"
        : "Research Cockpit",
    summary: datasetReady.value
      ? "Data, strategy, results, validation, and tasks are staged for the current research run."
      : "Start by loading data, then configure a strategy and run optimization.",
    modeLabel: navState.value.modeLabel,
    primaryAction: primaryAction.value,
    primaryDetail: primaryAction.value.disabled
      ? primaryAction.value.disabledReason
      : clean(progressMeta.value, "") ||
        clean(runReadinessHint.value, "Ready."),
    steps: steps.value,
    cards: cards.value,
    contextItems: contextItems.value.filter((item) => item.defaultVisible),
    secondaryContextItems: contextItems.value.filter(
      (item) => !item.defaultVisible,
    ),
    secondaryContextSummary: contextItems.value
      .filter((item) => !item.defaultVisible)
      .map((item) => item.key)
      .join(" · "),
  }));

  function openStep(id: ResearchCockpitStepId): void {
    if (id === "data") {
      workflowStore.selectWorkflowStep("data");
      return;
    }
    if (id === "configure") {
      workflowStore.selectWorkflowStep("optimize");
      workflowStore.selectControlsTab("strategy");
      return;
    }
    if (id === "optimize") {
      workflowStore.selectWorkflowStep("optimize");
      workflowStore.selectControlsTab("opt");
      return;
    }
    if (id === "validate") {
      workflowStore.selectWorkflowStep("validate");
      return;
    }
    if (id === "finalize") {
      workflowStore.selectWorkflowStep("finalize");
      return;
    }
    workflowStore.selectWorkflowStep("analyze");
  }

  return {
    model,
    openStep,
  };
}
