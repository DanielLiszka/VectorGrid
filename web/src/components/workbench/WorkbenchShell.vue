<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { ListTodo, PanelRightOpen, SlidersHorizontal, X } from "@lucide/vue";
import {
  TASK_CENTER_PANEL_ID,
  openTaskCenter as requestTaskCenterOpen,
  useTaskCenterVisibility,
} from "../../composables/useTaskCenterVisibility";
import { useTaskLifecycleSummary } from "../../composables/useTaskLifecycleSummary";
import { useUiReadiness } from "../../composables/useUiReadiness";
import { useAnalysisStore } from "../../stores/analysisStore";
import { useDataStore } from "../../stores/dataStore";
import { useHistoryStore } from "../../stores/historyStore";
import { useOptimizeStore } from "../../stores/optimizeStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useValidationStore } from "../../stores/validationStore";
import { useWorkflowStore } from "../../stores/workflowStore";
import { tauriRuntimeAvailable } from "../../tauri/invoke";
import {
  loadExecutionStrategySupportMatrix,
  type ExecutionStrategySupportRow,
} from "../../services/executionService";
import { previewValidationContextCompatibilityFromStores } from "../../services/validationService";
import {
  resolveWorkbenchInspectorState,
  type WorkbenchInspectorMode,
} from "../../utils/inspectorModel";
import { storeErrorText } from "../../utils/storeErrorText";
import {
  buildWorkbenchNavigationState,
  workbenchWorkspaceLabel,
  type WorkbenchLocalTabItem,
  type WorkbenchNavigationItem,
} from "../../utils/workbenchNavigationModel";
import WorkbenchActionBar from "./WorkbenchActionBar.vue";
import WorkbenchInspector from "./WorkbenchInspector.vue";
import WorkbenchRail from "./WorkbenchRail.vue";
import ResearchCockpit from "./ResearchCockpit.vue";

type TabKeyItem = { id: string };
type AdvancedDrawerAction = {
  actionId: string;
  label: string;
  detail: string;
  run: () => void;
};

const workflowStore = useWorkflowStore();
const settingsStore = useSettingsStore();
const dataStore = useDataStore();
const optimizeStore = useOptimizeStore();
const analysisStore = useAnalysisStore();
const validationStore = useValidationStore();
const historyStore = useHistoryStore();

const {
  controlsTab,
  mode,
  resultsTab,
  workspace,
  workflowStep,
  workflowSubtitle,
} = storeToRefs(workflowStore);
const { controlsCollapsed } = storeToRefs(settingsStore);
const { dataLen, dataTimeframe, datasetLabel, datasetRange } =
  storeToRefs(dataStore);
const {
  backend,
  gpuStatus,
  objective,
  pairsPreview,
  progressMeta,
  runReadinessHint,
  strategyKind,
} = storeToRefs(optimizeStore);
const { bestMeta, selectedTopSummary, selectedTopTitle } =
  storeToRefs(analysisStore);
const {
  crossMeta,
  cscvSlices,
  minDsr,
  minPsr,
  statusLine: validationStatusLine,
  tableRowCount,
  updatedAtUnixMs: validationUpdatedAtUnixMs,
  validationAccelerationStatus,
  validationMeta,
  validationOutputPreview,
  validationWarningSummary,
} = storeToRefs(validationStore);
const {
  canApplyHistory,
  canApplySavedRun,
  canClearHistory,
  canLoadSavedRun,
  canOpenRunsDir,
  canRefreshSavedRuns,
  compareRunCount,
  savedRunCount,
  savedRunDetailMeta,
  savedRunDetailTitle,
  savedRunsMeta,
  savedSelectedLabel,
  savedSelectedValue,
  sessionRunCount,
  sessionSelectedLabel,
  statusLine: historyStatusLine,
} = storeToRefs(historyStore);
const { activeTaskCount, centerStatusLabel, taskHeadline } =
  useTaskLifecycleSummary();
const taskCenterOpen = useTaskCenterVisibility();
const { datasetReady, hasOptimizationResults, validationStage } =
  useUiReadiness();

const inspectorCollapsed = ref(false);
const compactInspector = ref(false);
const compactInspectorOpen = ref(false);
const requestedInspectorMode = ref<WorkbenchInspectorMode | "">("");
const advancedDrawerOpen = ref(false);
let compactInspectorQuery: MediaQueryList | null = null;

function syncCompactInspector(event?: MediaQueryListEvent): void {
  const matches = event?.matches ?? compactInspectorQuery?.matches ?? false;
  compactInspector.value = matches;
  if (!matches) compactInspectorOpen.value = false;
}

onMounted(() => {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
  compactInspectorQuery = window.matchMedia("(max-width: 1180px)");
  syncCompactInspector();
  compactInspectorQuery.addEventListener("change", syncCompactInspector);
});

onBeforeUnmount(() => {
  compactInspectorQuery?.removeEventListener("change", syncCompactInspector);
  compactInspectorQuery = null;
});

const modeItems = [
  {
    id: "conventional",
    label: "Conventional",
    actionId: "shell.mode.conventional",
    tabId: "mode_tab_conventional",
  },
  {
    id: "options",
    label: "Options",
    actionId: "shell.mode.options",
    tabId: "mode_tab_options",
  },
  {
    id: "ml-signal",
    label: "ML Signal",
    actionId: "shell.mode.mlSignal",
    tabId: "mode_tab_ml-signal",
  },
] as const;

