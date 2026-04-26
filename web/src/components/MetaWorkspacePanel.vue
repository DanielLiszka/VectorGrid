<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import { storeToRefs } from "pinia";
import { type Cell, type Header } from "@tanstack/vue-table";
import {
  useHeadlessMatrixTable,
  type MatrixColumnMeta,
} from "../composables/useHeadlessMatrixTable";
import { usePersistedMatrixColumnState } from "../composables/usePersistedMatrixColumnState";
import { useShellMount } from "../composables/useShellMount";
import { useUiReadiness } from "../composables/useUiReadiness";
import { useVirtualTableRows } from "../composables/useVirtualTableRows";
import { useQueuedInputWriter } from "../composables/useQueuedInputWriter";
import AppVisualIcon from "./AppVisualIcon.vue";
import ConfirmationDialog from "./ConfirmationDialog.vue";
import ContextHelpTip from "./ContextHelpTip.vue";
import ContextActionMenu from "./ContextActionMenu.vue";
import AppDataTableShell from "./ui/AppDataTableShell.vue";
import { useAnalysisStore } from "../stores/analysisStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { META_RESULTS_TABLE_STATE_KEY } from "../utils/core/storageKeys";
import {
  artifactBundleIcon,
  emptyArtifactLibraryIllustration,
  emptyCompareWorkspaceIllustration,
  emptyResultsWorkspaceIllustration,
} from "../utils/appSvgAssets";
import { artifactActionIcon, type AppIcon } from "../utils/iconography";
import { formatUiTimeFromUnixMs } from "../utils/uiTime";

interface OptionItem {
  value: string;
  label: string;
  disabled: boolean;
}

interface MetaColumn {
  label: string;
  alignLeft: boolean;
}

interface MetaCell {
  text: string;
  className: string;
  title: string;
}

interface MetaRow {
  cells: MetaCell[];
}

interface BatchDatasetItem {
  path: string;
  label: string;
  meta: string;
  checked: boolean;
  disabled: boolean;
}

interface ContextActionMenuItem {
  key: string;
  label: string;
  icon?: AppIcon;
  disabled: boolean;
  destructive?: boolean;
  separatorBefore?: boolean;
}

function hasMeaningfulPanelText(value: unknown): boolean {
  const text = String(value ?? "").trim();
  return text !== "" && text !== "-" && text !== "—";
}

const aggregateTabItems = [
  { key: "meta", label: "Meta Selection" },
  { key: "batch", label: "Batch Generation" },
  { key: "saved", label: "Saved Meta" },
  { key: "compare", label: "Compare" },
] as const;

type AggregateTabKey = (typeof aggregateTabItems)[number]["key"];
type SavedArtifactFamilyKey = "meta" | "stage5" | "policyEval";

const savedArtifactFamilyItems = [
  { key: "meta", label: "Saved Meta" },
  { key: "stage5", label: "Saved Stage 5" },
  { key: "policyEval", label: "Saved Policy Eval" },
] as const;

const analysisStore = useAnalysisStore();
const workflowStore = useWorkflowStore();
const { metaCompareStage, metaStage } = useUiReadiness();
const loadSavedArtifactIcon = artifactActionIcon("load");
const {
  metaStrategyKind,
  metaStrategyOptions,
  metaTimeframe,
  metaRequiredTag,
  metaMinSamples,
  metaTopK,
  metaHoldoutTag,
  metaEvalHoldout,
  metaSaveHoldoutRows,
  metaHoldoutMode,
  metaHoldoutReps,
  metaHoldoutEvalTopN,
  metaHoldoutEvalRankBy,
  metaHoldoutExploreN,
  metaGroupingMode,
  metaClusterIntBucket,
  metaClusterFloatBucket,
  metaHoldoutNote,
  aggregateSummarySavedRuns,
  aggregateSummarySavedMeta,
  aggregateSummaryLatestGroups,
  aggregateSummaryLatestArtifacts,
  aggregateSummaryLatestHoldout,
  aggregateSummaryLatestRuntime,
  metaMeta,
  metaOutputPreview,
  metaTableColumns,
  metaTableRows,
  metaCompareOptions,
  metaCompareSelected,
  metaCompareSummary,
  batchDatasetSearch,
  batchTags,
  batchHoldoutPct,
  batchHoldoutTag,
  batchExecutionMode,
  batchPreloadPolicy,
  batchAutoMeta,
  batchAutoStage5,
  batchSkipDone,
  batchDatasets,
  batchSelectedCount,
  batchTotalCount,
  batchMeta,
  batchOutputPreview,
  canRunBatch,
  canBatchStop,
  canBatchRefresh,
  canBatchSelectAll,
  canBatchSelectNone,
  canBatchRetryFailed,
  canBatchResumeIncomplete,
  canBatchRevealSummary,
  metaSavedSearch,
  metaSavedOptions,
  metaSavedSelected,
  metaSavedTagsDraft,
  metaSavedMeta,
  canSaveMetaSavedTags,
  canRefreshMetaSaved,
  canLoadMetaSaved,
  canOpenMetaSavedReport,
  canRevealMetaSaved,
  canDeleteMetaSaved,
  stage4SavedSearch,
  stage4SavedOptions,
  stage4SavedSelected,
  stage4SavedTagsDraft,
  stage4SavedMeta,
  canSaveStage4SavedTags,
  canRefreshStage4Saved,
  canLoadStage4Saved,
  canRevealStage4Saved,
  canDeleteStage4Saved,
  policyEvalSavedSearch,
  policyEvalSavedOptions,
  policyEvalSavedSelected,
  policyEvalSavedTagsDraft,
  policyEvalSavedMeta,
  canSavePolicyEvalSavedTags,
  canRefreshPolicyEvalSaved,
  canLoadPolicyEvalSaved,
  canRevealPolicyEvalSaved,
  canDeletePolicyEvalSaved,
  metaUseMlConfidence,
  metaMlAutopilot,
  canMetaMlRefresh,
  canMetaMlTrain,
  canMetaMlApply,
  canMetaMlDelete,
  metaMlStatus,
  metaStage4TopK,
  metaStage4MinConf,
  metaStage4MetaWeight,
  metaStage4MlWeight,
  metaStage4UncertaintyWeight,
  metaStage4AcceptConf,
  metaStage4ReviewConf,
  metaStage4MaxAcceptUncertainty,
  metaStage4AutoTrain,
  metaStage4RequireHealthy,
  metaStage4FallbackToMeta,
  metaStage4TrainingHistoryPolicy,
  canRunMetaStage4,
  canRunMetaPolicyEval,
  canRevealMetaStage4,
  metaStage4Status,
  metaStage4OutputPreview,
  metaPolicyEvalStatus,
  metaPolicyEvalOutputPreview,
  canRunMeta,
  canOpenMetaReport,
  canRevealMetaFiles,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore);
const { aggregateTab, resultsTab } = storeToRefs(workflowStore);

const { mountReady } = useShellMount("#metaVueShellMount", {
  paneSelector: '[data-vue-panel="meta"]',
  paneDatasetKey: "vueMeta",
});

const strategyDraft = ref("CompositeMo");
const timeframeDraft = ref("");
const requiredTagDraft = ref("");
const minSamplesDraft = ref("30");
const topKDraft = ref("25");
const holdoutTagDraft = ref("holdout");
const holdoutEvalDraft = ref(true);
const holdoutSaveRowsDraft = ref(false);
const holdoutModeDraft = ref("light");
const holdoutRepsDraft = ref("2");
const holdoutEvalTopNDraft = ref("0");
const holdoutEvalRankByDraft = ref("auto");
const holdoutExploreNDraft = ref("0");
const groupingModeDraft = ref("clustered");
const clusterIntBucketDraft = ref("5");
const clusterFloatBucketDraft = ref("0.25");
const compareDraft = ref("");
const batchSearchDraft = ref("");
const batchTagsDraft = ref("");
const batchHoldoutPctDraft = ref("15");
const batchHoldoutTagDraft = ref("holdout");
const batchExecutionModeDraft = ref("full");
const batchPreloadPolicyDraft = ref("host_only");
const batchAutoMetaDraft = ref(true);
const batchAutoStage5Draft = ref(false);
const batchSkipDoneDraft = ref(true);
const metaSavedSearchDraft = ref("");
const metaSavedSelectDraft = ref("");
const metaSavedTagsLocalDraft = ref("");
const stage4SavedSearchDraft = ref("");
const stage4SavedSelectDraft = ref("");
const stage4SavedTagsLocalDraft = ref("");
const policyEvalSavedSearchDraft = ref("");
const policyEvalSavedSelectDraft = ref("");
const policyEvalSavedTagsLocalDraft = ref("");
const metaUseMlConfidenceDraft = ref(false);
const metaMlAutopilotDraft = ref(true);
const metaStage4TopKDraft = ref("10");
const metaStage4MinConfDraft = ref("0.55");
const metaStage4MetaWeightDraft = ref("0.60");
const metaStage4MlWeightDraft = ref("0.35");
const metaStage4UncertaintyWeightDraft = ref("0.05");
const metaStage4AcceptConfDraft = ref("0.70");
const metaStage4ReviewConfDraft = ref("0.55");
const metaStage4MaxAcceptUncertaintyDraft = ref("0.45");
const metaStage4AutoTrainDraft = ref(true);
const metaStage4RequireHealthyDraft = ref(true);
const metaStage4FallbackToMetaDraft = ref(true);
const metaStage4TrainingHistoryPolicyDraft = ref("source_excluded");
const savedArtifactFamily = ref<SavedArtifactFamilyKey>("meta");
const pendingSavedArtifactDeleteFamily = ref<SavedArtifactFamilyKey | null>(
  null,
);

const visible = computed(
  () => String(resultsTab.value || "").trim() === "meta",
);
const activeAggregateTab = computed(
  () => String(aggregateTab.value || "").trim() || "meta",
);
const metaTabActive = computed(() => activeAggregateTab.value === "meta");
const compareTabActive = computed(() => activeAggregateTab.value === "compare");
const showMetaTooling = computed(() => metaStage.value !== "no_data");
const renderedColumns = computed<MetaColumn[]>(
  () => metaTableColumns.value as MetaColumn[],
);
const renderedRows = computed<MetaRow[]>(
  () => metaTableRows.value as MetaRow[],
);
const {
  columnOptions,
  columnVisibility,
  visibleCount: visibleColumnCount,
  hiddenCount: hiddenColumnCount,
  canRestoreDefaults,
  orderedColumns,
  columnWidths,
  toggleColumnVisibility,
  restoreDefaultColumns,
  canHideColumn,
  setColumnWidth,
  resizeColumnWidth,
  clearColumnWidth,
  columnWidthStyle,
} = usePersistedMatrixColumnState(
  renderedColumns,
  META_RESULTS_TABLE_STATE_KEY,
);
const { headerGroups, rowModelRows, visibleLeafColumns } =
  useHeadlessMatrixTable(orderedColumns, renderedRows, { columnVisibility, columnWidths });
const renderedBatchDatasets = computed<BatchDatasetItem[]>(
  () => batchDatasets.value as BatchDatasetItem[],
);
const {
  scrollElementRef: tableScrollElementRef,
  virtualRows,
  topSpacerPx: virtualTopSpacerPx,
  bottomSpacerPx: virtualBottomSpacerPx,
  scrollToTop,
} = useVirtualTableRows(rowModelRows);

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value));

const activeAggregateTabLabel = computed(
  () =>
    aggregateTabItems.find((item) => item.key === activeAggregateTab.value)
      ?.label || "Meta Selection",
);

function pathBasename(value: string): string {
  const normalized = String(value || "")
    .trim()
    .replace(/\\/g, "/");
  if (!normalized) return "";
  const lastSlash = normalized.lastIndexOf("/");
  return lastSlash >= 0 ? normalized.slice(lastSlash + 1) : normalized;
}

const metaStageLabel = computed(() => {
  if (metaStage.value === "no_data") return "Awaiting saved runs";
  if (metaStage.value === "data_loaded_no_run") return "Ready for meta run";
  if (
    compareTabActive.value &&
    metaCompareStage.value === "results_available_no_selection"
  )
    return "Awaiting compare baseline";
  return "Results available";
});

const metaSummaryChips = computed(() => {
  return [
    metaStageLabel.value,
    activeAggregateTabLabel.value,
    String(aggregateSummarySavedRuns.value || "").trim()
      ? `Runs ${aggregateSummarySavedRuns.value}`
      : "",
    String(aggregateSummarySavedMeta.value || "").trim()
      ? `Meta ${aggregateSummarySavedMeta.value}`
      : "",
  ].filter((chip) => String(chip || "").trim().length > 0);
});
const columnVisibilitySummary = computed(
  () => `Columns ${visibleColumnCount.value}/${renderedColumns.value.length}`,
);
const metaTableShellSummary = computed(() => {
  const rowCount = renderedRows.value.length;
  const meta = String(metaMeta.value || "").trim();
  const base = `${rowCount} row${rowCount === 1 ? "" : "s"} · ${columnVisibilitySummary.value}`;
  return meta ? `${base} · ${meta}` : base;
});
const columnVisibilityHint = computed(() => {
  if (hiddenColumnCount.value > 0) {
    return `${hiddenColumnCount.value} hidden columns. Restore defaults to recover the full meta review table.`;
  }
  return "Hide lower-signal grouping columns when you want a tighter meta selection review.";
});

const strategyOptions = computed<OptionItem[]>(() => {
  const options = metaStrategyOptions.value as OptionItem[];
  if (options.length > 0) return options;
  const value =
    String(metaStrategyKind.value || "CompositeMo").trim() || "CompositeMo";
  return [{ value, label: value, disabled: false }];
});

const compareOptions = computed<OptionItem[]>(() => {
  const options = metaCompareOptions.value as OptionItem[];
  if (options.length > 0) return options;
  return [{ value: "", label: "Select saved meta baseline", disabled: false }];
});

const selectedCompareOption = computed(() => {
  const selectedValue = String(metaCompareSelected.value || "").trim();
  if (!selectedValue) return null;
  return (
    compareOptions.value.find((option) => option.value === selectedValue) ??
    null
  );
});

const selectedCompareLabel = computed(() => {
  const label = String(selectedCompareOption.value?.label || "").trim();
  return label || "No baseline selected";
});

const selectedCompareFile = computed(() => {
  return pathBasename(String(metaCompareSelected.value || "").trim()) || "—";
});

const compareSummaryLead = computed(() => {
  if (!String(metaCompareSelected.value || "").trim()) {
    return "Compare always uses a Saved Meta artifact as the baseline. Stage 5 and policy-evaluation artifacts stay in the Saved Aggregate library.";
  }
  return "Current Meta Selection output stays on the working side while the selected Saved Meta artifact anchors the baseline summary below.";
});

const metaSavedSelectOptions = computed<OptionItem[]>(() => {
  const options = metaSavedOptions.value as OptionItem[];
  if (options.length > 0) return options;
  return [{ value: "", label: "No saved meta", disabled: false }];
});

const stage4SavedSelectOptions = computed<OptionItem[]>(() => {
  const options = stage4SavedOptions.value as OptionItem[];
  if (options.length > 0) return options;
  return [{ value: "", label: "No saved Stage 5", disabled: false }];
});

const policyEvalSavedSelectOptions = computed<OptionItem[]>(() => {
  const options = policyEvalSavedOptions.value as OptionItem[];
  if (options.length > 0) return options;
  return [{ value: "", label: "No saved policy evaluation", disabled: false }];
});

interface SavedArtifactFamilyConfig {
  key: SavedArtifactFamilyKey;
  title: string;
  lead: string;
  selectLabel: string;
  searchFieldId: string;
  selectFieldId: string;
  tagsFieldId: string;
  searchDraft: string;
  selectedValue: string;
  tagsDraft: string;
  options: OptionItem[];
  meta: string;
  canSaveTags: boolean;
  canLoad: boolean;
  secondaryActions: ContextActionMenuItem[];
  actionMenuLabel: string;
}

const holdoutModeOptions: OptionItem[] = [
  { value: "light", label: "Light (fast)", disabled: false },
  {
    value: "heavy",
    label: "Heavy (walk-forward + optional perturb/MC)",
    disabled: false,
  },
];

const holdoutRepsOptions: OptionItem[] = [
  { value: "1", label: "1 (fastest)", disabled: false },
  { value: "2", label: "2 (recommended)", disabled: false },
  { value: "3", label: "3 (more robust)", disabled: false },
];

const holdoutRankByOptions: OptionItem[] = [
  { value: "auto", label: "Auto (ML if healthy else meta)", disabled: false },
  { value: "meta", label: "Meta score", disabled: false },
  { value: "ml", label: "ML confidence", disabled: false },
];

const groupingModeOptions: OptionItem[] = [
  { value: "clustered", label: "Clustered (recommended)", disabled: false },
  { value: "exact", label: "Exact (strict)", disabled: false },
];

const batchPreloadPolicyOptions: OptionItem[] = [
  { value: "host_only", label: "Host preload (recommended)", disabled: false },
  { value: "disabled", label: "Disabled", disabled: false },
  {
    value: "host_plus_doublema_vram",
    label: "Host + DoubleMa VRAM staging (experimental)",
    disabled: false,
  },
];

const batchExecutionModeOptions: OptionItem[] = [
  { value: "full", label: "Full campaign", disabled: false },
  { value: "optimize_only", label: "Optimize only", disabled: false },
  { value: "validate_only", label: "Validate only", disabled: false },
];

const batchRunButtonLabel = computed(() => {
  switch (String(batchExecutionModeDraft.value || "full").trim()) {
    case "optimize_only":
      return "Run optimize-only";
    case "validate_only":
      return "Run validate-only";
    default:
      return "Run full campaign";
  }
});

function optionLabel(
  options: OptionItem[],
  value: string,
  fallback: string,
): string {
  const selected = options.find((option) => option.value === value);
  return String(selected?.label || fallback).trim() || fallback;
}

const batchExecutionModeLabel = computed(() =>
  optionLabel(
    batchExecutionModeOptions,
    String(batchExecutionModeDraft.value || "full").trim(),
    "Full campaign",
  ),
);

const batchPreloadPolicyLabel = computed(() =>
  optionLabel(
    batchPreloadPolicyOptions,
    String(batchPreloadPolicyDraft.value || "host_only").trim(),
    "Host preload (recommended)",
  ),
);

const batchAutomationLabel = computed(() => {
  if (!batchAutoMetaDraft.value) return "Manual follow-up";
  if (batchAutoStage5Draft.value) return "Meta + Stage 5";
  return "Meta only";
});

const batchSelectionLabel = computed(
  () =>
    `${Math.max(0, Number(batchSelectedCount.value || 0))}/${Math.max(0, Number(batchTotalCount.value || 0))} selected`,
);

const batchHoldoutContractSummary = computed(() => {
  return [
    `${parseDraftInt(batchHoldoutPctDraft.value, 15, 0)}% holdout`,
    `tag ${String(batchHoldoutTagDraft.value || "holdout").trim() || "holdout"}`,
    batchSkipDoneDraft.value ? "skip completed on" : "skip completed off",
  ].join(" • ");
});

const batchRegistryContractSummary = computed(() => {
  const executionMode =
    String(batchExecutionModeDraft.value || "full").trim() || "full";
  const modeDetail =
    executionMode === "optimize_only"
      ? "meta and Stage 5 disabled"
      : executionMode === "validate_only"
        ? "reuses saved optimization artifacts"
        : "full campaign with optional follow-up";
  return [
    `tags ${summarizeDraftTags(batchTagsDraft.value)}`,
    "same-tf selection unless Data overrides",
    modeDetail,
  ].join(" • ");
});

const batchRecoveryContractSummary = computed(() => {
  const summaryState = canBatchRevealSummary.value
    ? "saved summary available"
    : "saved summary required";
  return [
    summaryState,
    "required for retry/resume",
    "retry restores the saved dataset set",
    "resume needs embedded request",
    "recovery forces skip completed",
  ].join(" • ");
});

const metaMlStatusLabel = computed(() => {
  return hasMeaningfulPanelText(metaMlStatus.value)
    ? String(metaMlStatus.value).trim()
    : "No ML model status loaded yet.";
});

const metaStage5StatusLabel = computed(() => {
  return hasMeaningfulPanelText(metaStage4Status.value)
    ? String(metaStage4Status.value).trim()
    : "No live Stage 5 output yet.";
});

const metaPolicyEvalStatusLabel = computed(() => {
  return hasMeaningfulPanelText(metaPolicyEvalStatus.value)
    ? String(metaPolicyEvalStatus.value).trim()
    : "No live policy evaluation output yet.";
});

const metaMlScopeContractSummary = computed(() => {
  const strategy =
    String(
      strategyDraft.value || metaStrategyKind.value || "CompositeMo",
    ).trim() || "CompositeMo";
  const timeframe = String(
    timeframeDraft.value || metaTimeframe.value || "",
  ).trim();
  if (!timeframe) {
    return `${strategy} / timeframe required • scope resolves from latest meta result first, then store fallback`;
  }
  return `${strategy} / ${timeframe} • model ml_confidence_${strategy}_${timeframe}.json • latest meta result scope first`;
});

const metaMlRowsContractSummary = computed(() => {
  const status = String(metaMlStatus.value || "").trim();
  const counts = extractAvailableCounts(status);
  const trainedRows = extractStatusField(status, "trained_rows") || "—";
  const target = extractStatusField(status, "target") || "auto";
  return `available meta ${counts.meta} • rows ${counts.rows} • trained ${trainedRows} • target ${target}`;
});

const metaMlLabelContractSummary = computed(() => {
  const status = String(metaMlStatus.value || "").trim();
  const label = String(status.match(/label:\s*([^·]+)/)?.[1] || "").trim();
  return (
    label || "pass>=0.60, p10>=0.000, datasets>=30, dd cap off unless override"
  );
});

const metaMlFamilyContractSummary = computed(() => {
  const resolvedKind = extractStatusField(
    String(metaMlStatus.value || "").trim(),
    "model_kind",
  );
  return `family Auto (backend owned) • resolved kind ${resolvedKind || "Logistic or GbdtStump"} • no shipped UI family picker`;
});

const metaMlHealthContractSummary = computed(() => {
  const status = String(metaMlStatus.value || "").trim();
  const state = extractStatusField(status, "state") || "missing";
  const freshness = status.includes("stale")
    ? "stale"
    : status.includes("fresh")
      ? "fresh"
      : "freshness unknown";
  const retrain = status.includes("retrain recommended")
    ? "retrain recommended"
    : "retrain off";
  const legacy = status.includes("legacy") ? "legacy" : "modern or missing";
  return `state ${state} • ${freshness} • ${retrain} • ${legacy}`;
});

const metaMlApplyContractSummary = computed(() => {
  return [
    metaMlAutopilotDraft.value ? "autopilot on" : "autopilot off",
    "Apply mutates current meta only",
    "Stage 5 saves rerank artifact",
    "Policy Eval saves offline comparison",
  ].join(" • ");
});

const metaStage5RequestContractSummary = computed(() => {
  const strategy =
    String(
      strategyDraft.value || metaStrategyKind.value || "CompositeMo",
    ).trim() || "CompositeMo";
  const timeframe = String(
    timeframeDraft.value || metaTimeframe.value || "",
  ).trim();
  const scope = timeframe
    ? `${strategy} / ${timeframe}`
    : `${strategy} / timeframe required`;
  return `${scope} • source latest Meta artifact first, then Saved Meta selection • Stage 5 auto-build reuses current controls`;
});

const metaStage5SelectionContractSummary = computed(() => {
  return "rank uses meta/ml/unc weights • min_ml filters first • fallback only if shortlist empties and toggle stays on • decision buckets assign after ranking";
});

const metaStage5ExecutionContractSummary = computed(() => {
  const stage5File = extractStatusField(
    String(metaStage4Status.value || "").trim(),
    "file",
  );
  return [
    "run_meta_stage5_ml_confidence via Run Stage 5",
    "Run Stage 5 returns the durable saved replay artifact used by load/reveal/delete",
    stage5File
      ? `saved Stage 5 JSON artifact ${stage5File}`
      : "saved Stage 5 JSON artifact returned on completion",
  ].join(" • ");
});

const metaPolicyEvalContractSummary = computed(() => {
  return "offline scoped history over saved Meta artifacts • request reuses current Stage 5 thresholds and weights • compares meta_only / ml_only / stage5_blended • JSON artifact replay only";
});

const metaSavedReplayContractSummary = computed(() => {
  return "Saved Stage 5 UI is backed by durable replay state • load/reveal/delete operate on saved JSON artifacts • current live Stage 5 preview is reused on load";
});

const metaMlStatusSurfaceSummary = computed(() => {
  return "metaMlStatus is formatted model status • statusLine is action feedback • buttons gate on cached backend status and scope completeness • autopilot reacts after meta runs";
});

function parseDraftInt(value: string, fallback: number, min = 0): number {
  const parsed = Number(String(value || "").trim());
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(min, Math.floor(parsed));
}

function parseDraftFloat(value: string, fallback: number, min = 0): number {
  const parsed = Number(String(value || "").trim());
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(min, parsed);
}

function summarizeDraftTags(value: string): string {
  const seen = new Set<string>();
  const out: string[] = [];
  const parts = String(value || "")
    .split(/[,;\n]+/g)
    .flatMap((part) =>
      String(part || "")
        .trim()
        .split(/\s+/g),
    );

  for (const part of parts) {
    const next = String(part || "").trim();
    if (!next) continue;
    const key = next.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(next);
  }

  return out.length > 0 ? out.join(", ") : "none";
}

function extractStatusField(text: string, key: string): string {
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = String(text || "").match(new RegExp(`${escapedKey}=([^·]+)`));
  return String(match?.[1] || "").trim();
}

function extractAvailableCounts(text: string): { meta: string; rows: string } {
  const match = String(text || "").match(
    /available:\s*meta=(\d+),\s*rows=(\d+)/i,
  );
  return {
    meta: String(match?.[1] || "—").trim() || "—",
    rows: String(match?.[2] || "—").trim() || "—",
  };
}

const stage3RequestScopeSummary = computed(() => {
  const parts = [
    String(strategyDraft.value || "CompositeMo").trim() || "CompositeMo",
  ];
  const timeframe = String(timeframeDraft.value || "").trim();
  const requiredTag = String(requiredTagDraft.value || "").trim();
  if (timeframe) parts.push(`tf ${timeframe}`);
  if (requiredTag) parts.push(`tag ${requiredTag}`);
  parts.push(`min ${parseDraftInt(minSamplesDraft.value, 30, 1)}`);
  parts.push(`top ${parseDraftInt(topKDraft.value, 25, 1)}`);
  return parts.join(" • ");
});

const stage3HoldoutContractSummary = computed(() => {
  if (!holdoutEvalDraft.value) {
    return `holdout off • mode ${String(holdoutModeDraft.value || "light").trim() || "light"}`;
  }
  return [
    `holdout ${String(holdoutModeDraft.value || "light").trim() || "light"}`,
    `tag ${String(holdoutTagDraft.value || "holdout").trim() || "holdout"}`,
    holdoutSaveRowsDraft.value ? "save rows on" : "save rows off",
  ].join(" • ");
});

const stage3HoldoutModeDetailSummary = computed(() => {
  const mode = String(holdoutModeDraft.value || "light").trim() || "light";
  if (!holdoutEvalDraft.value) {
    return "Holdout is disabled for this request. Unsupported strategies are downgraded by the request builder and holdout rank controls are ignored until holdout evaluation is turned back on.";
  }
  if (mode === "heavy") {
    return "Heavy mode emits nested validation config (walk-forward, perturbation, bootstrap, iterative engine). Backend still owns holdout rank fallback and warning text.";
  }
  return "Light mode keeps holdout enabled but omits the nested validation payload. Backend still owns holdout rank fallback and warning text.";
});

const stage3GroupingContractSummary = computed(() => {
  const reps = Math.min(
    3,
    Math.max(1, parseDraftInt(holdoutRepsDraft.value, 2, 1)),
  );
  return [
    String(groupingModeDraft.value || "clustered").trim() || "clustered",
    `int ${parseDraftInt(clusterIntBucketDraft.value, 5, 1)}`,
    `float ${parseDraftFloat(clusterFloatBucketDraft.value, 0.25).toFixed(2)}`,
    `reps ${reps} (clamped 1-3)`,
  ].join(" • ");
});

const stage3ShortlistContractSummary = computed(() => {
  return [
    "strict shortlist on",
    "fallback off",
    `rank ${String(holdoutEvalRankByDraft.value || "auto").trim() || "auto"}`,
    `topN ${parseDraftInt(holdoutEvalTopNDraft.value, 0, 0)}`,
    `explore ${parseDraftInt(holdoutExploreNDraft.value, 0, 0)}`,
  ].join(" • ");
});

const stage3ScoreContractSummary = computed(() => {
  return "Stage 3 meta_score uses backend MetaScoreWeights. Holdout can replace ordering with final_score when holdout data exists. Stage 5 blend weights stay separate.";
});

const stage3EvidenceContractSummary = computed(() => {
  return "Evidence flags remain warnings, not hard gates. After holdout runs, flags are refreshed on the holdout-enriched group state. Workbook output is score-first, so JSON and saved artifact detail remain the strongest evidence surface.";
});

const metaMlWarningText = computed(() => {
  const text = String(metaMlStatus.value || "").trim();
  const lower = text.toLowerCase();
  if (!text) return "";
  if (
    lower.includes("unhealthy") ||
    lower.includes("failed to load") ||
    lower.includes("no ml model saved") ||
    lower.includes("meta_scan_error=")
  ) {
    return text;
  }
  return "";
});