const navState = computed(() =>
  buildWorkbenchNavigationState({
    workspace: workspace.value,
    mode: mode.value,
    workflowStep: workflowStep.value,
    controlsTab: controlsTab.value,
    resultsTab: resultsTab.value,
  }),
);
const currentWorkspace = computed(() => navState.value.activeWorkspaceId);
const currentWorkflowStep = computed(() => navState.value.activeWorkflowStep);
const currentMode = computed(() => navState.value.mode);
const currentWorkspaceLabel = computed(() =>
  workbenchWorkspaceLabel(currentWorkspace.value),
);
const shellSubtitle = computed(
  () => String(workflowSubtitle.value || "").trim() || "Ready.",
);
const standardWorkspaceActive = computed(
  () =>
    currentWorkspace.value !== "settings" &&
    currentWorkspace.value !== "live" &&
    currentWorkspace.value !== "portfolio",
);
const liveWorkspaceActive = computed(
  () => currentWorkspace.value === "live",
);
const portfolioWorkspaceActive = computed(
  () => currentWorkspace.value === "portfolio",
);
const settingsWorkspaceActive = computed(
  () => currentWorkspace.value === "settings",
);
const liveSupportRows = ref<ExecutionStrategySupportRow[]>([]);
const liveSupportMatrixStatus = ref(
  "Support matrix uses desktop backend data when Tauri is available.",
);
const liveSupportMatrixLoaded = ref(false);
const liveSupportStrategiesLabel = computed(() => {
  const supported = liveSupportRows.value
    .filter((row) => row.supported)
    .map((row) => row.strategyKind)
    .filter((value) => value.length > 0);
  return supported.length ? supported.join(" · ") : "DoubleMa · CompositeCpu";
});
const liveSupportAdaptersLabel = computed(() => {
  const adapters = liveSupportRows.value
    .filter((row) => row.supported)
    .map((row) => row.adapter || row.supportLevel)
    .filter((value) => value.length > 0);
  return adapters.length ? adapters.join(" · ") : "Backend support matrix";
});
async function refreshLiveSupportMatrix(): Promise<void> {
  if (liveSupportMatrixLoaded.value || !tauriRuntimeAvailable()) return;
  try {
    const rows = await loadExecutionStrategySupportMatrix();
    liveSupportRows.value = rows;
    liveSupportMatrixStatus.value = rows.length
      ? `${rows.filter((row) => row.supported).length} supported strategy adapters reported by backend.`
      : "Backend returned an empty live strategy support matrix.";
  } catch (error) {
    liveSupportMatrixStatus.value =
      `Live support matrix unavailable: ${storeErrorText(error)}`.trim();
  } finally {
    liveSupportMatrixLoaded.value = true;
  }
}
const visibleControlsItems = computed(() => navState.value.controlsItems);
const visibleResultsItems = computed(() => navState.value.resultsItems);
const controlsColumnVisible = computed(
  () =>
    standardWorkspaceActive.value &&
    visibleControlsItems.value.length > 0 &&
    !controlsCollapsed.value,
);
const resultsColumnVisible = computed(
  () => standardWorkspaceActive.value && visibleResultsItems.value.length > 0,
);
const controlsTabsVisible = computed(
  () => controlsColumnVisible.value && visibleControlsItems.value.length > 1,
);
const resultsTabsVisible = computed(
  () => resultsColumnVisible.value && visibleResultsItems.value.length > 1,
);
const researchCockpitVisible = computed(
  () => standardWorkspaceActive.value && currentWorkspace.value === "workflow",
);
const advancedDrawerVisible = computed(
  () => researchCockpitVisible.value && advancedDrawerOpen.value,
);
const activeModeLabel = computed(
  () => modeItems.find((item) => item.id === currentMode.value)?.label || "Conventional",
);
const activeStageLabel = computed(
  () =>
    navState.value.stageItems.find((item) => item.id === currentWorkflowStep.value)
      ?.label || currentWorkflowStep.value,
);
const advancedDrawerTitle = computed(() => `${activeStageLabel.value} Controls`);
const advancedDrawerSummary = computed(() => {
  if (currentMode.value === "options") {
    return "Options research keeps expression, dataset, validation, and replay controls in the foreground; parent optimization, library, and operator tools stay one hop away.";
  }
  if (currentMode.value === "ml-signal") {
    return "ML Signal keeps the active dataset, score-cache, validation, or policy-sweep task focused while preserving parent data, artifacts, and diagnostics.";
  }
  if (currentWorkflowStep.value === "validate") {
    return "Validation foregrounds robustness evidence. Strategy ranges, optimization setup, meta-analysis, and diagnostics remain reachable here.";
  }
  if (currentWorkflowStep.value === "analyze") {
    return "Candidate Review foregrounds top candidates, charts, heatmaps, and Pareto evidence. Run controls and downstream review stay available here.";
  }
  if (currentWorkflowStep.value === "finalize") {
    return "Final Review foregrounds promotion evidence. Use these entries for meta-analysis, artifacts, diagnostics, or returning to earlier setup.";
  }
  return "Use this drawer for related expert controls without changing the current stage until an action is selected.";
});
const workbenchGridClass = computed(() => ({
  singleColumn: !controlsColumnVisible.value || !resultsColumnVisible.value,
}));
const selectedShellLabelledBy = computed(() => navState.value.selectedShellTabId);
const dataContextLabel = computed(() => {
  const label = String(datasetLabel.value || "").trim();
  const timeframe = String(dataTimeframe.value || "").trim();
  const rows = String(dataLen.value || "").trim();
  const range = String(datasetRange.value || "").trim();
  if (!label) return "No dataset";
  return [label, timeframe, rows ? `${rows} rows` : "", range]
    .filter(Boolean)
    .join(" · ");
});
const validationCompatibilityPreview = computed(() => {
  void dataStore.updatedAtUnixMs;
  void validationUpdatedAtUnixMs.value;
  return previewValidationContextCompatibilityFromStores();
});
const validationGatesLabel = computed(() => {
  const parts: string[] = [];
  if (minPsr.value) parts.push(`PSR>=${minPsr.value}`);
  if (minDsr.value) parts.push(`DSR>=${minDsr.value}`);
  if (cscvSlices.value) parts.push(`CSCV=${cscvSlices.value}`);
  return parts.join(" · ") || "—";
});
const strategyContextLabel = computed(() => {
  const strategy = String(strategyKind.value || "").trim();
  const objectiveLabel = String(objective.value || "").trim();
  return [strategy, objectiveLabel].filter(Boolean).join(" · ");
});
const backendContextLabel = computed(() => {
  const backendLabel = String(backend.value || "").trim();
  const gpu = String(gpuStatus.value || "").trim();
  const pairs = String(pairsPreview.value || "").trim();
  return [backendLabel, gpu, pairs].filter(Boolean).join(" · ");
});
const inspectorState = computed(() =>
  resolveWorkbenchInspectorState({
    workspace: currentWorkspace.value,
    workflowStep: currentWorkflowStep.value,
    resultsTab: navState.value.activeResultsTab,
    requestedMode: requestedInspectorMode.value,
    datasetLabel: dataContextLabel.value,
    strategyLabel: strategyContextLabel.value,
    backendLabel: backendContextLabel.value,
    selectedCandidateTitle: hasOptimizationResults.value
      ? selectedTopTitle.value
      : "No candidate",
    selectedCandidateSummary: hasOptimizationResults.value
      ? String(selectedTopSummary.value || bestMeta.value || "").trim()
      : "Run optimization for the active dataset.",
    status: String(
      progressMeta.value ||
        (datasetReady.value && !hasOptimizationResults.value
          ? "Ready to configure and run optimization."
          : runReadinessHint.value) ||
        centerStatusLabel.value ||
        "",
    ).trim(),
    validationStage: validationStage.value,
    validationSummary: String(
      validationAccelerationStatus.value ||
        validationMeta.value ||
        validationWarningSummary.value ||
        "",
    ).trim(),
    validationOutputPreview: validationOutputPreview.value,
    validationCrossSummary: crossMeta.value,
    validationTableRows: tableRowCount.value,
    validationGatesLabel: validationGatesLabel.value,
    validationCompatibilitySummary:
      validationCompatibilityPreview.value.summary,
    validationCompatibilityDetail: validationCompatibilityPreview.value.detail,
    validationStatus: validationStatusLine.value,
    historySummary:
      currentWorkspace.value === "library"
        ? "Session and saved run library"
        : "Session and saved runs",
    historySessionSummary: `${sessionRunCount.value} runs · ${sessionSelectedLabel.value}`,
    historySavedSummary: `${savedRunCount.value} saved · ${savedSelectedLabel.value}`,
    historyCompareSummary: `${compareRunCount.value} choices · ${savedRunsMeta.value}`,
    historyDetailTitle: savedRunDetailTitle.value,
    historyDetailMeta: savedRunDetailMeta.value,
    historyStatus: historyStatusLine.value,
    historyCanApplySession: canApplyHistory.value,
    historyCanRefreshSaved: canRefreshSavedRuns.value,
    historyCanLoadSaved: canLoadSavedRun.value,
    historyCanApplySaved: canApplySavedRun.value,
    historyCanClearSession: canClearHistory.value,
    historyCanOpenRunsDir: canOpenRunsDir.value,
    activeTaskCount: activeTaskCount.value,
    taskSummary: taskHeadline.value,
    diagnosticsSummary:
      `${navState.value.selectionAnnouncement} ${backendContextLabel.value}`.trim(),
  }),
);

watch(
  [
    () => currentWorkspace.value,
    () => currentWorkflowStep.value,
    () => navState.value.activeResultsTab,
  ],
  () => {
    requestedInspectorMode.value = "";
  },
);

watch(
  () => researchCockpitVisible.value,
  (visible) => {
    if (!visible) advancedDrawerOpen.value = false;
  },
);

watch(
  () => liveWorkspaceActive.value,
  (active) => {
    if (active) void refreshLiveSupportMatrix();
  },
  { immediate: true },
);

watch(
  [visibleControlsItems, visibleResultsItems],
  () => {
    const controlsKeys = visibleControlsItems.value.map((item) => item.id);
    const resultsKeys = visibleResultsItems.value.map((item) => item.id);
    const activeControls = navState.value.activeControlsTab;
    const activeResults = navState.value.activeResultsTab;
    if (
      controlsKeys.length > 0 &&
      (!activeControls || !controlsKeys.includes(activeControls))
    ) {
      const nextControls = controlsKeys[0];
      if (nextControls)
        workflowStore.selectControlsTab(nextControls, { persist: false });
    }
    if (
      resultsKeys.length > 0 &&
      (!activeResults || !resultsKeys.includes(activeResults))
    ) {
      const nextResults = resultsKeys[0];
      if (nextResults)
        workflowStore.selectResultsTab(nextResults, { persist: false });
    }
  },
  { immediate: true },
);

function selectWorkspace(id: string): void {
  workflowStore.selectWorkspace(id);
}

function selectMode(id: string): void {
  workflowStore.selectMode(id);
}

function selectWorkflowStep(id: string): void {
  workflowStore.selectWorkflowStep(id);
}

function selectControlsTab(id: string): void {
  workflowStore.selectControlsTab(id);
}

function selectResultsTab(id: string): void {
  workflowStore.selectResultsTab(id);
}

function openTaskCenter(): void {
  requestTaskCenterOpen(taskCenterOpen);
}

function toggleAdvancedDrawer(): void {
  advancedDrawerOpen.value = !advancedDrawerOpen.value;
}

function closeAdvancedDrawer(): void {
  advancedDrawerOpen.value = false;
}

function openStrategyControls(): void {
  workflowStore.selectWorkflowStep("optimize");
  workflowStore.selectControlsTab("strategy");
  closeAdvancedDrawer();
}

function openOptimizationControls(): void {
  workflowStore.selectWorkflowStep("optimize");
  workflowStore.selectControlsTab("opt");
  closeAdvancedDrawer();
}

function openAdvancedDiagnostics(): void {
  workflowStore.selectWorkspace("pro");
  closeAdvancedDrawer();
}

function openMetaReview(): void {
  workflowStore.selectWorkflowStep("finalize");
  workflowStore.selectResultsTab("meta");
  closeAdvancedDrawer();
}

function openLibraryReview(): void {
  workflowStore.selectWorkspace("library");
  closeAdvancedDrawer();
}

const advancedDrawerActions = computed<AdvancedDrawerAction[]>(() => {
  const strategyDetail = strategyContextLabel.value || "No strategy";
  const backendDetail = backendContextLabel.value || "Backend route";
  const metaDetail = String(validationMeta.value || validationStage.value || "Meta and final evidence").trim();
  const libraryDetail = `Saved-run catalog · ${savedRunsMeta.value || "not refreshed"}`;
  const diagnosticsDetail = gpuStatus.value || "Operator tools";

  const strategyAction: AdvancedDrawerAction = {
    actionId: "shell.advancedDrawer.openStrategy",
    label: currentMode.value === "options" ? "Parent strategy setup" : "Strategy controls",
    detail: strategyDetail,
    run: openStrategyControls,
  };
  const optimizationAction: AdvancedDrawerAction = {
    actionId: "shell.advancedDrawer.openOptimization",
    label: currentMode.value === "ml-signal" ? "ML stage controls" : "Optimization controls",
    detail: backendDetail,
    run: openOptimizationControls,
  };
  const metaAction: AdvancedDrawerAction = {
    actionId: "shell.advancedDrawer.openMeta",
    label: currentMode.value === "options" ? "Final and meta review" : "Meta analysis",
    detail: metaDetail,
    run: openMetaReview,
  };
  const libraryAction: AdvancedDrawerAction = {
    actionId: "shell.advancedDrawer.openLibrary",
    label: "Artifact library",
    detail: libraryDetail,
    run: openLibraryReview,
  };
  const diagnosticsAction: AdvancedDrawerAction = {
    actionId: "shell.advancedDrawer.openAdvancedControls",
    label: "Diagnostics",
    detail: diagnosticsDetail,
    run: openAdvancedDiagnostics,
  };

  if (currentMode.value === "options") {
    return [libraryAction, metaAction, strategyAction, optimizationAction, diagnosticsAction];
  }
  if (currentMode.value === "ml-signal") {
    return [optimizationAction, libraryAction, metaAction, diagnosticsAction];
  }
  if (currentWorkflowStep.value === "validate") {
    return [strategyAction, optimizationAction, metaAction, libraryAction, diagnosticsAction];
  }
  if (currentWorkflowStep.value === "analyze") {
    return [optimizationAction, metaAction, libraryAction, strategyAction, diagnosticsAction];
  }
  if (currentWorkflowStep.value === "finalize") {
    return [metaAction, libraryAction, diagnosticsAction, strategyAction, optimizationAction];
  }
  return [strategyAction, optimizationAction, metaAction, libraryAction, diagnosticsAction];
});