watch(
  () =>
    [
      String(metaStrategyKind.value || "CompositeMo"),
      String(metaTimeframe.value || ""),
      String(metaRequiredTag.value || ""),
      String(metaMinSamples.value || "30"),
      String(metaTopK.value || "25"),
      String(metaHoldoutTag.value || "holdout").trim() || "holdout",
      Boolean(metaEvalHoldout.value),
      Boolean(metaSaveHoldoutRows.value),
      String(metaHoldoutMode.value || "light").trim() || "light",
      String(metaHoldoutReps.value || "2").trim() || "2",
      String(metaHoldoutEvalTopN.value || "0").trim() || "0",
      String(metaHoldoutEvalRankBy.value || "auto").trim() || "auto",
      String(metaHoldoutExploreN.value || "0").trim() || "0",
      String(metaGroupingMode.value || "clustered").trim() || "clustered",
      String(metaClusterIntBucket.value || "5").trim() || "5",
      String(metaClusterFloatBucket.value || "0.25").trim() || "0.25",
      String(metaCompareSelected.value || "").trim(),
      String(batchDatasetSearch.value || "").trim(),
      String(batchTags.value || "").trim(),
      String(batchHoldoutPct.value || "15").trim() || "15",
      String(batchHoldoutTag.value || "holdout").trim() || "holdout",
      String(batchExecutionMode.value || "full").trim() || "full",
      String(batchPreloadPolicy.value || "host_only").trim() || "host_only",
      Boolean(batchAutoMeta.value),
      Boolean(batchAutoStage5.value),
      Boolean(batchSkipDone.value),
      String(metaSavedSearch.value || "").trim(),
      String(metaSavedSelected.value || "").trim(),
      String(metaSavedTagsDraft.value || "").trim(),
      String(stage4SavedSearch.value || "").trim(),
      String(stage4SavedSelected.value || "").trim(),
      String(stage4SavedTagsDraft.value || "").trim(),
      String(policyEvalSavedSearch.value || "").trim(),
      String(policyEvalSavedSelected.value || "").trim(),
      String(policyEvalSavedTagsDraft.value || "").trim(),
      Boolean(metaUseMlConfidence.value),
      Boolean(metaMlAutopilot.value),
      String(metaStage4TopK.value || "10").trim() || "10",
      String(metaStage4MinConf.value || "0.55").trim() || "0.55",
      String(metaStage4MetaWeight.value || "0.60").trim() || "0.60",
      String(metaStage4MlWeight.value || "0.35").trim() || "0.35",
      String(metaStage4UncertaintyWeight.value || "0.05").trim() || "0.05",
      String(metaStage4AcceptConf.value || "0.70").trim() || "0.70",
      String(metaStage4ReviewConf.value || "0.55").trim() || "0.55",
      String(metaStage4MaxAcceptUncertainty.value || "0.45").trim() || "0.45",
      Boolean(metaStage4AutoTrain.value),
      Boolean(metaStage4RequireHealthy.value),
      Boolean(metaStage4FallbackToMeta.value),
      String(metaStage4TrainingHistoryPolicy.value || "source_excluded").trim() ||
        "source_excluded",
    ] as const,
  ([
    nextStrategy,
    nextTimeframe,
    nextRequiredTag,
    nextMinSamples,
    nextTopK,
    nextHoldoutTag,
    nextHoldoutEval,
    nextHoldoutSaveRows,
    nextHoldoutMode,
    nextHoldoutReps,
    nextHoldoutEvalTopN,
    nextHoldoutEvalRankBy,
    nextHoldoutExploreN,
    nextGroupingMode,
    nextClusterIntBucket,
    nextClusterFloatBucket,
    nextCompare,
    nextBatchSearch,
    nextBatchTags,
    nextBatchHoldoutPct,
    nextBatchHoldoutTag,
    nextBatchExecutionMode,
    nextBatchPreloadPolicy,
    nextBatchAutoMeta,
    nextBatchAutoStage5,
    nextBatchSkipDone,
    nextMetaSavedSearch,
    nextMetaSavedSelect,
    nextMetaSavedTags,
    nextStage4SavedSearch,
    nextStage4SavedSelect,
    nextStage4SavedTags,
    nextPolicyEvalSavedSearch,
    nextPolicyEvalSavedSelect,
    nextPolicyEvalSavedTags,
    nextUseMlConfidence,
    nextMlAutopilot,
    nextStage4TopK,
    nextStage4MinConf,
    nextStage4MetaWeight,
    nextStage4MlWeight,
    nextStage4UncertaintyWeight,
    nextStage4AcceptConf,
    nextStage4ReviewConf,
    nextStage4MaxAcceptUncertainty,
    nextStage4AutoTrain,
    nextStage4RequireHealthy,
    nextStage4FallbackToMeta,
    nextStage4TrainingHistoryPolicy,
  ]) => {
    if (strategyDraft.value !== nextStrategy)
      strategyDraft.value = nextStrategy;
    if (timeframeDraft.value !== nextTimeframe)
      timeframeDraft.value = nextTimeframe;
    if (requiredTagDraft.value !== nextRequiredTag)
      requiredTagDraft.value = nextRequiredTag;
    if (minSamplesDraft.value !== nextMinSamples)
      minSamplesDraft.value = nextMinSamples;
    if (topKDraft.value !== nextTopK) topKDraft.value = nextTopK;
    if (holdoutTagDraft.value !== nextHoldoutTag)
      holdoutTagDraft.value = nextHoldoutTag;
    if (holdoutEvalDraft.value !== nextHoldoutEval)
      holdoutEvalDraft.value = nextHoldoutEval;
    if (holdoutSaveRowsDraft.value !== nextHoldoutSaveRows)
      holdoutSaveRowsDraft.value = nextHoldoutSaveRows;
    if (holdoutModeDraft.value !== nextHoldoutMode)
      holdoutModeDraft.value = nextHoldoutMode;
    if (holdoutRepsDraft.value !== nextHoldoutReps)
      holdoutRepsDraft.value = nextHoldoutReps;
    if (holdoutEvalTopNDraft.value !== nextHoldoutEvalTopN)
      holdoutEvalTopNDraft.value = nextHoldoutEvalTopN;
    if (holdoutEvalRankByDraft.value !== nextHoldoutEvalRankBy)
      holdoutEvalRankByDraft.value = nextHoldoutEvalRankBy;
    if (holdoutExploreNDraft.value !== nextHoldoutExploreN)
      holdoutExploreNDraft.value = nextHoldoutExploreN;
    if (groupingModeDraft.value !== nextGroupingMode)
      groupingModeDraft.value = nextGroupingMode;
    if (clusterIntBucketDraft.value !== nextClusterIntBucket)
      clusterIntBucketDraft.value = nextClusterIntBucket;
    if (clusterFloatBucketDraft.value !== nextClusterFloatBucket)
      clusterFloatBucketDraft.value = nextClusterFloatBucket;
    if (compareDraft.value !== nextCompare) compareDraft.value = nextCompare;
    if (batchSearchDraft.value !== nextBatchSearch)
      batchSearchDraft.value = nextBatchSearch;
    if (batchTagsDraft.value !== nextBatchTags)
      batchTagsDraft.value = nextBatchTags;
    if (batchHoldoutPctDraft.value !== nextBatchHoldoutPct)
      batchHoldoutPctDraft.value = nextBatchHoldoutPct;
    if (batchHoldoutTagDraft.value !== nextBatchHoldoutTag)
      batchHoldoutTagDraft.value = nextBatchHoldoutTag;
    if (batchExecutionModeDraft.value !== nextBatchExecutionMode)
      batchExecutionModeDraft.value = nextBatchExecutionMode;
    if (batchPreloadPolicyDraft.value !== nextBatchPreloadPolicy)
      batchPreloadPolicyDraft.value = nextBatchPreloadPolicy;
    if (batchAutoMetaDraft.value !== nextBatchAutoMeta)
      batchAutoMetaDraft.value = nextBatchAutoMeta;
    if (batchAutoStage5Draft.value !== nextBatchAutoStage5)
      batchAutoStage5Draft.value = nextBatchAutoStage5;
    if (batchSkipDoneDraft.value !== nextBatchSkipDone)
      batchSkipDoneDraft.value = nextBatchSkipDone;
    if (metaSavedSearchDraft.value !== nextMetaSavedSearch)
      metaSavedSearchDraft.value = nextMetaSavedSearch;
    if (metaSavedSelectDraft.value !== nextMetaSavedSelect)
      metaSavedSelectDraft.value = nextMetaSavedSelect;
    if (metaSavedTagsLocalDraft.value !== nextMetaSavedTags)
      metaSavedTagsLocalDraft.value = nextMetaSavedTags;
    if (stage4SavedSearchDraft.value !== nextStage4SavedSearch)
      stage4SavedSearchDraft.value = nextStage4SavedSearch;
    if (stage4SavedSelectDraft.value !== nextStage4SavedSelect)
      stage4SavedSelectDraft.value = nextStage4SavedSelect;
    if (stage4SavedTagsLocalDraft.value !== nextStage4SavedTags)
      stage4SavedTagsLocalDraft.value = nextStage4SavedTags;
    if (policyEvalSavedSearchDraft.value !== nextPolicyEvalSavedSearch)
      policyEvalSavedSearchDraft.value = nextPolicyEvalSavedSearch;
    if (policyEvalSavedSelectDraft.value !== nextPolicyEvalSavedSelect)
      policyEvalSavedSelectDraft.value = nextPolicyEvalSavedSelect;
    if (policyEvalSavedTagsLocalDraft.value !== nextPolicyEvalSavedTags)
      policyEvalSavedTagsLocalDraft.value = nextPolicyEvalSavedTags;
    if (metaUseMlConfidenceDraft.value !== nextUseMlConfidence)
      metaUseMlConfidenceDraft.value = nextUseMlConfidence;
    if (metaMlAutopilotDraft.value !== nextMlAutopilot)
      metaMlAutopilotDraft.value = nextMlAutopilot;
    if (metaStage4TopKDraft.value !== nextStage4TopK)
      metaStage4TopKDraft.value = nextStage4TopK;
    if (metaStage4MinConfDraft.value !== nextStage4MinConf)
      metaStage4MinConfDraft.value = nextStage4MinConf;
    if (metaStage4MetaWeightDraft.value !== nextStage4MetaWeight)
      metaStage4MetaWeightDraft.value = nextStage4MetaWeight;
    if (metaStage4MlWeightDraft.value !== nextStage4MlWeight)
      metaStage4MlWeightDraft.value = nextStage4MlWeight;
    if (metaStage4UncertaintyWeightDraft.value !== nextStage4UncertaintyWeight)
      metaStage4UncertaintyWeightDraft.value = nextStage4UncertaintyWeight;
    if (metaStage4AcceptConfDraft.value !== nextStage4AcceptConf)
      metaStage4AcceptConfDraft.value = nextStage4AcceptConf;
    if (metaStage4ReviewConfDraft.value !== nextStage4ReviewConf)
      metaStage4ReviewConfDraft.value = nextStage4ReviewConf;
    if (
      metaStage4MaxAcceptUncertaintyDraft.value !==
      nextStage4MaxAcceptUncertainty
    )
      metaStage4MaxAcceptUncertaintyDraft.value =
        nextStage4MaxAcceptUncertainty;
    if (metaStage4AutoTrainDraft.value !== nextStage4AutoTrain)
      metaStage4AutoTrainDraft.value = nextStage4AutoTrain;
    if (metaStage4RequireHealthyDraft.value !== nextStage4RequireHealthy)
      metaStage4RequireHealthyDraft.value = nextStage4RequireHealthy;
    if (metaStage4FallbackToMetaDraft.value !== nextStage4FallbackToMeta)
      metaStage4FallbackToMetaDraft.value = nextStage4FallbackToMeta;
    if (
      metaStage4TrainingHistoryPolicyDraft.value !==
      nextStage4TrainingHistoryPolicy
    )
      metaStage4TrainingHistoryPolicyDraft.value =
        nextStage4TrainingHistoryPolicy;
  },
  { immediate: true },
);

function setInputValue(id: string, nextValue: string): void {
  analysisStore.setMetaInputField(id, nextValue);
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } =
  useQueuedInputWriter(setInputValue);

function setSelectValue(id: string, nextValue: string): void {
  analysisStore.setMetaSelectField(id, nextValue);
}

function setCheckedValue(id: string, nextValue: boolean): void {
  analysisStore.setMetaCheckboxField(id, nextValue);
}

function runAction(action: () => void): void {
  flushQueuedInputValuesNow();
  action();
}

const savedMetaSecondaryActions = computed<ContextActionMenuItem[]>(() => [
  {
    key: "refresh",
    label: "Refresh",
    icon: artifactActionIcon("refresh"),
    disabled: !canRefreshMetaSaved.value,
  },
  {
    key: "report",
    label: "Open report",
    icon: artifactActionIcon("report"),
    disabled: !canOpenMetaSavedReport.value,
  },
  {
    key: "reveal",
    label: "Reveal file",
    icon: artifactActionIcon("reveal"),
    disabled: !canRevealMetaSaved.value,
  },
  {
    key: "delete",
    label: "Delete",
    icon: artifactActionIcon("delete"),
    disabled: !canDeleteMetaSaved.value,
    destructive: true,
    separatorBefore: true,
  },
]);

const savedStage5SecondaryActions = computed<ContextActionMenuItem[]>(() => [
  {
    key: "refresh",
    label: "Refresh",
    icon: artifactActionIcon("refresh"),
    disabled: !canRefreshStage4Saved.value,
  },
  {
    key: "reveal",
    label: "Reveal file",
    icon: artifactActionIcon("reveal"),
    disabled: !canRevealStage4Saved.value,
  },
  {
    key: "delete",
    label: "Delete",
    icon: artifactActionIcon("delete"),
    disabled: !canDeleteStage4Saved.value,
    destructive: true,
    separatorBefore: true,
  },
]);

const savedPolicyEvalSecondaryActions = computed<ContextActionMenuItem[]>(
  () => [
    {
      key: "refresh",
      label: "Refresh",
      icon: artifactActionIcon("refresh"),
      disabled: !canRefreshPolicyEvalSaved.value,
    },
    {
      key: "reveal",
      label: "Reveal file",
      icon: artifactActionIcon("reveal"),
      disabled: !canRevealPolicyEvalSaved.value,
    },
    {
      key: "delete",
      label: "Delete",
      icon: artifactActionIcon("delete"),
      disabled: !canDeletePolicyEvalSaved.value,
      destructive: true,
      separatorBefore: true,
    },
  ],
);