function toggleInspector(): void {
  inspectorCollapsed.value = !inspectorCollapsed.value;
}

function openCompactInspector(): void {
  inspectorCollapsed.value = false;
  compactInspectorOpen.value = true;
}

function closeCompactInspector(): void {
  compactInspectorOpen.value = false;
}

function selectInspectorMode(mode: WorkbenchInspectorMode): void {
  requestedInspectorMode.value = mode;
}

function runInspectorAction(actionId: string): void {
  if (actionId === "history.applySession") {
    historyStore.applySession();
    return;
  }
  if (actionId === "history.refreshSaved") {
    historyStore.refreshSavedRuns();
    return;
  }
  if (actionId === "history.loadSaved") {
    historyStore.loadSavedRun(savedSelectedValue.value);
    return;
  }
  if (actionId === "history.applySaved") {
    historyStore.applySavedRun(savedSelectedValue.value);
    return;
  }
  if (actionId === "history.clearSession") {
    historyStore.clearSession();
    return;
  }
  if (actionId === "history.openRunsDir") {
    historyStore.openRunsDir();
    return;
  }
  if (actionId === "validation.open") {
    workflowStore.selectWorkflowStep("validate");
    return;
  }
  if (actionId === "tasks.open") {
    openTaskCenter();
    return;
  }
  if (actionId === "selection.openData") {
    workflowStore.selectWorkflowStep("data");
    return;
  }
  if (actionId === "selection.openOptimize") {
    workflowStore.selectWorkflowStep("optimize");
    workflowStore.selectControlsTab("opt");
    return;
  }
  if (actionId === "selection.openAnalyze") {
    workflowStore.selectWorkflowStep("analyze");
  }
}

function controlTabId(id: string): string {
  return `controls_tab_${id}`;
}

function resultTabId(id: string): string {
  return `results_tab_${id}`;
}

function isControlPanelVisible(id: string): boolean {
  return (
    visibleControlsItems.value.some((item) => item.id === id) &&
    navState.value.activeControlsTab === id
  );
}

function isResultsPanelVisible(id: string): boolean {
  return (
    visibleResultsItems.value.some((item) => item.id === id) &&
    navState.value.activeResultsTab === id
  );
}

function panelLabel(items: WorkbenchLocalTabItem[], id: string): string {
  return items.find((item) => item.id === id)?.label || id;
}

function focusSelector(selector: string): void {
  nextTick(() => document.querySelector<HTMLElement>(selector)?.focus());
}

function moveTabFocus(
  event: KeyboardEvent,
  items: readonly TabKeyItem[],
  currentId: string,
  selectTab: (id: string) => void,
  selectorForId: (id: string) => string,
): void {
  const key = event.key;
  if (
    ![
      "ArrowRight",
      "ArrowDown",
      "ArrowLeft",
      "ArrowUp",
      "Home",
      "End",
    ].includes(key)
  )
    return;
  event.preventDefault();
  const currentIndex = Math.max(
    0,
    items.findIndex((item) => item.id === currentId),
  );
  let nextIndex = currentIndex;
  if (key === "ArrowRight" || key === "ArrowDown")
    nextIndex = (currentIndex + 1) % items.length;
  else if (key === "ArrowLeft" || key === "ArrowUp")
    nextIndex = (currentIndex - 1 + items.length) % items.length;
  else if (key === "Home") nextIndex = 0;
  else if (key === "End") nextIndex = items.length - 1;
  const nextItem = items[nextIndex];
  if (!nextItem) return;
  selectTab(nextItem.id);
  focusSelector(selectorForId(nextItem.id));
}

function onModeKeydown(event: KeyboardEvent, id: string): void {
  moveTabFocus(
    event,
    modeItems,
    id,
    selectMode,
    (nextId) => `#modeBar [data-mode="${nextId}"]`,
  );
}

function onStageKeydown(
  event: KeyboardEvent,
  item: WorkbenchNavigationItem,
): void {
  moveTabFocus(
    event,
    navState.value.stageItems,
    item.id,
    selectWorkflowStep,
    (nextId) => `#workflowStepBar [data-workflow-step="${nextId}"]`,
  );
}

function onControlsKeydown(
  event: KeyboardEvent,
  item: WorkbenchLocalTabItem,
): void {
  moveTabFocus(
    event,
    visibleControlsItems.value,
    item.id,
    selectControlsTab,
    (nextId) => `#controlsTabBar [data-tab="${nextId}"]`,
  );
}

function onResultsKeydown(
  event: KeyboardEvent,
  item: WorkbenchLocalTabItem,
): void {
  moveTabFocus(
    event,
    visibleResultsItems.value,
    item.id,
    selectResultsTab,
    (nextId) => `#resultsTabBar [data-tab="${nextId}"]`,
  );
}
</script>