const savedArtifactFamilyConfig = computed<SavedArtifactFamilyConfig>(() => {
  switch (savedArtifactFamily.value) {
    case "stage5":
      return {
        key: "stage5",
        title: "Saved Stage 5",
        lead: "Manage saved Stage 5 confidence outputs alongside the base meta artifacts.",
        selectLabel: "Saved Stage 5",
        searchFieldId: "vueStage4SavedSearch",
        selectFieldId: "vueStage4SavedSelect",
        tagsFieldId: "vueStage4SavedTags",
        searchDraft: stage4SavedSearchDraft.value,
        selectedValue: stage4SavedSelectDraft.value,
        tagsDraft: stage4SavedTagsLocalDraft.value,
        options: stage4SavedSelectOptions.value,
        meta: String(stage4SavedMeta.value || ""),
        canSaveTags: !!canSaveStage4SavedTags.value,
        canLoad: !!canLoadStage4Saved.value,
        secondaryActions: savedStage5SecondaryActions.value,
        actionMenuLabel: "Saved Stage 5 artifact actions",
      };
    case "policyEval":
      return {
        key: "policyEval",
        title: "Saved Policy Evaluation",
        lead: "Review and reopen saved policy-evaluation outputs without leaving the aggregate workspace.",
        selectLabel: "Saved policy evaluation",
        searchFieldId: "vuePolicyEvalSavedSearch",
        selectFieldId: "vuePolicyEvalSavedSelect",
        tagsFieldId: "vuePolicyEvalSavedTags",
        searchDraft: policyEvalSavedSearchDraft.value,
        selectedValue: policyEvalSavedSelectDraft.value,
        tagsDraft: policyEvalSavedTagsLocalDraft.value,
        options: policyEvalSavedSelectOptions.value,
        meta: String(policyEvalSavedMeta.value || ""),
        canSaveTags: !!canSavePolicyEvalSavedTags.value,
        canLoad: !!canLoadPolicyEvalSaved.value,
        secondaryActions: savedPolicyEvalSecondaryActions.value,
        actionMenuLabel: "Saved policy evaluation artifact actions",
      };
    default:
      return {
        key: "meta",
        title: "Saved Meta",
        lead: "Search, tag, load, and reveal previously generated meta-analysis artifacts.",
        selectLabel: "Saved meta",
        searchFieldId: "vueMetaSavedSearch",
        selectFieldId: "vueMetaSavedSelect",
        tagsFieldId: "vueMetaSavedTags",
        searchDraft: metaSavedSearchDraft.value,
        selectedValue: metaSavedSelectDraft.value,
        tagsDraft: metaSavedTagsLocalDraft.value,
        options: metaSavedSelectOptions.value,
        meta: String(metaSavedMeta.value || ""),
        canSaveTags: !!canSaveMetaSavedTags.value,
        canLoad: !!canLoadMetaSaved.value,
        secondaryActions: savedMetaSecondaryActions.value,
        actionMenuLabel: "Saved meta artifact actions",
      };
  }
});

const pendingSavedArtifactDeleteIdentity = computed(() => {
  const family = pendingSavedArtifactDeleteFamily.value;
  if (!family) return "";
  const config =
    family === savedArtifactFamily.value
      ? savedArtifactFamilyConfig.value
      : family === "stage5"
        ? {
            options: stage4SavedSelectOptions.value,
            selectedValue: stage4SavedSelectDraft.value,
          }
        : family === "policyEval"
          ? {
              options: policyEvalSavedSelectOptions.value,
              selectedValue: policyEvalSavedSelectDraft.value,
            }
          : {
              options: metaSavedSelectOptions.value,
              selectedValue: metaSavedSelectDraft.value,
            };
  const selectedOption = config.options.find(
    (option) => option.value === config.selectedValue,
  );
  return String(selectedOption?.label || config.selectedValue || "").trim();
});

const savedArtifactDeleteTitle = computed(() => {
  if (pendingSavedArtifactDeleteFamily.value === "stage5") {
    return "Delete saved Stage 5 artifact";
  }
  if (pendingSavedArtifactDeleteFamily.value === "policyEval") {
    return "Delete saved policy evaluation";
  }
  return "Delete saved meta artifact";
});

const savedArtifactDeleteBody = computed(() => {
  const identity = pendingSavedArtifactDeleteIdentity.value;
  const familyLabel =
    pendingSavedArtifactDeleteFamily.value === "stage5"
      ? "saved Stage 5 artifact"
      : pendingSavedArtifactDeleteFamily.value === "policyEval"
        ? "saved policy evaluation"
        : "saved meta artifact";
  if (!identity) {
    return `Select a ${familyLabel} before deleting it.`;
  }
  return `Delete this ${familyLabel}?\n\n${identity}\n\nThis cannot be undone from the aggregate artifact library.`;
});

const canConfirmSavedArtifactDelete = computed(() => {
  switch (pendingSavedArtifactDeleteFamily.value) {
    case "stage5":
      return Boolean(
        canDeleteStage4Saved.value && stage4SavedSelectDraft.value,
      );
    case "policyEval":
      return Boolean(
        canDeletePolicyEvalSaved.value && policyEvalSavedSelectDraft.value,
      );
    case "meta":
      return Boolean(canDeleteMetaSaved.value && metaSavedSelectDraft.value);
    default:
      return false;
  }
});

function clickSavedArtifactFamily(key: SavedArtifactFamilyKey): void {
  savedArtifactFamily.value = key;
}

function moveSavedArtifactFamilyFocus(event: KeyboardEvent): void {
  if (savedArtifactFamilyItems.length <= 1) return;
  const currentIndex = savedArtifactFamilyItems.findIndex(
    (item) => item.key === savedArtifactFamily.value,
  );
  if (currentIndex < 0) return;

  let nextIndex = currentIndex;
  if (event.key === "ArrowRight" || event.key === "ArrowDown") {
    nextIndex = (currentIndex + 1) % savedArtifactFamilyItems.length;
  } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
    nextIndex =
      (currentIndex - 1 + savedArtifactFamilyItems.length) %
      savedArtifactFamilyItems.length;
  } else if (event.key === "Home") {
    nextIndex = 0;
  } else if (event.key === "End") {
    nextIndex = savedArtifactFamilyItems.length - 1;
  } else {
    return;
  }

  event.preventDefault();
  clickSavedArtifactFamily(savedArtifactFamilyItems[nextIndex]!.key);
  void nextTick().then(() => {
    const buttons = Array.from(
      document.querySelectorAll<HTMLButtonElement>(
        "[data-saved-artifact-family]",
      ),
    );
    buttons[nextIndex]?.focus();
  });
}

function updateSavedArtifactSearch(nextValue: string): void {
  if (savedArtifactFamily.value === "stage5") {
    stage4SavedSearchDraft.value = nextValue;
    queueInputValue("stage4SavedSearch", nextValue);
    return;
  }
  if (savedArtifactFamily.value === "policyEval") {
    policyEvalSavedSearchDraft.value = nextValue;
    queueInputValue("policyEvalSavedSearch", nextValue);
    return;
  }
  metaSavedSearchDraft.value = nextValue;
  queueInputValue("metaSavedSearch", nextValue);
}

function commitSavedArtifactSearch(): void {
  commitInputValue(
    savedArtifactFamilyConfig.value.searchFieldId
      .replace(/^vue/, "")
      .replace(/^./, (v) => v.toLowerCase()),
    savedArtifactFamilyConfig.value.searchDraft,
  );
}

function updateSavedArtifactSelect(nextValue: string): void {
  if (savedArtifactFamily.value === "stage5") {
    stage4SavedSelectDraft.value = nextValue;
    setSelectValue("stage4SavedSelect", nextValue);
    return;
  }
  if (savedArtifactFamily.value === "policyEval") {
    policyEvalSavedSelectDraft.value = nextValue;
    setSelectValue("policyEvalSavedSelect", nextValue);
    return;
  }
  metaSavedSelectDraft.value = nextValue;
  setSelectValue("metaSavedSelect", nextValue);
}

function updateSavedArtifactTags(nextValue: string): void {
  if (savedArtifactFamily.value === "stage5") {
    stage4SavedTagsLocalDraft.value = nextValue;
    queueInputValue("stage4SavedTags", nextValue);
    return;
  }
  if (savedArtifactFamily.value === "policyEval") {
    policyEvalSavedTagsLocalDraft.value = nextValue;
    queueInputValue("policyEvalSavedTags", nextValue);
    return;
  }
  metaSavedTagsLocalDraft.value = nextValue;
  queueInputValue("metaSavedTags", nextValue);
}

function commitSavedArtifactTags(): void {
  commitInputValue(
    savedArtifactFamilyConfig.value.tagsFieldId
      .replace(/^vue/, "")
      .replace(/^./, (v) => v.toLowerCase()),
    savedArtifactFamilyConfig.value.tagsDraft,
  );
}

function saveSelectedArtifactTags(): void {
  if (savedArtifactFamily.value === "stage5") {
    runAction(() => analysisStore.saveStage5SavedTags());
    return;
  }
  if (savedArtifactFamily.value === "policyEval") {
    runAction(() => analysisStore.savePolicyEvalSavedTags());
    return;
  }
  runAction(() => analysisStore.saveSavedTags());
}

function loadSelectedArtifact(): void {
  if (savedArtifactFamily.value === "stage5") {
    runAction(() => analysisStore.loadStage5Saved());
    return;
  }
  if (savedArtifactFamily.value === "policyEval") {
    runAction(() => analysisStore.loadPolicyEvalSaved());
    return;
  }
  runAction(() => analysisStore.loadSaved());
}

function queueSavedArtifactDeleteConfirmation(
  family: SavedArtifactFamilyKey,
): void {
  flushQueuedInputValuesNow();
  void nextTick().then(() => {
    pendingSavedArtifactDeleteFamily.value = family;
  });
}

function cancelPendingSavedArtifactDelete(): void {
  pendingSavedArtifactDeleteFamily.value = null;
}

function confirmPendingSavedArtifactDelete(): void {
  const family = pendingSavedArtifactDeleteFamily.value;
  pendingSavedArtifactDeleteFamily.value = null;
  if (family === "stage5") {
    runAction(() => analysisStore.deleteStage5Saved());
    return;
  }
  if (family === "policyEval") {
    runAction(() => analysisStore.deletePolicyEvalSaved());
    return;
  }
  runAction(() => analysisStore.deleteSaved());
}

function onSavedArtifactActionSelect(actionKey: string): void {
  if (actionKey === "delete") {
    queueSavedArtifactDeleteConfirmation(savedArtifactFamily.value);
    return;
  }

  if (savedArtifactFamily.value === "stage5") {
    if (actionKey === "refresh") {
      runAction(() => analysisStore.refreshStage5Saved());
      return;
    }
    if (actionKey === "reveal") {
      runAction(() => analysisStore.revealStage5Saved());
    }
    return;
  }

  if (savedArtifactFamily.value === "policyEval") {
    if (actionKey === "refresh") {
      runAction(() => analysisStore.refreshPolicyEvalSaved());
      return;
    }
    if (actionKey === "reveal") {
      runAction(() => analysisStore.revealPolicyEvalSaved());
    }
    return;
  }

  if (actionKey === "refresh") {
    runAction(() => analysisStore.refreshSaved());
    return;
  }
  if (actionKey === "report") {
    runAction(() => analysisStore.openSavedReport());
    return;
  }
  if (actionKey === "reveal") {
    runAction(() => analysisStore.revealSaved());
  }
}

function aggregateTabId(key: string): string {
  return `meta_aggregate_tab_${key}`;
}

function aggregatePanelId(key: AggregateTabKey): string {
  if (key === "meta") {
    return metaStage.value === "data_loaded_no_run"
      ? "meta_aggregate_panel_meta_empty"
      : "meta_aggregate_panel_meta_main";
  }
  if (key === "compare") {
    return metaCompareStage.value === "results_available_no_selection"
      ? "meta_aggregate_panel_compare_empty"
      : "meta_aggregate_panel_compare_main";
  }
  return `meta_aggregate_panel_${key}`;
}

function headerMeta(header: Header<MetaRow, unknown>): MatrixColumnMeta {
  return header.column.columnDef.meta as MatrixColumnMeta;
}

function metaCellPayload(cell: Cell<MetaRow, unknown>): MetaCell {
  return cell.getValue() as MetaCell;
}

const resizingMetaColumnKey = ref("");
let metaResizeStartX = 0;
let metaResizeStartWidth = 0;

function metaHeaderStyle(header: Header<MetaRow, unknown>): Record<string, string> {
  return columnWidthStyle(headerMeta(header).columnKey);
}

function metaCellStyle(cell: Cell<MetaRow, unknown>): Record<string, string> {
  const meta = cell.column.columnDef.meta as MatrixColumnMeta;
  return columnWidthStyle(meta.columnKey);
}

function finishMetaColumnResize(): void {
  resizingMetaColumnKey.value = "";
  window.removeEventListener("pointermove", onMetaColumnResizeMove);
  window.removeEventListener("pointerup", finishMetaColumnResize);
  window.removeEventListener("pointercancel", finishMetaColumnResize);
}

function onMetaColumnResizeMove(event: PointerEvent): void {
  const columnKey = resizingMetaColumnKey.value;
  if (!columnKey) return;
  setColumnWidth(columnKey, metaResizeStartWidth + event.clientX - metaResizeStartX);
}

function startMetaColumnResize(event: PointerEvent, header: Header<MetaRow, unknown>): void {
  const columnKey = headerMeta(header).columnKey;
  if (!columnKey) return;
  event.preventDefault();
  event.stopPropagation();
  resizingMetaColumnKey.value = columnKey;
  metaResizeStartX = event.clientX;
  metaResizeStartWidth =
    columnWidths.value[columnKey] ??
    (event.currentTarget instanceof HTMLElement
      ? event.currentTarget.closest("th")?.getBoundingClientRect().width
      : 128) ??
    128;
  window.addEventListener("pointermove", onMetaColumnResizeMove);
  window.addEventListener("pointerup", finishMetaColumnResize);
  window.addEventListener("pointercancel", finishMetaColumnResize);
}

function onMetaColumnResizeKeydown(event: KeyboardEvent, header: Header<MetaRow, unknown>): void {
  const columnKey = headerMeta(header).columnKey;
  if (!columnKey) return;
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    resizeColumnWidth(columnKey, event.shiftKey ? -48 : -16);
    return;
  }
  if (event.key === "ArrowRight") {
    event.preventDefault();
    resizeColumnWidth(columnKey, event.shiftKey ? 48 : 16);
    return;
  }
  if (event.key === "Delete" || event.key === "Backspace") {
    event.preventDefault();
    clearColumnWidth(columnKey);
  }
}

function metaColumnToggleId(columnKey: string): string {
  return `metaColumnToggle_${String(columnKey || "").replace(/[^a-zA-Z0-9_-]+/g, "_")}`;
}

function moveAggregateTabFocus(event: KeyboardEvent): void {
  if (aggregateTabItems.length <= 1) return;
  const currentIndex = aggregateTabItems.findIndex(
    (item) => item.key === activeAggregateTab.value,
  );
  if (currentIndex < 0) return;

  let nextIndex = currentIndex;
  if (event.key === "ArrowRight" || event.key === "ArrowDown")
    nextIndex = (currentIndex + 1) % aggregateTabItems.length;
  else if (event.key === "ArrowLeft" || event.key === "ArrowUp")
    nextIndex =
      (currentIndex - 1 + aggregateTabItems.length) % aggregateTabItems.length;
  else if (event.key === "Home") nextIndex = 0;
  else if (event.key === "End") nextIndex = aggregateTabItems.length - 1;
  else return;

  event.preventDefault();
  const nextKey = aggregateTabItems[nextIndex]?.key;
  if (!nextKey) return;
  clickAggregateTab(nextKey);
  void nextTick(() => {
    document.querySelector<HTMLElement>(`#${aggregateTabId(nextKey)}`)?.focus();
  });
}