<template>
  <section
    class="workbenchRoot"
    :class="{ inspectorCollapsed }"
    data-qa-shell="workbench"
  >
    <WorkbenchRail
      :items="navState.workspaceItems"
      @select="selectWorkspace"
    />

    <div class="workbenchContent">
      <header
        class="workbenchHeader"
        role="banner"
        aria-label="Workspace shell header"
      >
        <div class="titleRow">
          <div class="titleCopy">
            <span class="eyebrow">VectorGrid</span>
            <h1>{{ currentWorkspaceLabel }}</h1>
            <p>{{ shellSubtitle }}</p>
          </div>
          <div
            class="headerContext"
            aria-label="Current workbench context"
          >
            <span class="contextChip mono">{{ navState.modeLabel }}</span>
            <span class="contextChip mono">{{ dataContextLabel }}</span>
          </div>
        </div>

        <div class="toolbarRow">
          <div
            v-if="currentWorkspace === 'workflow'"
            id="modeBar"
            class="segmentedBar"
            role="tablist"
            aria-label="Workflow mode"
          >
            <button
              v-for="item in modeItems"
              :id="item.tabId"
              :key="item.id"
              type="button"
              class="tabBtn modeBtn"
              :class="{ active: currentMode === item.id }"
              role="tab"
              :aria-selected="currentMode === item.id ? 'true' : 'false'"
              :tabindex="currentMode === item.id ? 0 : -1"
              :data-mode="item.id"
              :data-qa-action="item.actionId"
              @click="selectMode(item.id)"
              @keydown="onModeKeydown($event, item.id)"
            >
              {{ item.label }}
            </button>
          </div>

          <button
            v-if="currentWorkspace === 'workflow'"
            type="button"
            class="advancedDrawerButton"
            :class="{ open: advancedDrawerOpen }"
            aria-controls="researchAdvancedDrawer"
            :aria-expanded="advancedDrawerOpen ? 'true' : 'false'"
            data-qa-action="shell.advancedDrawer.toggle"
            @click="toggleAdvancedDrawer"
          >
            <SlidersHorizontal aria-hidden="true" />
            <span>Advanced</span>
          </button>

          <button
            v-if="compactInspector"
            type="button"
            class="compactInspectorButton"
            :class="{ open: compactInspectorOpen }"
            aria-controls="workbenchInspector"
            :aria-expanded="compactInspectorOpen ? 'true' : 'false'"
            data-qa-action="workbench.inspector.compactOpen"
            @click="openCompactInspector"
          >
            <PanelRightOpen aria-hidden="true" />
            <span>Inspector</span>
          </button>

          <button
            type="button"
            class="taskLauncher"
            :class="{ active: activeTaskCount > 0, open: taskCenterOpen }"
            :aria-controls="TASK_CENTER_PANEL_ID"
            :aria-expanded="taskCenterOpen ? 'true' : 'false'"
            aria-haspopup="dialog"
            :aria-label="`Open task center. ${centerStatusLabel}.`"
            data-qa-action="shell.taskCenter.open"
            @click="openTaskCenter"
          >
            <ListTodo aria-hidden="true" />
            <span>{{ centerStatusLabel }}</span>
          </button>
        </div>

        <div
          v-if="currentWorkspace === 'workflow'"
          id="workflowStepBar"
          class="stageBar"
          role="tablist"
          aria-label="Workflow steps"
        >
          <button
            v-for="item in navState.stageItems"
            :id="item.tabId"
            :key="item.id"
            type="button"
            class="stageButton"
            :class="{ active: item.selected }"
            role="tab"
            :aria-selected="item.selected ? 'true' : 'false'"
            :aria-controls="item.ariaControls"
            :tabindex="item.selected ? 0 : -1"
            :data-workflow-step="item.id"
            :data-qa-action="item.actionId"
            @click="selectWorkflowStep(item.id)"
            @keydown="onStageKeydown($event, item)"
          >
            {{ item.label }}
          </button>
        </div>
      </header>

      <main
        id="shellMainPanel"
        class="workbenchMain"
        :aria-labelledby="selectedShellLabelledBy"
      >
        <div
          id="shellSelectionStatus"
          class="srOnly"
          role="status"
          aria-live="polite"
        >
          {{ navState.selectionAnnouncement }}
        </div>

        <ResearchCockpit v-if="researchCockpitVisible" />

        <aside
          v-if="researchCockpitVisible"
          v-show="advancedDrawerVisible"
          id="researchAdvancedDrawer"
          class="advancedDrawer"
          aria-label="Advanced workflow controls"
        >
          <div class="advancedDrawerHeader">
            <div>
              <span class="eyebrow">{{ activeModeLabel }} · Advanced</span>
              <h2>{{ advancedDrawerTitle }}</h2>
              <p>{{ advancedDrawerSummary }}</p>
            </div>
            <button
              type="button"
              class="iconOnlyButton"
              aria-label="Close advanced workflow controls"
              data-qa-action="shell.advancedDrawer.close"
              @click="closeAdvancedDrawer"
            >
              <X aria-hidden="true" />
            </button>
          </div>
          <div class="advancedDrawerGrid">
            <button
              v-for="action in advancedDrawerActions"
              :key="action.actionId"
              type="button"
              class="advancedDrawerAction"
              :data-qa-action="action.actionId"
              @click="action.run"
            >
              <span>{{ action.label }}</span>
              <strong class="mono">{{ action.detail }}</strong>
            </button>
          </div>
        </aside>

        <section
          v-show="standardWorkspaceActive"
          class="workbenchPaneGrid"
          :class="workbenchGridClass"
          aria-label="Workbench panels"
        >
          <section
            v-show="controlsColumnVisible"
            class="workbenchLane setupLane"
            aria-label="Setup pane"
          >
            <div
              v-if="controlsTabsVisible"
              id="controlsTabBar"
              class="localTabs"
              role="tablist"
              aria-label="Setup panels"
            >
              <button
                v-for="item in visibleControlsItems"
                :id="item.tabId"
                :key="item.id"
                type="button"
                class="tabBtn"
                :class="{ active: item.selected }"
                role="tab"
                :aria-selected="item.selected ? 'true' : 'false'"
                :aria-controls="item.panelId"
                :tabindex="item.selected ? 0 : -1"
                :data-tab="item.id"
                :data-qa-action="item.actionId"
                @click="selectControlsTab(item.id)"
                @keydown="onControlsKeydown($event, item)"
              >
                {{ item.label }}
              </button>
            </div>

            <div
              id="controls_panel_data"
              class="panelHost"
              data-vue-panel="data"
              :role="controlsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                controlsTabsVisible ? controlTabId('data') : undefined
              "
              :aria-label="
                controlsTabsVisible
                  ? undefined
                  : panelLabel(visibleControlsItems, 'data')
              "
              :aria-hidden="!isControlPanelVisible('data')"
              v-show="isControlPanelVisible('data')"
            >
              <div id="dataVueShellMount"></div>
            </div>
            <div
              id="controls_panel_opt"
              class="panelHost"
              data-vue-panel="opt"
              :role="controlsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                controlsTabsVisible ? controlTabId('opt') : undefined
              "
              :aria-label="
                controlsTabsVisible
                  ? undefined
                  : panelLabel(visibleControlsItems, 'opt')
              "
              :aria-hidden="!isControlPanelVisible('opt')"
              v-show="isControlPanelVisible('opt')"
            >
              <div id="optVueShellMount"></div>
            </div>
            <div
              id="controls_panel_strategy"
              class="panelHost"
              data-vue-panel="strategy"
              :role="controlsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                controlsTabsVisible ? controlTabId('strategy') : undefined
              "
              :aria-label="
                controlsTabsVisible
                  ? undefined
                  : panelLabel(visibleControlsItems, 'strategy')
              "
              :aria-hidden="!isControlPanelVisible('strategy')"
              v-show="isControlPanelVisible('strategy')"
            >
              <div id="strategyVueShellMount"></div>
            </div>
            <div
              id="controls_panel_advanced"
              class="panelHost"
              data-vue-panel="advanced"
              :role="controlsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                controlsTabsVisible ? controlTabId('advanced') : undefined
              "
              :aria-label="
                controlsTabsVisible
                  ? undefined
                  : panelLabel(visibleControlsItems, 'advanced')
              "
              :aria-hidden="!isControlPanelVisible('advanced')"
              v-show="isControlPanelVisible('advanced')"
            >
              <div id="advancedVueShellMount"></div>
            </div>
          </section>

          <section
            v-show="resultsColumnVisible"
            class="workbenchLane resultsLane"
            aria-label="Results pane"
            tabindex="0"
          >
            <div
              v-if="resultsTabsVisible"
              id="resultsTabBar"
              class="localTabs"
              role="tablist"
              aria-label="Results panels"
            >
              <button
                v-for="item in visibleResultsItems"
                :id="item.tabId"
                :key="item.id"
                type="button"
                class="tabBtn"
                :class="{ active: item.selected }"
                role="tab"
                :aria-selected="item.selected ? 'true' : 'false'"
                :aria-controls="item.panelId"
                :tabindex="item.selected ? 0 : -1"
                :data-tab="item.id"
                :data-qa-action="item.actionId"
                @click="selectResultsTab(item.id)"
                @keydown="onResultsKeydown($event, item)"
              >
                {{ item.label }}
              </button>
            </div>

            <div
              id="results_panel_overview"
              class="panelHost"
              data-vue-panel="overview"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('overview') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'overview')
              "
              :aria-hidden="!isResultsPanelVisible('overview')"
              v-show="isResultsPanelVisible('overview')"
            >
              <div id="overviewVueShellMount"></div>
            </div>
            <div
              id="results_panel_top"
              class="panelHost"
              data-vue-panel="top"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('top') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'top')
              "
              :aria-hidden="!isResultsPanelVisible('top')"
              v-show="isResultsPanelVisible('top')"
            >
              <div id="topVueShellMount"></div>
            </div>
            <div
              id="results_panel_pareto"
              class="panelHost"
              data-vue-panel="pareto"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('pareto') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'pareto')
              "
              :aria-hidden="!isResultsPanelVisible('pareto')"
              v-show="isResultsPanelVisible('pareto')"
            >
              <div id="paretoVueShellMount"></div>
            </div>
            <div
              id="results_panel_validate"
              class="panelHost"
              data-vue-panel="validate"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('validate') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'validate')
              "
              :aria-hidden="!isResultsPanelVisible('validate')"
              v-show="isResultsPanelVisible('validate')"
            >
              <div id="validationVueShellMount"></div>
            </div>
            <div
              id="results_panel_drill"
              class="panelHost"
              data-vue-panel="drill"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('drill') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'drill')
              "
              :aria-hidden="!isResultsPanelVisible('drill')"
              v-show="isResultsPanelVisible('drill')"
            >
              <div id="drillVueShellMount"></div>
            </div>
            <div
              id="results_panel_heatmap"
              class="panelHost"
              data-vue-panel="heatmap"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('heatmap') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'heatmap')
              "
              :aria-hidden="!isResultsPanelVisible('heatmap')"
              v-show="isResultsPanelVisible('heatmap')"
            >
              <div id="heatmapVueShellMount"></div>
            </div>
            <div
              id="results_panel_history"
              class="panelHost"
              data-vue-panel="history"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('history') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'history')
              "
              :aria-hidden="!isResultsPanelVisible('history')"
              v-show="isResultsPanelVisible('history')"
            >
              <div id="historyVueShellMount"></div>
            </div>
            <div
              id="results_panel_meta"
              class="panelHost"
              data-vue-panel="meta"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('meta') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'meta')
              "
              :aria-hidden="!isResultsPanelVisible('meta')"
              v-show="isResultsPanelVisible('meta')"
            >
              <div id="metaVueShellMount"></div>
            </div>
            <div
              id="results_panel_debug"
              class="panelHost"
              data-vue-panel="debug"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('debug') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'debug')
              "
              :aria-hidden="!isResultsPanelVisible('debug')"
              v-show="isResultsPanelVisible('debug')"
            >
              <div id="debugVueShellMount"></div>
            </div>
            <div
              id="results_panel_options"
              class="panelHost"
              data-vue-panel="options"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('options') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'options')
              "
              :aria-hidden="!isResultsPanelVisible('options')"
              v-show="isResultsPanelVisible('options')"
            >
              <div id="optionsVueShellMount"></div>
            </div>
            <div
              id="results_panel_final"
              class="panelHost"
              data-vue-panel="final"
              :role="resultsTabsVisible ? 'tabpanel' : 'region'"
              :aria-labelledby="
                resultsTabsVisible ? resultTabId('final') : undefined
              "
              :aria-label="
                resultsTabsVisible
                  ? undefined
                  : panelLabel(visibleResultsItems, 'final')
              "
              :aria-hidden="!isResultsPanelVisible('final')"
              v-show="isResultsPanelVisible('final')"
            >
              <div id="finalReviewVueShellMount"></div>
            </div>
          </section>
        </section>

        <section
          v-show="liveWorkspaceActive"
          id="liveShellGrid"
          aria-label="Live workspace shell"
        >
          <section
            class="liveSupportPane"
            aria-label="Current live support envelope"
          >
            <div>
              <span class="eyebrow">Current live support envelope</span>
              <h2>Live Operations</h2>
            </div>
            <div class="liveSupportGrid">
              <article>
                <span>Strategies</span>
                <strong class="mono" :title="liveSupportMatrixStatus">{{
                  liveSupportStrategiesLabel
                }}</strong>
              </article>
              <article>
                <span>Adapters</span>
                <strong class="mono" :title="liveSupportMatrixStatus">{{
                  liveSupportAdaptersLabel
                }}</strong>
              </article>
              <article>
                <span>Broker</span>
                <strong class="mono">Alpaca REST</strong>
              </article>
              <article>
                <span>Feed</span>
                <strong class="mono">Polygon stocks</strong>
              </article>
              <article>
                <span>Limits</span>
                <strong class="mono"
                  >One symbol · one runner · one stream</strong
                >
              </article>
            </div>
          </section>
          <div id="executionShellPane">
            <div id="executionVueShellMount"></div>
          </div>
        </section>

        <section
          v-show="portfolioWorkspaceActive"
          id="portfolioShellGrid"
          aria-label="Portfolio research workspace shell"
        >
          <div id="portfolioVueShellMount"></div>
        </section>

        <section
          v-show="settingsWorkspaceActive"
          id="settingsShellGrid"
          aria-label="Settings workspace shell"
        >
          <div id="settingsVueShellMount"></div>
        </section>
      </main>
    </div>

    <div
      v-if="compactInspector && compactInspectorOpen"
      class="compactInspectorBackdrop"
      aria-hidden="true"
      @click="closeCompactInspector"
    ></div>

    <WorkbenchInspector
      id="workbenchInspector"
      :state="inspectorState"
      :collapsed="inspectorCollapsed"
      :compact="compactInspector"
      :compact-open="compactInspectorOpen"
      @toggle="toggleInspector"
      @close-compact="closeCompactInspector"
      @select-mode="selectInspectorMode"
      @run-action="runInspectorAction"
    />
    <WorkbenchActionBar />
  </section>
</template>

<style scoped>
.workbenchRoot {
  position: fixed;
  inset: var(--desktop-titlebar-h, 0px) 0 0 0;
  z-index: 1200;
  color: var(--text);
  background: var(--app-bg);
  display: grid;
  grid-template-columns: 72px minmax(0, 1fr) minmax(286px, 330px);
  grid-template-rows: minmax(0, 1fr) auto;
  pointer-events: auto;
  overflow: hidden;
}

.workbenchRoot.inspectorCollapsed {
  grid-template-columns: 72px minmax(0, 1fr) 48px;
}

.workbenchRoot :deep(.workbenchRail) {
  grid-column: 1;
  grid-row: 1 / 3;
}

.workbenchContent {
  grid-column: 2;
  grid-row: 1;
  min-width: 0;
  min-height: 0;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
}

.workbenchRoot :deep(.workbenchInspector) {
  grid-column: 3;
  grid-row: 1;
}

.workbenchRoot :deep(.workbenchActionBar) {
  grid-column: 2 / 4;
  grid-row: 2;
}

.workbenchHeader {
  min-width: 0;
  border-bottom: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  background: color-mix(in oklab, var(--header-bg) 92%, var(--surface-shell));
  box-shadow: 0 8px 18px
    color-mix(in srgb, rgb(var(--shadow-overflow-rgb) / 0.12) 72%, transparent);
  padding: 0.58rem 0.75rem 0.52rem;
  display: grid;
  gap: 0.42rem;
}