function clickAggregateTab(tab: AggregateTabKey): void {
  const aggregateTab = String(tab || "").trim() as AggregateTabKey;
  if (!aggregateTab || activeAggregateTab.value === aggregateTab) return;
  workflowStore.selectAggregateTab(aggregateTab);
}

function toggleBatchDatasetByPath(
  pathValue: string,
  nextChecked: boolean,
): void {
  const path = String(pathValue || "").trim();
  if (!path) return;
  analysisStore.toggleBatchDataset(path, nextChecked);
}

function batchDatasetInputId(index: number): string {
  return `meta_batch_dataset_${index}`;
}

function batchDatasetDescriptionId(index: number): string {
  return `meta_batch_dataset_meta_${index}`;
}

onMounted(async () => {
  await nextTick();
  scrollToTop();
});

onBeforeUnmount(() => {
  finishMetaColumnResize();
  flushQueuedInputValuesNow();
});
</script>

<template>
  <Teleport v-if="mountReady" to="#metaVueShellMount">
    <section class="metaPanelVueRoot metaVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Aggregate</div>
          <h2 class="panelTitle">Meta Analysis</h2>
          <div class="panelSubtitle">
            Group saved runs, generate meta artifacts, review saved aggregate
            outputs, and compare baselines before promotion.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">
          {{ activeAggregateTabLabel }} · {{ stamp }}
        </div>
      </header>

      <div class="panelSummaryChips" aria-label="Meta analysis summary">
        <div
          v-for="chip in metaSummaryChips"
          :key="chip"
          class="panelSummaryChip"
        >
          {{ chip }}
        </div>
      </div>

      <section v-if="metaStage === 'no_data'" class="emptyState">
        <img
          class="emptyStateIllustration"
          :src="emptyArtifactLibraryIllustration"
          alt=""
          aria-hidden="true"
          draggable="false"
        />
        <div class="emptyTitle">Meta Analysis needs saved runs first.</div>
        <div class="emptyBody">
          Build a run library by saving optimizations, then use this workspace
          to generate meta artifacts, compare groups, and rank robust
          candidates.
        </div>
        <div
          class="panelStatus mono panelNumeric"
          role="status"
          aria-live="polite"
        >
          {{ statusLine }}
        </div>
      </section>

      <section
        v-else-if="
          metaStage === 'data_loaded_no_run' && activeAggregateTab === 'meta'
        "
        id="meta_aggregate_panel_meta_empty"
        class="emptyState"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('meta')"
      >
        <img
          class="emptyStateIllustration"
          :src="emptyResultsWorkspaceIllustration"
          alt=""
          aria-hidden="true"
          draggable="false"
        />
        <div class="emptyTitle">
          Saved runs are available, but no meta artifact is active yet.
        </div>
        <div class="emptyBody">
          Configure the selection criteria below, then run meta analysis to
          generate grouped recommendations and reports.
        </div>
        <div class="panelMuted mono panelNumeric">
          {{ aggregateSummarySavedRuns }} saved runs available
        </div>
      </section>

      <section
        v-else-if="
          metaCompareStage === 'results_available_no_selection' &&
          compareTabActive
        "
        id="meta_aggregate_panel_compare_empty"
        class="emptyState"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('compare')"
      >
        <img
          class="emptyStateIllustration"
          :src="emptyCompareWorkspaceIllustration"
          alt=""
          aria-hidden="true"
          draggable="false"
        />
        <div class="emptyTitle">
          Select a baseline meta artifact to compare.
        </div>
        <div class="emptyBody">
          Comparison results appear here once you choose a saved meta artifact
          from the compare selector.
        </div>
        <div
          class="panelStatus mono panelNumeric"
          role="status"
          aria-live="polite"
        >
          {{ statusLine }}
        </div>
      </section>

      <section v-if="showMetaTooling" id="metaAggregateSurface" class="pane">
        <div
          class="subtabs"
          aria-label="Meta analysis sections"
          role="tablist"
          aria-orientation="horizontal"
          @keydown="moveAggregateTabFocus"
        >
          <button
            v-for="item in aggregateTabItems"
            :id="aggregateTabId(item.key)"
            :key="`meta_aggregate_tab_${item.key}`"
            type="button"
            :class="{ active: activeAggregateTab === item.key }"
            role="tab"
            :aria-selected="activeAggregateTab === item.key"
            :data-aggregate-tab="item.key"
            :aria-controls="aggregatePanelId(item.key)"
            :tabindex="activeAggregateTab === item.key ? 0 : -1"
            @click="clickAggregateTab(item.key)"
          >
            {{ item.label }}
          </button>
        </div>
      </section>

      <section
        v-if="showMetaTooling && activeAggregateTab === 'batch'"
        :id="aggregatePanelId('batch')"
        class="pane panelProminentPane"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('batch')"
      >
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Batch</div>
            <h3 class="panelPaneTitle">Dataset generation queue</h3>
            <div class="panelSectionLead">
              Choose the dataset set, holdout settings, and retry behavior for
              the next batch-generation sweep.
            </div>
          </div>
          <ContextHelpTip
            title="Batch generation queue"
            summary="Use this batch surface for dataset selection, holdout policy, and retry behavior before launching a sweep."
            detail="The queue is designed to expose the next run's selection, holdout tag, and auto-meta policy without opening another panel. Long-running output stays behind disclosure so the dataset list remains the dominant surface."
            aria-label="Batch generation queue help"
          />
        </div>
        <div class="gridFour">
          <div>
            <label for="vueBatchDatasetSearch">Search</label>
            <input
              id="vueBatchDatasetSearch"
              type="text"
              :value="batchSearchDraft"
              @input="
                batchSearchDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueInputValue('batchDatasetSearch', batchSearchDraft);
              "
              @change="commitInputValue('batchDatasetSearch', batchSearchDraft)"
            />
          </div>
          <div>
            <label for="vueBatchTags">Tags</label>
            <input
              id="vueBatchTags"
              type="text"
              :value="batchTagsDraft"
              @input="
                batchTagsDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueInputValue('batchTags', batchTagsDraft);
              "
              @change="commitInputValue('batchTags', batchTagsDraft)"
            />
          </div>
          <div>
            <label for="vueBatchHoldoutPct">Holdout %</label>
            <input
              id="vueBatchHoldoutPct"
              type="number"
              min="0"
              max="50"
              step="1"
              :value="batchHoldoutPctDraft"
              @input="
                batchHoldoutPctDraft = String(
                  ($event.target as HTMLInputElement).value || '15',
                );
                queueInputValue('batchHoldoutPct', batchHoldoutPctDraft);
              "
              @change="
                commitInputValue('batchHoldoutPct', batchHoldoutPctDraft)
              "
            />
          </div>
          <div>
            <label for="vueBatchHoldoutTag">Holdout tag</label>
            <input
              id="vueBatchHoldoutTag"
              type="text"
              :value="batchHoldoutTagDraft"
              @input="
                batchHoldoutTagDraft = String(
                  ($event.target as HTMLInputElement).value || 'holdout',
                );
                queueInputValue('batchHoldoutTag', batchHoldoutTagDraft);
              "
              @change="
                commitInputValue('batchHoldoutTag', batchHoldoutTagDraft)
              "
            />
          </div>
          <div>
            <label for="vueBatchExecutionMode">Execution mode</label>
            <select
              id="vueBatchExecutionMode"
              :value="batchExecutionModeDraft"
              @change="
                batchExecutionModeDraft = String(
                  ($event.target as HTMLSelectElement).value || 'full',
                );
                setSelectValue('batchExecutionMode', batchExecutionModeDraft);
                if (batchExecutionModeDraft === 'optimize_only') {
                  batchAutoMetaDraft = false;
                  batchAutoStage5Draft = false;
                  setCheckedValue('batchAutoMeta', false);
                  setCheckedValue('batchAutoStage5', false);
                }
              "
            >
              <option
                v-for="option in batchExecutionModeOptions"
                :key="`batch_mode_${option.value}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
          <div>
            <label for="vueBatchPreloadPolicy">Preload policy</label>
            <select
              id="vueBatchPreloadPolicy"
              :value="batchPreloadPolicyDraft"
              @change="
                batchPreloadPolicyDraft = String(
                  ($event.target as HTMLSelectElement).value || 'host_only',
                );
                setSelectValue('batchPreloadPolicy', batchPreloadPolicyDraft);
              "
            >
              <option
                v-for="option in batchPreloadPolicyOptions"
                :key="`batch_preload_${option.value}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
          </div>
        </div>

        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="batchAutoMetaDraft"
              :disabled="batchExecutionModeDraft === 'optimize_only'"
              @change="
                batchAutoMetaDraft = ($event.target as HTMLInputElement)
                  .checked;
                setCheckedValue('batchAutoMeta', batchAutoMetaDraft);
                if (!batchAutoMetaDraft) {
                  batchAutoStage5Draft = false;
                  setCheckedValue('batchAutoStage5', false);
                }
              "
            />
            Auto-run meta after batch
          </label>
          <label>
            <input
              type="checkbox"
              :checked="batchAutoStage5Draft"
              :disabled="
                batchExecutionModeDraft === 'optimize_only' ||
                !batchAutoMetaDraft
              "
              @change="
                batchAutoStage5Draft = ($event.target as HTMLInputElement)
                  .checked;
                setCheckedValue('batchAutoStage5', batchAutoStage5Draft);
              "
            />
            Run Stage 5 after meta
          </label>
          <label>
            <input
              type="checkbox"
              :checked="batchSkipDoneDraft"
              @change="
                batchSkipDoneDraft = ($event.target as HTMLInputElement)
                  .checked;
                setCheckedValue('batchSkipDone', batchSkipDoneDraft);
              "
            />
            Skip completed (same batch tag)
          </label>
        </div>
        <div class="panelMuted">
          Host preload is the default campaign path. The VRAM staging mode is
          experimental and only applies to the plain DoubleMa exact GpuKernel
          route; other paths fall back to host preload. `Optimize only` skips
          meta and Stage 5, while `Validate only` reuses existing optimization
          artifacts instead of rerunning search. Validation shortlist admission
          follows the current Validation workspace policy.
        </div>

        <div class="summaryGrid" aria-label="Batch generation operating mode">
          <article>
            <div class="panelMetricKey">Execution mode</div>
            <div class="panelMetricValue">{{ batchExecutionModeLabel }}</div>
          </article>
          <article>
            <div class="panelMetricKey">Preload policy</div>
            <div class="panelMetricValue">{{ batchPreloadPolicyLabel }}</div>
          </article>
          <article>
            <div class="panelMetricKey">Automation</div>
            <div class="panelMetricValue">{{ batchAutomationLabel }}</div>
          </article>
          <article>
            <div class="panelMetricKey">Datasets</div>
            <div class="panelMetricValue mono panelNumeric">
              {{ batchSelectionLabel }}
            </div>
          </article>
        </div>
        <div class="panelMuted">{{ batchHoldoutContractSummary }}</div>
        <div class="panelMuted">{{ batchRegistryContractSummary }}</div>
        <div class="panelMuted">{{ batchRecoveryContractSummary }}</div>

        <div class="actions">
          <button
            type="button"
            class="primary"
            :disabled="!canRunBatch"
            @click="runAction(() => analysisStore.runBatch())"
          >
            {{ batchRunButtonLabel }}
          </button>
          <button
            type="button"
            :disabled="!canBatchStop"
            @click="runAction(() => analysisStore.batchStop())"
          >
            Stop after current
          </button>
          <button
            type="button"
            :disabled="!canBatchRefresh"
            @click="runAction(() => analysisStore.batchRefresh())"
          >
            Refresh
          </button>
          <button
            type="button"
            :disabled="!canBatchSelectAll"
            @click="runAction(() => analysisStore.batchSelectAll())"
          >
            All
          </button>
          <button
            type="button"
            :disabled="!canBatchSelectNone"
            @click="runAction(() => analysisStore.batchSelectNone())"
          >
            None
          </button>
          <button
            type="button"
            :disabled="!canBatchRetryFailed"
            @click="runAction(() => analysisStore.batchRetryFailed())"
          >
            Retry failed
          </button>
          <button
            type="button"
            :disabled="!canBatchResumeIncomplete"
            @click="runAction(() => analysisStore.batchResumeIncomplete())"
          >
            Resume incomplete
          </button>
          <button
            type="button"
            :disabled="!canBatchRevealSummary"
            @click="runAction(() => analysisStore.batchRevealSummary())"
          >
            Reveal summary
          </button>
        </div>

        <div class="metaLine mono">{{ batchMeta }}</div>
        <div class="metaLine mono" role="status" aria-live="polite">
          Selected {{ batchSelectedCount }}/{{ batchTotalCount }}
        </div>

        <div
          class="batchList"
          role="group"
          aria-label="Batch generation datasets"
        >
          <label
            v-for="(item, index) in renderedBatchDatasets"
            :key="`batch_item_${item.path || item.label}_${index}`"
            class="batchItem"
            :class="{ disabled: item.disabled }"
            :for="batchDatasetInputId(index)"
          >
            <div class="left">
              <div class="mono">{{ item.label }}</div>
              <div :id="batchDatasetDescriptionId(index)" class="panelMuted">
                {{ item.meta }}
              </div>
            </div>
            <input
              :id="batchDatasetInputId(index)"
              type="checkbox"
              :checked="item.checked"
              :disabled="item.disabled"
              :aria-describedby="batchDatasetDescriptionId(index)"
              @change="
                toggleBatchDatasetByPath(
                  item.path,
                  ($event.target as HTMLInputElement).checked,
                )
              "
            />
          </label>
        </div>

        <details class="panelDetails">
          <summary>Batch output preview</summary>
          <div class="panelDetailsBody">
            <div class="panelMuted">
              Keep the raw batch summary close at hand for troubleshooting
              without letting it dominate the dataset-selection flow.
            </div>
            <pre class="mono preview">{{ batchOutputPreview }}</pre>
          </div>
        </details>
      </section>

      <div
        v-if="showMetaTooling && activeAggregateTab === 'saved'"
        :id="aggregatePanelId('saved')"
        class="aggregatePanelStack"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('saved')"
      >
        <section class="pane panelProminentPane savedArtifactPane">
          <div class="panelSectionHeader">
            <div class="panelHeadCopy">
              <div class="panelEyebrow">Aggregate library</div>
              <h3 class="panelPaneTitle savedArtifactHeading">
                <AppVisualIcon
                  :icon="artifactBundleIcon"
                  class="sectionGlyph"
                />
                <span>{{ savedArtifactFamilyConfig.title }}</span>
              </h3>
              <div class="panelSectionLead">
                {{ savedArtifactFamilyConfig.lead }}
              </div>
            </div>
          </div>

          <div
            class="subtabs savedArtifactFamilyTabs"
            aria-label="Saved aggregate artifact families"
            role="tablist"
            aria-orientation="horizontal"
            @keydown="moveSavedArtifactFamilyFocus"
          >
            <button
              v-for="item in savedArtifactFamilyItems"
              :key="`saved_artifact_family_${item.key}`"
              type="button"
              :class="{ active: savedArtifactFamily === item.key }"
              role="tab"
              :aria-selected="savedArtifactFamily === item.key"
              :tabindex="savedArtifactFamily === item.key ? 0 : -1"
              :data-saved-artifact-family="item.key"
              @click="clickSavedArtifactFamily(item.key)"
            >
              {{ item.label }}
            </button>
          </div>

          <div class="panelMuted">
            Saved Meta keeps aggregate selection baselines. Saved Stage 5 keeps
            ML confidence reranks. Saved Policy Eval keeps policy sweep outputs.
          </div>

          <div
            class="summaryGrid savedArtifactSummaryGrid"
            aria-label="Saved aggregate artifact provenance"
          >
            <article>
              <div class="panelMetricKey">Family role</div>
              <div class="panelMetricValue">
                {{
                  savedArtifactFamily === "stage5"
                    ? "ML confidence reranks and Stage 5 decision buckets."
                    : savedArtifactFamily === "policyEval"
                      ? "Policy sweep evaluations over saved Stage 5 or meta-derived candidates."
                      : "Aggregate selection baselines from saved meta artifacts."
                }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Active selection</div>
              <div class="panelMetricValue mono">
                {{
                  String(
                    savedArtifactFamilyConfig.options.find(
                      (option) =>
                        option.value ===
                        savedArtifactFamilyConfig.selectedValue,
                    )?.label ||
                      savedArtifactFamilyConfig.selectedValue ||
                      "No artifact selected.",
                  ).trim() || "No artifact selected."
                }}
              </div>
            </article>
          </div>

          <div class="gridThree">
            <div>
              <label :for="savedArtifactFamilyConfig.searchFieldId"
                >Search</label
              >
              <input
                :id="savedArtifactFamilyConfig.searchFieldId"
                type="text"
                :value="savedArtifactFamilyConfig.searchDraft"
                @input="
                  updateSavedArtifactSearch(
                    String(($event.target as HTMLInputElement).value || ''),
                  )
                "
                @change="commitSavedArtifactSearch()"
              />
            </div>
            <div>
              <label :for="savedArtifactFamilyConfig.selectFieldId">
                {{ savedArtifactFamilyConfig.selectLabel }}
              </label>
              <select
                :id="savedArtifactFamilyConfig.selectFieldId"
                :value="savedArtifactFamilyConfig.selectedValue"
                @change="
                  updateSavedArtifactSelect(
                    String(($event.target as HTMLSelectElement).value || ''),
                  )
                "
              >
                <option
                  v-for="option in savedArtifactFamilyConfig.options"
                  :key="`saved_artifact_${savedArtifactFamilyConfig.key}_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
            </div>
            <div>
              <label :for="savedArtifactFamilyConfig.tagsFieldId">Tags</label>
              <div class="rowAuto">
                <input
                  :id="savedArtifactFamilyConfig.tagsFieldId"
                  type="text"
                  :value="savedArtifactFamilyConfig.tagsDraft"
                  @input="
                    updateSavedArtifactTags(
                      String(($event.target as HTMLInputElement).value || ''),
                    )
                  "
                  @change="commitSavedArtifactTags()"
                />
                <button
                  type="button"
                  :disabled="!savedArtifactFamilyConfig.canSaveTags"
                  @click="saveSelectedArtifactTags"
                >
                  Save
                </button>
              </div>
            </div>
          </div>

          <div class="actions savedArtifactActions">
            <button
              type="button"
              class="buttonWithIcon"
              :disabled="!savedArtifactFamilyConfig.canLoad"
              @click="loadSelectedArtifact"
            >
              <AppVisualIcon :icon="loadSavedArtifactIcon" class="actionIcon" />
              <span>Load</span>
            </button>
            <ContextActionMenu
              :aria-label="savedArtifactFamilyConfig.actionMenuLabel"
              :items="savedArtifactFamilyConfig.secondaryActions"
              @select="onSavedArtifactActionSelect"
            />
          </div>

          <div class="metaLine mono">{{ savedArtifactFamilyConfig.meta }}</div>
        </section>
      </div>

      <div
        v-if="showMetaTooling && metaTabActive"
        id="meta_aggregate_panel_meta_main"
        class="aggregatePanelStack"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('meta')"
      >
        <section class="pane panelProminentPane">
          <div class="panelSectionHeader">
            <div class="panelHeadCopy">
              <div class="panelEyebrow">Snapshot</div>
              <h3 class="panelPaneTitle">Aggregate inventory</h3>
              <div class="panelSectionLead">
                Confirm how many saved runs and meta artifacts are in play
                before starting a new aggregate selection pass.
              </div>
            </div>
          </div>
          <div class="summaryGrid">
            <article>
              <div class="panelMetricKey">Saved runs</div>
              <div class="panelMetricValue mono panelNumeric">
                {{ aggregateSummarySavedRuns }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Saved meta</div>
              <div class="panelMetricValue mono panelNumeric">
                {{ aggregateSummarySavedMeta }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Latest groups</div>
              <div class="panelMetricValue mono panelNumeric">
                {{ aggregateSummaryLatestGroups }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Latest artifacts</div>
              <div class="panelMetricValue mono panelNumeric">
                {{ aggregateSummaryLatestArtifacts }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Latest holdout</div>
              <div class="panelMetricValue mono panelNumeric">
                {{ aggregateSummaryLatestHoldout }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Latest runtime</div>
              <div class="panelMetricValue mono panelNumeric">
                {{ aggregateSummaryLatestRuntime }}
              </div>
            </article>
          </div>
        </section>

        <section class="pane">
          <div class="panelSectionHeader">
            <div class="panelHeadCopy">
              <div class="panelEyebrow">ML</div>
              <h3 class="panelPaneTitle">ML Confidence + Stage 5</h3>
              <div class="panelSectionLead">
                Refresh the confidence model, tune Stage 5 weighting, and decide
                when aggregate ranking should trust ML outputs.
              </div>
            </div>
          </div>
          <div class="checks">
            <label>
              <input
                type="checkbox"
                :checked="metaUseMlConfidenceDraft"
                @change="
                  metaUseMlConfidenceDraft = ($event.target as HTMLInputElement)
                    .checked;
                  setCheckedValue(
                    'metaUseMlConfidence',
                    metaUseMlConfidenceDraft,
                  );
                "
              />
              Enable ML confidence
            </label>
            <label>
              <input
                type="checkbox"
                :checked="metaMlAutopilotDraft"
                @change="
                  metaMlAutopilotDraft = ($event.target as HTMLInputElement)
                    .checked;
                  setCheckedValue('metaMlAutopilot', metaMlAutopilotDraft);
                "
              />
              Autopilot
            </label>
          </div>
          <div class="actions">
            <button
              type="button"
              :disabled="!canMetaMlRefresh"
              @click="runAction(() => analysisStore.mlRefresh())"
            >
              Refresh
            </button>
            <button
              type="button"
              :disabled="!canMetaMlTrain"
              @click="runAction(() => analysisStore.mlTrain())"
            >
              Train / update
            </button>
            <button
              type="button"
              :disabled="!canMetaMlApply"
              @click="runAction(() => analysisStore.mlApply())"
            >
              Apply to current meta
            </button>
            <button
              type="button"
              :disabled="!canMetaMlDelete"
              @click="runAction(() => analysisStore.mlDelete())"
            >
              Delete model
            </button>
          </div>
          <div class="summaryGrid" aria-label="ML confidence report surfaces">
            <article>
              <div class="panelMetricKey">Model status</div>
              <div class="panelMetricValue mono">{{ metaMlStatusLabel }}</div>
            </article>
            <article>
              <div class="panelMetricKey">Stage 5 output</div>
              <div class="panelMetricValue mono">
                {{ metaStage5StatusLabel }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Policy evaluation</div>
              <div class="panelMetricValue mono">
                {{ metaPolicyEvalStatusLabel }}
              </div>
            </article>
          </div>
          <div class="summaryGrid" aria-label="ML confidence contract">
            <article>
              <div class="panelMetricKey">Scope</div>
              <div class="panelMetricValue mono">
                {{ metaMlScopeContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Rows</div>
              <div class="panelMetricValue mono">
                {{ metaMlRowsContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Label defaults</div>
              <div class="panelMetricValue mono">
                {{ metaMlLabelContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Family</div>
              <div class="panelMetricValue">
                {{ metaMlFamilyContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Health</div>
              <div class="panelMetricValue mono">
                {{ metaMlHealthContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Apply paths</div>
              <div class="panelMetricValue">
                {{ metaMlApplyContractSummary }}
              </div>
            </article>
          </div>
          <div class="summaryGrid" aria-label="Stage 5 and policy contracts">
            <article>
              <div class="panelMetricKey">Stage 5 request</div>
              <div class="panelMetricValue">
                {{ metaStage5RequestContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Selection order</div>
              <div class="panelMetricValue">
                {{ metaStage5SelectionContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Execution alias</div>
              <div class="panelMetricValue mono">
                {{ metaStage5ExecutionContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Policy eval</div>
              <div class="panelMetricValue">
                {{ metaPolicyEvalContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Saved replay</div>
              <div class="panelMetricValue">
                {{ metaSavedReplayContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Status surface</div>
              <div class="panelMetricValue">
                {{ metaMlStatusSurfaceSummary }}
              </div>
            </article>
          </div>
          <div class="metaLine mono">{{ metaMlStatus }}</div>
          <div
            v-if="metaMlWarningText"
            class="riskNotice mlRiskNotice"
            role="alert"
          >
            <div class="riskTitle">ML model health needs review</div>
            <div class="riskBody mono">{{ metaMlWarningText }}</div>
          </div>
          <div class="panelMuted">
            Model status reflects the current confidence model scope and health.
            Stage 5 preview shows the current live rerank output, and policy
            evaluation preview shows the current live evaluation payload. Saved
            Stage 5 and Saved Policy Eval artifacts stay in the Saved Aggregate
            library.
          </div>
          <div class="panelMuted">
            Auto target prefers per-dataset holdout rows when available and
            falls back to aggregate holdout gates. Explicit unhealthy means the
            backend AUC/Brier health gates failed; stale and retrain-recommended
            remain freshness signals.
          </div>
          <div class="panelMuted">
            Scope resolves from the latest meta result first and then falls back
            to the current Meta strategy/timeframe inputs. Only Stage 5 and
            Policy Eval persist durable artifacts; Apply to current meta is
            intentionally in-memory only.
          </div>
          <div class="panelMuted">
            Stage 5 excludes the active source meta artifact from auto-train
            history, reranks first, then assigns accept/review/reject buckets.
            If no group clears min ML confidence and fallback stays on, the
            backend falls back to pure meta-score ranking with a warning.
          </div>
          <div class="panelMuted">
            Policy evaluation is offline analysis over saved Meta history, not
            in-memory Apply and not the current live Stage 5 rerank. It compares
            meta_only, ml_only, and stage5_blended arms and keeps a separate
            saved JSON artifact family.
          </div>
          <div class="panelMuted">
            Saved Stage 5 uses the same durable replay artifact surface as
            load/reveal/delete; the UI no longer surfaces Stage 4 path alias
            wording.
          </div>

          <div class="gridThree" style="margin-top: 0.6rem">
            <div>
              <label for="vueMetaStage4TopK">Top K</label>
              <input
                id="vueMetaStage4TopK"
                type="number"
                min="1"
                step="1"
                :value="metaStage4TopKDraft"
                @input="
                  metaStage4TopKDraft = String(
                    ($event.target as HTMLInputElement).value || '10',
                  );
                  queueInputValue('metaStage4TopK', metaStage4TopKDraft);
                "
                @change="
                  commitInputValue('metaStage4TopK', metaStage4TopKDraft)
                "
              />
            </div>
            <div>
              <label for="vueMetaStage4MinConf">Min ML confidence</label>
              <input
                id="vueMetaStage4MinConf"
                type="number"
                min="0"
                max="1"
                step="0.01"
                :value="metaStage4MinConfDraft"
                @input="
                  metaStage4MinConfDraft = String(
                    ($event.target as HTMLInputElement).value || '0.55',
                  );
                  queueInputValue('metaStage4MinConf', metaStage4MinConfDraft);
                "
                @change="
                  commitInputValue('metaStage4MinConf', metaStage4MinConfDraft)
                "
              />
            </div>
            <div class="actions" style="margin-top: 1.4rem">
              <button
                type="button"
                :disabled="!canRunMetaStage4"
                @click="runAction(() => analysisStore.runStage5())"
              >
                Run Stage 5
              </button>
              <button
                type="button"
                :disabled="!canRunMetaPolicyEval"
                @click="runAction(() => analysisStore.runMlPolicyEvaluation())"
              >
                Evaluate policy
              </button>
              <button
                type="button"
                :disabled="!canRevealMetaStage4"
                @click="runAction(() => analysisStore.revealStage5())"
              >
                Reveal Stage 5 file
              </button>
            </div>
          </div>

          <div class="gridThree" style="margin-top: 0.6rem">
            <div>
              <label for="vueMetaStage4MetaWeight">Weight: Meta score</label>
              <input
                id="vueMetaStage4MetaWeight"
                type="number"
                min="0"
                step="0.01"
                :value="metaStage4MetaWeightDraft"
                @input="
                  metaStage4MetaWeightDraft = String(
                    ($event.target as HTMLInputElement).value || '0.60',
                  );
                  queueInputValue(
                    'metaStage4MetaWeight',
                    metaStage4MetaWeightDraft,
                  );
                "
                @change="
                  commitInputValue(
                    'metaStage4MetaWeight',
                    metaStage4MetaWeightDraft,
                  )
                "
              />
            </div>
            <div>
              <label for="vueMetaStage4MlWeight">Weight: ML confidence</label>
              <input
                id="vueMetaStage4MlWeight"
                type="number"
                min="0"
                step="0.01"
                :value="metaStage4MlWeightDraft"
                @input="
                  metaStage4MlWeightDraft = String(
                    ($event.target as HTMLInputElement).value || '0.35',
                  );
                  queueInputValue(
                    'metaStage4MlWeight',
                    metaStage4MlWeightDraft,
                  );
                "
                @change="
                  commitInputValue(
                    'metaStage4MlWeight',
                    metaStage4MlWeightDraft,
                  )
                "
              />
            </div>
            <div>
              <label for="vueMetaStage4UncertaintyWeight"
                >Weight: uncertainty penalty</label
              >
              <input
                id="vueMetaStage4UncertaintyWeight"
                type="number"
                min="0"
                step="0.01"
                :value="metaStage4UncertaintyWeightDraft"
                @input="
                  metaStage4UncertaintyWeightDraft = String(
                    ($event.target as HTMLInputElement).value || '0.05',
                  );
                  queueInputValue(
                    'metaStage4UncertaintyWeight',
                    metaStage4UncertaintyWeightDraft,
                  );
                "
                @change="
                  commitInputValue(
                    'metaStage4UncertaintyWeight',
                    metaStage4UncertaintyWeightDraft,
                  )
                "
              />
            </div>
          </div>

          <div class="gridThree" style="margin-top: 0.6rem">
            <div>
              <label for="vueMetaStage4AcceptConf">Accept confidence</label>
              <input
                id="vueMetaStage4AcceptConf"
                type="number"
                min="0"
                max="1"
                step="0.01"
                :value="metaStage4AcceptConfDraft"
                @input="
                  metaStage4AcceptConfDraft = String(
                    ($event.target as HTMLInputElement).value || '0.70',
                  );
                  queueInputValue(
                    'metaStage4AcceptConf',
                    metaStage4AcceptConfDraft,
                  );
                "
                @change="
                  commitInputValue(
                    'metaStage4AcceptConf',
                    metaStage4AcceptConfDraft,
                  )
                "
              />
            </div>
            <div>
              <label for="vueMetaStage4ReviewConf">Review confidence</label>
              <input
                id="vueMetaStage4ReviewConf"
                type="number"
                min="0"
                max="1"
                step="0.01"
                :value="metaStage4ReviewConfDraft"
                @input="
                  metaStage4ReviewConfDraft = String(
                    ($event.target as HTMLInputElement).value || '0.55',
                  );
                  queueInputValue(
                    'metaStage4ReviewConf',
                    metaStage4ReviewConfDraft,
                  );
                "
                @change="
                  commitInputValue(
                    'metaStage4ReviewConf',
                    metaStage4ReviewConfDraft,
                  )
                "
              />
            </div>
            <div>
              <label for="vueMetaStage4MaxAcceptUncertainty"
                >Max accept uncertainty</label
              >
              <input
                id="vueMetaStage4MaxAcceptUncertainty"
                type="number"
                min="0"
                max="1"
                step="0.01"
                :value="metaStage4MaxAcceptUncertaintyDraft"
                @input="
                  metaStage4MaxAcceptUncertaintyDraft = String(
                    ($event.target as HTMLInputElement).value || '0.45',
                  );
                  queueInputValue(
                    'metaStage4MaxAcceptUncertainty',
                    metaStage4MaxAcceptUncertaintyDraft,
                  );
                "
                @change="
                  commitInputValue(
                    'metaStage4MaxAcceptUncertainty',
                    metaStage4MaxAcceptUncertaintyDraft,
                  )
                "
              />
            </div>
          </div>

          <div>
            <label for="metaStage4TrainingHistory">Training history</label>
            <select
              id="metaStage4TrainingHistory"
              :value="metaStage4TrainingHistoryPolicyDraft"
              @change="
                metaStage4TrainingHistoryPolicyDraft = String(
                  ($event.target as HTMLSelectElement).value ||
                    'source_excluded',
                );
                setInputValue(
                  'metaStage4TrainingHistoryPolicy',
                  metaStage4TrainingHistoryPolicyDraft,
                );
              "
            >
              <option value="source_excluded">Source excluded</option>
              <option value="chronological_before_source">
                Chronological before source
              </option>
            </select>
          </div>

          <div class="checks">
            <label>
              <input
                type="checkbox"
                :checked="metaStage4AutoTrainDraft"
                @change="
                  metaStage4AutoTrainDraft = ($event.target as HTMLInputElement)
                    .checked;
                  setCheckedValue(
                    'metaStage4AutoTrain',
                    metaStage4AutoTrainDraft,
                  );
                "
              />
              Auto-train / retrain model
            </label>
            <label>
              <input
                type="checkbox"
                :checked="metaStage4RequireHealthyDraft"
                @change="
                  metaStage4RequireHealthyDraft = (
                    $event.target as HTMLInputElement
                  ).checked;
                  setCheckedValue(
                    'metaStage4RequireHealthy',
                    metaStage4RequireHealthyDraft,
                  );
                "
              />
              Require healthy model
            </label>
            <label>
              <input
                type="checkbox"
                :checked="metaStage4FallbackToMetaDraft"
                @change="
                  metaStage4FallbackToMetaDraft = (
                    $event.target as HTMLInputElement
                  ).checked;
                  setCheckedValue(
                    'metaStage4FallbackToMeta',
                    metaStage4FallbackToMetaDraft,
                  );
                "
              />
              Fallback to meta if empty
            </label>
          </div>

          <div class="metaLine mono">{{ metaStage4Status }}</div>
          <details class="panelDetails">
            <summary>Current live Stage 5 output</summary>
            <div class="panelDetailsBody">
              <div class="panelMuted">
                This preview reflects the latest live Stage 5 rerank output.
                Saved Stage 5 artifacts can be reopened from Saved Aggregate.
              </div>
              <pre class="mono preview">{{ metaStage4OutputPreview }}</pre>
            </div>
          </details>
          <div class="metaLine mono">{{ metaPolicyEvalStatus }}</div>
          <details class="panelDetails">
            <summary>Current live policy evaluation</summary>
            <div class="panelDetailsBody">
              <div class="panelMuted">
                This preview reflects the latest live policy-evaluation output.
                Saved policy-evaluation artifacts remain available from Saved
                Aggregate.
              </div>
              <pre class="mono preview">{{ metaPolicyEvalOutputPreview }}</pre>
            </div>
          </details>
        </section>

        <section class="pane">
          <div class="panelSectionHeader">
            <div class="panelHeadCopy">
              <div class="panelEyebrow">Selection</div>
              <h3 class="panelPaneTitle">Meta run configuration</h3>
              <div class="panelSectionLead">
                Set the strategy scope, run filters, and top-k size for the next
                meta-analysis execution.
              </div>
            </div>
          </div>
          <div class="gridFive">
            <div>
              <label for="vueMetaStrategy">Strategy</label>
              <select
                id="vueMetaStrategy"
                :value="strategyDraft"
                @change="
                  strategyDraft = String(
                    ($event.target as HTMLSelectElement).value || 'CompositeMo',
                  );
                  setSelectValue('metaStrategyKind', strategyDraft);
                "
              >
                <option
                  v-for="option in strategyOptions"
                  :key="`meta_strategy_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
            </div>
            <div>
              <label for="vueMetaTimeframe">Timeframe filter</label>
              <input
                id="vueMetaTimeframe"
                type="text"
                :value="timeframeDraft"
                placeholder="1m"
                @input="
                  timeframeDraft = String(
                    ($event.target as HTMLInputElement).value || '',
                  );
                  queueInputValue('metaTimeframe', timeframeDraft);
                "
                @change="commitInputValue('metaTimeframe', timeframeDraft)"
              />
            </div>
            <div>
              <label for="vueMetaRequiredTag">Required tag</label>
              <input
                id="vueMetaRequiredTag"
                type="text"
                :value="requiredTagDraft"
                placeholder="e.g. 1m, forex, top50"
                @input="
                  requiredTagDraft = String(
                    ($event.target as HTMLInputElement).value || '',
                  );
                  queueInputValue('metaRequiredTag', requiredTagDraft);
                "
                @change="commitInputValue('metaRequiredTag', requiredTagDraft)"
              />
            </div>
            <div>
              <label for="vueMetaMinSamples">Min samples</label>
              <input
                id="vueMetaMinSamples"
                type="number"
                min="1"
                step="1"
                :value="minSamplesDraft"
                @input="
                  minSamplesDraft = String(
                    ($event.target as HTMLInputElement).value || '30',
                  );
                  queueInputValue('metaMinSamples', minSamplesDraft);
                "
                @change="commitInputValue('metaMinSamples', minSamplesDraft)"
              />
            </div>
            <div>
              <label for="vueMetaTopK">Top K</label>
              <input
                id="vueMetaTopK"
                type="number"
                min="1"
                step="1"
                :value="topKDraft"
                @input="
                  topKDraft = String(
                    ($event.target as HTMLInputElement).value || '25',
                  );
                  queueInputValue('metaTopK', topKDraft);
                "
                @change="commitInputValue('metaTopK', topKDraft)"
              />
            </div>
          </div>
          <div class="actions">
            <button
              type="button"
              class="primary"
              :disabled="!canRunMeta"
              @click="runAction(() => analysisStore.runMeta())"
            >
              Run meta analysis
            </button>
            <button
              type="button"
              :disabled="!canOpenMetaReport"
              @click="runAction(() => analysisStore.openMetaReport())"
            >
              Open meta report (XLSX)
            </button>
            <button
              type="button"
              :disabled="!canRevealMetaFiles"
              @click="runAction(() => analysisStore.revealMetaFiles())"
            >
              Reveal files
            </button>
          </div>
        </section>

        <section class="pane">
          <div class="panelSectionHeader">
            <div class="panelHeadCopy">
              <div class="panelEyebrow">Grouping</div>
              <h3 class="panelPaneTitle">Holdout + Grouping</h3>
              <div class="panelSectionLead">
                Control holdout evaluation, grouping strictness, and clustering
                so the aggregate selection logic stays comparable.
              </div>
            </div>
          </div>
          <div class="summaryGrid" aria-label="Stage 3 meta selection contract">
            <article>
              <div class="panelMetricKey">Scope</div>
              <div class="panelMetricValue mono">
                {{ stage3RequestScopeSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Holdout</div>
              <div class="panelMetricValue mono">
                {{ stage3HoldoutContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Grouping</div>
              <div class="panelMetricValue mono">
                {{ stage3GroupingContractSummary }}
              </div>
            </article>
            <article>
              <div class="panelMetricKey">Shortlist</div>
              <div class="panelMetricValue mono">
                {{ stage3ShortlistContractSummary }}
              </div>
            </article>
          </div>
          <div class="panelMuted">{{ stage3ScoreContractSummary }}</div>
          <div class="panelMuted">{{ stage3EvidenceContractSummary }}</div>
          <div class="panelMuted">{{ stage3HoldoutModeDetailSummary }}</div>
          <div class="gridThree">
            <div>
              <label for="vueMetaHoldoutTag">Holdout tag</label>
              <input
                id="vueMetaHoldoutTag"
                type="text"
                :value="holdoutTagDraft"
                @input="
                  holdoutTagDraft = String(
                    ($event.target as HTMLInputElement).value || 'holdout',
                  );
                  queueInputValue('metaHoldoutTag', holdoutTagDraft);
                "
                @change="commitInputValue('metaHoldoutTag', holdoutTagDraft)"
              />
            </div>
            <div>
              <label for="vueMetaHoldoutMode">Holdout mode</label>
              <select
                id="vueMetaHoldoutMode"
                :value="holdoutModeDraft"
                @change="
                  holdoutModeDraft = String(
                    ($event.target as HTMLSelectElement).value || 'light',
                  );
                  setSelectValue('metaHoldoutMode', holdoutModeDraft);
                "
              >
                <option
                  v-for="option in holdoutModeOptions"
                  :key="`meta_holdout_mode_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
            </div>
            <div>
              <label for="vueMetaHoldoutReps">Holdout reps per cluster</label>
              <select
                id="vueMetaHoldoutReps"
                :value="holdoutRepsDraft"
                @change="
                  holdoutRepsDraft = String(
                    ($event.target as HTMLSelectElement).value || '2',
                  );
                  setSelectValue('metaHoldoutReps', holdoutRepsDraft);
                "
              >
                <option
                  v-for="option in holdoutRepsOptions"
                  :key="`meta_holdout_reps_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
            </div>
          </div>

          <div class="checks">
            <label>
              <input
                type="checkbox"
                :checked="holdoutEvalDraft"
                @change="
                  holdoutEvalDraft = ($event.target as HTMLInputElement)
                    .checked;
                  setCheckedValue('metaEvalHoldout', holdoutEvalDraft);
                "
              />
              Evaluate holdout (recommended)
            </label>
            <label>
              <input
                type="checkbox"
                :checked="holdoutSaveRowsDraft"
                @change="
                  holdoutSaveRowsDraft = ($event.target as HTMLInputElement)
                    .checked;
                  setCheckedValue('metaSaveHoldoutRows', holdoutSaveRowsDraft);
                "
              />
              Save per-dataset holdout rows (for ML)
            </label>
          </div>

          <div class="gridThree" style="margin-top: 0.6rem">
            <div>
              <label for="vueMetaHoldoutEvalTopN"
                >Holdout eval top N (0 = all)</label
              >
              <input
                id="vueMetaHoldoutEvalTopN"
                type="number"
                min="0"
                step="1"
                :value="holdoutEvalTopNDraft"
                @input="
                  holdoutEvalTopNDraft = String(
                    ($event.target as HTMLInputElement).value || '0',
                  );
                  queueInputValue('metaHoldoutEvalTopN', holdoutEvalTopNDraft);
                "
                @change="
                  commitInputValue('metaHoldoutEvalTopN', holdoutEvalTopNDraft)
                "
              />
            </div>
            <div>
              <label for="vueMetaHoldoutEvalRankBy">Holdout rank by</label>
              <select
                id="vueMetaHoldoutEvalRankBy"
                :value="holdoutEvalRankByDraft"
                @change="
                  holdoutEvalRankByDraft = String(
                    ($event.target as HTMLSelectElement).value || 'auto',
                  );
                  setSelectValue(
                    'metaHoldoutEvalRankBy',
                    holdoutEvalRankByDraft,
                  );
                "
              >
                <option
                  v-for="option in holdoutRankByOptions"
                  :key="`meta_holdout_rank_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
            </div>
            <div>
              <label for="vueMetaHoldoutExploreN"
                >Explore additional finalists (optional)</label
              >
              <input
                id="vueMetaHoldoutExploreN"
                type="number"
                min="0"
                step="1"
                :value="holdoutExploreNDraft"
                @input="
                  holdoutExploreNDraft = String(
                    ($event.target as HTMLInputElement).value || '0',
                  );
                  queueInputValue('metaHoldoutExploreN', holdoutExploreNDraft);
                "
                @change="
                  commitInputValue('metaHoldoutExploreN', holdoutExploreNDraft)
                "
              />
            </div>
          </div>

          <div class="gridThree" style="margin-top: 0.6rem">
            <div>
              <label for="vueMetaGroupingMode">Grouping</label>
              <select
                id="vueMetaGroupingMode"
                :value="groupingModeDraft"
                @change="
                  groupingModeDraft = String(
                    ($event.target as HTMLSelectElement).value || 'clustered',
                  );
                  setSelectValue('metaGroupingMode', groupingModeDraft);
                "
              >
                <option
                  v-for="option in groupingModeOptions"
                  :key="`meta_grouping_${option.value}_${option.label}`"
                  :value="option.value"
                  :disabled="option.disabled"
                >
                  {{ option.label }}
                </option>
              </select>
            </div>
            <div>
              <label for="vueMetaClusterIntBucket">Integer bucket size</label>
              <input
                id="vueMetaClusterIntBucket"
                type="number"
                min="1"
                step="1"
                :value="clusterIntBucketDraft"
                @input="
                  clusterIntBucketDraft = String(
                    ($event.target as HTMLInputElement).value || '5',
                  );
                  queueInputValue(
                    'metaClusterIntBucket',
                    clusterIntBucketDraft,
                  );
                "
                @change="
                  commitInputValue(
                    'metaClusterIntBucket',
                    clusterIntBucketDraft,
                  )
                "
              />
            </div>
            <div>
              <label for="vueMetaClusterFloatBucket">Float bucket size</label>
              <input
                id="vueMetaClusterFloatBucket"
                type="number"
                min="0"
                step="0.01"
                :value="clusterFloatBucketDraft"
                @input="
                  clusterFloatBucketDraft = String(
                    ($event.target as HTMLInputElement).value || '0.25',
                  );
                  queueInputValue(
                    'metaClusterFloatBucket',
                    clusterFloatBucketDraft,
                  );
                "
                @change="
                  commitInputValue(
                    'metaClusterFloatBucket',
                    clusterFloatBucketDraft,
                  )
                "
              />
            </div>
          </div>
          <div class="metaLine">{{ metaHoldoutNote }}</div>
        </section>

        <section class="pane">
          <div class="panelSectionHeader">
            <div class="panelHeadCopy">
              <div class="panelEyebrow">Results</div>
              <h3 class="panelPaneTitle">Meta selection table</h3>
              <div class="panelSectionLead">
                Review grouped aggregate recommendations, holdout outcomes, and
                selection notes for the current meta run.
              </div>
            </div>
          </div>
          <div class="metaLine mono" role="status" aria-live="polite">
            {{ metaMeta }}
          </div>
          <AppDataTableShell
            title="Meta selection rows"
            :summary="metaTableShellSummary"
            :row-count="renderedRows.length"
            :viewport="false"
            aria-label="Meta analysis results table shell"
            keyboard-hint="Meta rows are virtualized; spacer rows are hidden from assistive technology."
          >
            <template #controls>
              <details v-if="renderedColumns.length > 0" class="columnDetails">
                <summary class="columnDetailsSummary">
                  <span>Visible columns</span>
                  <span class="mono panelNumeric">{{
                    columnVisibilitySummary
                  }}</span>
                </summary>
                <div class="columnDetailsBody">
                  <div class="panelMuted">{{ columnVisibilityHint }}</div>
                  <div
                    class="columnToggleGrid"
                    role="group"
                    aria-label="Visible meta result columns"
                  >
                    <label
                      v-for="column in columnOptions"
                      :key="column.key"
                      class="columnToggle"
                      :class="{
                        disabled: !canHideColumn(column.key) && column.visible,
                      }"
                      :for="metaColumnToggleId(column.key)"
                    >
                      <input
                        :id="metaColumnToggleId(column.key)"
                        type="checkbox"
                        :checked="column.visible"
                        :disabled="!canHideColumn(column.key) && column.visible"
                        @change="toggleColumnVisibility(column.key)"
                      />
                      <span>{{ column.label }}</span>
                    </label>
                  </div>
                  <div class="actions columnDetailsActions">
                    <button
                      type="button"
                      :disabled="!canRestoreDefaults"
                      @click="restoreDefaultColumns()"
                    >
                      Restore defaults
                    </button>
                  </div>
                </div>
              </details>
            </template>
            <div
              ref="tableScrollElementRef"
              class="tableWrap overflowShadowY overflowShadowX surfaceWorking"
              tabindex="0"
              aria-label="Meta analysis results table"
            >
              <table class="metaTableVue">
                <caption class="srOnly">
                  Meta analysis selection results grouped by the configured
                  strategy, holdout, and clustering rules.
                </caption>
                <thead>
                  <tr v-for="headerGroup in headerGroups" :key="headerGroup.id">
                    <th
                      v-for="header in headerGroup.headers"
                      :key="header.id"
                      :class="{ left: headerMeta(header).alignLeft }"
                      :style="metaHeaderStyle(header)"
                      scope="col"
                    >
                      <span>{{ headerMeta(header).columnLabel }}</span>
                      <button
                        v-if="!header.isPlaceholder"
                        type="button"
                        class="columnResizeHandle"
                        :class="{ active: resizingMetaColumnKey === headerMeta(header).columnKey }"
                        :aria-label="`Resize ${headerMeta(header).columnLabel} column`"
                        title="Drag to resize. Use Left/Right arrows to adjust, Delete to reset."
                        @click.stop
                        @pointerdown="startMetaColumnResize($event, header)"
                        @keydown.stop="onMetaColumnResizeKeydown($event, header)"
                      ></button>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-if="virtualTopSpacerPx > 0"
                    class="spacerRow"
                    aria-hidden="true"
                  >
                    <td
                      :colspan="Math.max(1, visibleLeafColumns.length)"
                      :style="{ height: `${virtualTopSpacerPx}px` }"
                    ></td>
                  </tr>
                  <tr v-for="virtualRow in virtualRows" :key="virtualRow.row.id">
                    <template
                      v-for="cell in virtualRow.row.getVisibleCells()"
                      :key="cell.id"
                    >
                      <td
                        :class="metaCellPayload(cell).className"
                        :style="metaCellStyle(cell)"
                        :title="metaCellPayload(cell).title || undefined"
                      >
                        {{ metaCellPayload(cell).text }}
                      </td>
                    </template>
                  </tr>
                  <tr
                    v-if="virtualBottomSpacerPx > 0"
                    class="spacerRow"
                    aria-hidden="true"
                  >
                    <td
                      :colspan="Math.max(1, visibleLeafColumns.length)"
                      :style="{ height: `${virtualBottomSpacerPx}px` }"
                    ></td>
                  </tr>
                </tbody>
              </table>
            </div>
          </AppDataTableShell>
          <details class="panelDetails">
            <summary>Meta output preview</summary>
            <div class="panelDetailsBody">
              <pre class="mono preview">{{ metaOutputPreview }}</pre>
            </div>
          </details>
        </section>
      </div>

      <section
        v-if="showMetaTooling && compareTabActive"
        id="meta_aggregate_panel_compare_main"
        class="pane"
        role="tabpanel"
        :aria-labelledby="aggregateTabId('compare')"
      >
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Comparison</div>
            <h3 class="panelPaneTitle">Compare meta baseline</h3>
            <div class="panelSectionLead">
              Choose a saved meta artifact as the baseline and inspect the
              provenance summary before accepting a new aggregate baseline.
            </div>
          </div>
          <ContextHelpTip
            title="Meta baseline compare"
            summary="Use Compare to hold the current Meta Selection output against one Saved Meta artifact at a time."
            detail="The compare pane intentionally uses only Saved Meta artifacts as the baseline family. Saved Stage 5 and Saved Policy Evaluation outputs remain in the Saved Aggregate library because they represent later-stage decisions, not a direct meta-selection baseline."
            aria-label="Meta compare help"
          />
        </div>
        <div class="summaryGrid" aria-label="Meta compare provenance">
          <article>
            <div class="panelMetricKey">Compare source</div>
            <div class="panelMetricValue">Saved Meta artifact</div>
          </article>
          <article>
            <div class="panelMetricKey">Current candidate</div>
            <div class="panelMetricValue">Active Meta Selection output</div>
          </article>
          <article>
            <div class="panelMetricKey">Selected baseline</div>
            <div class="panelMetricValue mono panelNumeric">
              {{ selectedCompareLabel }}
            </div>
          </article>
          <article>
            <div class="panelMetricKey">Baseline file</div>
            <div class="panelMetricValue mono panelNumeric">
              {{ selectedCompareFile }}
            </div>
          </article>
        </div>
        <div class="panelMuted">
          {{ compareSummaryLead }}
        </div>
        <label for="vueMetaCompareSelect">Saved meta baseline</label>
        <select
          id="vueMetaCompareSelect"
          :value="compareDraft"
          @change="
            compareDraft = String(
              ($event.target as HTMLSelectElement).value || '',
            );
            setSelectValue('metaCompareSelect', compareDraft);
          "
        >
          <option
            v-for="option in compareOptions"
            :key="`meta_compare_${option.value}_${option.label}`"
            :value="option.value"
            :disabled="option.disabled"
          >
            {{ option.label }}
          </option>
        </select>
        <details class="panelDetails">
          <summary>Compare summary</summary>
          <div class="panelDetailsBody">
            <pre class="mono preview">{{ metaCompareSummary }}</pre>
          </div>
        </details>
      </section>

      <div
        class="panelStatus mono panelNumeric"
        role="status"
        aria-live="polite"
      >
        {{ statusLine }}
      </div>
      <ConfirmationDialog
        id-base="meta_saved_artifact_confirmation"
        :open="pendingSavedArtifactDeleteFamily !== null"
        :title="savedArtifactDeleteTitle"
        :body="savedArtifactDeleteBody"
        confirm-label="Delete artifact"
        :confirm-disabled="!canConfirmSavedArtifactDelete"
        danger
        @cancel="cancelPendingSavedArtifactDelete"
        @confirm="confirmPendingSavedArtifactDelete"
      />
    </section>
  </Teleport>
</template>

<style scoped>
.metaPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.subtabs {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.subtabs button {
  padding: 0.4rem 0.58rem;
  font-size: 0.75rem;
  cursor: pointer;
}

.subtabs button.active {
  border-color: color-mix(in oklab, var(--accent) 65%, var(--border));
}

.subtabs button:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring);
}

.aggregatePanelStack {
  display: grid;
  gap: 0.85rem;
}

.savedArtifactPane {
  gap: 0.75rem;
}

.savedArtifactFamilyTabs {
  margin-top: 0.2rem;
  margin-bottom: 0.1rem;
}

.savedArtifactHeading {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.summaryGrid {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 0.55rem;
}

.summaryGrid article {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0.5rem 0.55rem;
  background: color-mix(in oklab, var(--panel) 88%, var(--panel-footer));
}

.gridFive {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.6rem;
}

.gridFour {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.6rem;
}

.gridThree {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.6rem;
}

.actions {
  margin-top: 0.65rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.buttonWithIcon {
  display: inline-flex;
  align-items: center;
  gap: 0.42rem;
}

.actions .primary {
  border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
}

.actionIcon,
.sectionGlyph {
  width: 0.98rem;
  height: 0.98rem;
  flex: 0 0 auto;
}

.checks {
  margin-top: 0.6rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.7rem;
}

.metaLine {
  font-size: 0.78rem;
  color: var(--muted);
}

.tableWrap {
  margin-top: 0.55rem;
  overflow: auto;
}

.columnDetails {
  margin-top: 0.55rem;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface-sunken);
}

.columnDetailsSummary {
  list-style: none;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  cursor: pointer;
  padding: calc(var(--density-pane-y) - 0.12rem)
    calc(var(--density-pane-x) - 0.02rem);
  font-size: 0.78rem;
  font-weight: 600;
}

.columnDetailsSummary::-webkit-details-marker {
  display: none;
}

.columnDetailsBody {
  border-top: 1px solid var(--border);
  padding: calc(var(--density-pane-y) - 0.05rem)
    calc(var(--density-pane-x) - 0.02rem);
  display: grid;
  gap: 0.65rem;
}

.columnToggleGrid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.45rem 0.75rem;
}

.columnToggle {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
  font-size: 0.76rem;
}

.columnToggle.disabled {
  opacity: 0.62;
}

.columnToggle input {
  margin: 0;
}

.columnDetailsActions {
  margin-top: 0;
}

.metaTableVue {
  width: 100%;
  border-collapse: collapse;
  min-width: 1080px;
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

.metaTableVue th,
.metaTableVue td {
  border-bottom: 1px solid var(--border);
  padding: 0.34rem 0.42rem;
  text-align: right;
  vertical-align: top;
}

.metaTableVue th {
  position: relative;
  user-select: none;
}

.columnResizeHandle {
  position: absolute;
  inset: 0 0 0 auto;
  width: 0.48rem;
  min-width: 0.48rem;
  min-height: 100%;
  border: 0;
  border-radius: 0;
  padding: 0;
  background: transparent;
  cursor: col-resize;
}

.columnResizeHandle::after {
  content: "";
  position: absolute;
  top: 24%;
  bottom: 24%;
  right: 0.16rem;
  width: 2px;
  border-radius: 999px;
  background: color-mix(in oklab, var(--border) 72%, transparent);
}

.columnResizeHandle:hover::after,
.columnResizeHandle:focus-visible::after,
.columnResizeHandle.active::after {
  background: var(--focus-border);
}

.columnResizeHandle:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: -2px;
}

.metaTableVue th.left,
.metaTableVue td.left {
  text-align: left;
}

.metaTableVue tbody tr.spacerRow {
  pointer-events: none;
}

.metaTableVue tbody tr.spacerRow td {
  border-bottom: none;
  padding: 0;
}

.batchList {
  margin-top: 0.55rem;
  max-height: 300px;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0.25rem;
}

.batchItem {
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 0.42rem 0.5rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.7rem;
  cursor: pointer;
}

.batchItem:hover {
  border-color: var(--border);
  background: color-mix(in oklab, var(--panel) 92%, var(--panel-footer));
}

.batchItem:focus-within {
  border-color: color-mix(in oklab, var(--accent) 62%, var(--border));
  box-shadow: var(--focus-ring);
  background: color-mix(in oklab, var(--panel) 90%, var(--panel-footer));
}

.batchItem.disabled {
  cursor: not-allowed;
  opacity: 0.64;
}

.rowAuto {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.45rem;
}

.riskNotice {
  border: 1px solid color-mix(in oklab, var(--warning-ink) 34%, var(--border));
  border-radius: 12px;
  background: color-mix(
    in oklab,
    var(--surface-warning) 86%,
    var(--panel-footer)
  );
  padding: 0.8rem 0.9rem;
}

.mlRiskNotice {
  margin-top: 0.55rem;
}

.riskTitle {
  font-size: 0.86rem;
  font-weight: 650;
  color: var(--warning-ink);
}

.riskBody {
  margin-top: 0.35rem;
  color: var(--text);
  font-size: 0.82rem;
}

.preview {
  margin-top: 0.45rem;
  max-height: 220px;
  overflow: auto;
}

.emptyStateIllustration {
  width: clamp(88px, 16vw, 124px);
  height: auto;
  margin-bottom: 0.2rem;
  opacity: 0.92;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1220px) {
  .summaryGrid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .gridFive,
  .gridFour,
  .gridThree {
    grid-template-columns: 1fr 1fr;
  }

  .columnToggleGrid {
    grid-template-columns: 1fr 1fr;
  }
}

@media (max-width: 740px) {
  .summaryGrid,
  .gridFive,
  .gridFour,
  .gridThree {
    grid-template-columns: 1fr;
  }

  .columnToggleGrid {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel="meta"][data-vue-meta="1"] > :not(#metaVueShellMount) {
  display: none !important;
}
</style>