.titleRow,
.toolbarRow {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 0.55rem;
}

.titleRow {
  justify-content: space-between;
}

.titleCopy {
  min-width: 0;
  display: grid;
  gap: 0.05rem;
}

.titleCopy h1 {
  margin: 0;
  font-size: 1rem;
  line-height: 1.16;
  letter-spacing: 0;
}

.titleCopy p {
  margin: 0;
  max-width: min(76ch, 100%);
  color: var(--muted-strong);
  font-size: 0.76rem;
  line-height: 1.32;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.eyebrow {
  font-size: var(--text-xs);
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--muted);
}

.headerContext {
  min-width: 0;
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.28rem;
}

.contextChip {
  max-width: 24ch;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 92%, var(--panel));
  color: var(--muted-strong);
  padding: 0.22rem 0.46rem;
  font-size: var(--text-xs);
  line-height: 1.2;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.contextChip.blocked {
  color: var(--warning-ink);
  background: color-mix(
    in oklab,
    var(--surface-warning) 58%,
    var(--surface-working)
  );
}

.toolbarRow {
  flex-wrap: wrap;
}

.segmentedBar,
.stageBar,
.localTabs {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-sunken) 94%, var(--panel));
  display: flex;
  flex-wrap: wrap;
  gap: 0.22rem;
  padding: 0.18rem;
}

.segmentedBar {
  flex: 0 1 auto;
}

.stageBar {
  width: 100%;
}

.tabBtn,
.stageButton {
  min-width: 0;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--muted);
  font-weight: 650;
  font-size: var(--text-xs);
  line-height: 1.16;
  padding: 0.32rem 0.52rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.modeBtn {
  min-width: 6.4rem;
}

.stageButton {
  flex: 1 1 min(8rem, 100%);
}

.tabBtn:hover:not(:disabled),
.stageButton:hover:not(:disabled) {
  background: color-mix(in oklab, var(--surface-working) 88%, var(--panel));
  border-color: color-mix(in oklab, var(--border) 82%, transparent);
  transform: none;
}

.tabBtn.active,
.stageButton.active {
  color: var(--text);
  border-color: color-mix(in oklab, var(--panel-focus-edge) 76%, var(--accent));
  background: color-mix(
    in oklab,
    var(--selection-fill) 86%,
    var(--surface-working)
  );
  box-shadow: inset 0 -2px 0 var(--selection-rail);
}

.taskLauncher,
.advancedDrawerButton,
.compactInspectorButton {
  margin-left: auto;
  min-width: 0;
  min-height: 34px;
  display: inline-flex;
  align-items: center;
  gap: 0.38rem;
  border-radius: 8px;
  padding: 0.34rem 0.54rem;
  color: var(--muted-strong);
}

.advancedDrawerButton,
.compactInspectorButton {
  margin-left: 0;
}

.taskLauncher svg,
.advancedDrawerButton svg,
.compactInspectorButton svg,
.iconOnlyButton svg {
  width: 1rem;
  height: 1rem;
}

.taskLauncher.active,
.taskLauncher.open,
.advancedDrawerButton.open,
.compactInspectorButton.open {
  color: var(--text);
  border-color: color-mix(in oklab, var(--panel-focus-edge) 72%, var(--border));
  background: color-mix(
    in oklab,
    var(--selection-fill) 84%,
    var(--surface-working)
  );
}

.workbenchContent > .workbenchMain {
  position: relative;
  min-width: 0;
  min-height: 0;
  padding: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  grid-template-columns: none;
  gap: 0;
  align-items: stretch;
}

.advancedDrawer {
  position: absolute;
  z-index: 20;
  top: 0.62rem;
  right: 0.65rem;
  width: min(430px, calc(100% - 1.3rem));
  max-height: calc(100% - 1.24rem);
  overflow: auto;
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 58%, var(--border));
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-shell) 96%, var(--panel));
  box-shadow: var(--shadow-lg);
  padding: 0.72rem;
  display: grid;
  gap: 0.58rem;
}

.advancedDrawerHeader {
  min-width: 0;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}

.advancedDrawerHeader h2 {
  margin: 0.08rem 0 0;
  font-size: 0.98rem;
  line-height: 1.18;
}

.advancedDrawerHeader p {
  margin: 0.26rem 0 0;
  max-width: 46ch;
  color: var(--muted-strong);
  font-size: var(--text-xs);
  line-height: 1.34;
}

.iconOnlyButton {
  min-width: 34px;
  min-height: 34px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  color: var(--muted-strong);
}

.advancedDrawerGrid {
  display: grid;
  gap: 0.44rem;
}

.advancedDrawerAction {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 84%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-working) 94%, var(--panel));
  color: var(--text);
  padding: 0.54rem 0.58rem;
  display: grid;
  gap: 0.12rem;
  text-align: left;
}

.advancedDrawerAction:hover {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 62%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 82%, var(--surface-working));
  transform: none;
}

.advancedDrawerAction span {
  font-size: var(--text-xs);
  font-weight: 760;
  line-height: 1.2;
}

.advancedDrawerAction strong {
  min-width: 0;
  color: var(--muted-strong);
  font-size: var(--text-xs);
  line-height: 1.2;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.workbenchPaneGrid {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(340px, 0.42fr) minmax(0, 1fr);
  gap: 0.65rem;
  padding: 0.65rem;
}

.workbenchPaneGrid.singleColumn {
  grid-template-columns: minmax(0, 1fr);
}

.workbenchLane {
  min-width: 0;
  min-height: 0;
  overflow: auto;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--panel) 98%, var(--surface-working));
  box-shadow: var(--shadow-sm);
  padding: 0.62rem;
  display: grid;
  align-content: start;
  gap: 0.56rem;
}

.setupLane {
  background: linear-gradient(
    180deg,
    color-mix(in oklab, var(--surface-shell) 86%, var(--panel)),
    color-mix(in oklab, var(--panel) 98%, var(--surface-working))
  );
}

.resultsLane {
  background: linear-gradient(
    180deg,
    color-mix(in oklab, var(--surface-working) 96%, var(--panel)),
    color-mix(in oklab, var(--panel) 98%, var(--surface-inline))
  );
}

.localTabs {
  position: sticky;
  top: 0;
  z-index: 5;
  margin: -0.08rem -0.08rem 0.04rem;
  background: color-mix(
    in oklab,
    var(--surface-sticky) 94%,
    var(--panel-footer)
  );
}

.panelHost {
  min-width: 0;
  min-height: 0;
}

#settingsShellGrid,
#portfolioShellGrid,
#liveShellGrid {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  overflow: auto;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--panel) 98%, var(--surface));
  margin: 0.65rem;
  padding: 0.68rem;
  display: grid;
  gap: 0.8rem;
  align-content: start;
  box-shadow: var(--shadow-sm);
}

.liveSupportPane {
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 8px;
  padding: 0.78rem 0.84rem;
  display: grid;
  gap: 0.55rem;
  background: color-mix(
    in oklab,
    var(--surface-working) 94%,
    var(--surface-shell)
  );
}

.liveSupportPane h2 {
  margin: 0.08rem 0 0;
  font-size: 0.96rem;
  line-height: 1.2;
}

.liveSupportGrid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.5rem;
}

.liveSupportGrid article {
  min-width: 0;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface) 95%, var(--panel));
  padding: 0.58rem 0.62rem;
  display: grid;
  gap: 0.12rem;
}

.liveSupportGrid span {
  font-size: var(--text-xs);
  letter-spacing: 0.08em;
  color: var(--muted);
  text-transform: uppercase;
}

.liveSupportGrid strong {
  min-width: 0;
  font-size: var(--text-xs);
  line-height: 1.25;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

#executionShellPane {
  border-top: 1px dashed color-mix(in oklab, var(--border) 88%, transparent);
  padding-top: 0.85rem;
}

.srOnly {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1180px) {
  .workbenchRoot,
  .workbenchRoot.inspectorCollapsed {
    grid-template-columns: 72px minmax(0, 1fr);
  }

  .workbenchRoot :deep(.workbenchActionBar) {
    grid-column: 2;
  }

  .workbenchRoot :deep(.workbenchInspector) {
    grid-column: auto;
    grid-row: auto;
  }

  .compactInspectorBackdrop {
    position: fixed;
    inset: var(--desktop-titlebar-h, 0px) 0 0;
    z-index: 1250;
    background: rgb(var(--shadow-overflow-rgb) / 0.32);
  }
}

@media (max-width: 920px) {
  .headerContext {
    display: none;
  }

  .workbenchPaneGrid {
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: repeat(2, minmax(180px, 1fr));
  }

  .liveSupportGrid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 780px) {
  .workbenchRoot,
  .workbenchRoot.inspectorCollapsed {
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr) auto;
  }

  .workbenchRoot :deep(.workbenchRail) {
    grid-column: 1;
    grid-row: 1;
  }

  .workbenchContent {
    grid-column: 1;
    grid-row: 2;
  }

  .workbenchContent > .workbenchMain {
    overflow-x: hidden;
    overflow-y: auto;
  }

  .workbenchPaneGrid {
    flex: 0 0 auto;
    height: calc(360px + 1.5rem);
  }

  .workbenchRoot :deep(.workbenchActionBar) {
    grid-column: 1;
    grid-row: 3;
  }

  .workbenchHeader {
    padding: 0.45rem 0.5rem;
  }

  .titleRow {
    align-items: flex-start;
  }

  .titleCopy p {
    white-space: normal;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .toolbarRow,
  .stageBar {
    flex-wrap: nowrap;
    overflow-x: auto;
    scrollbar-width: thin;
  }

  .segmentedBar,
  .stageBar,
  .localTabs {
    flex-wrap: nowrap;
  }

  .tabBtn {
    flex: 0 0 auto;
  }

  .stageButton {
    flex: 1 1 0;
    padding-inline: 0.34rem;
  }

  .taskLauncher {
    margin-left: 0;
  }

  .workbenchPaneGrid,
  #settingsShellGrid,
  #portfolioShellGrid,
  #liveShellGrid {
    margin: 0.5rem;
    padding: 0.5rem;
  }

  .workbenchPaneGrid {
    margin: 0;
    gap: 0.5rem;
  }

  .resultsLane {
    scroll-margin-block-end: 2px;
  }

  .liveSupportGrid {
    grid-template-columns: 1fr;
  }
}
</style>
