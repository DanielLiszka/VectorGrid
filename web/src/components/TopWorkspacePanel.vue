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
import AppVisualIcon from "./AppVisualIcon.vue";
import {
  useHeadlessMatrixTable,
  type MatrixColumnMeta,
} from "../composables/useHeadlessMatrixTable";
import {
  usePersistedMatrixColumnState,
  type MatrixColumnViewPreset,
} from "../composables/usePersistedMatrixColumnState";
import { usePersistedTableDensity } from "../composables/usePersistedTableDensity";
import { useShellMount } from "../composables/useShellMount";
import { useUiReadiness } from "../composables/useUiReadiness";
import { useVirtualTableRows } from "../composables/useVirtualTableRows";
import { useQueuedInputWriter } from "../composables/useQueuedInputWriter";
import { useAnalysisStore } from "../stores/analysisStore";
import { useWorkflowStore } from "../stores/workflowStore";
import {
  TOP_RESULTS_TABLE_DENSITY_KEY,
  TOP_RESULTS_TABLE_STATE_KEY,
} from "../utils/core/storageKeys";
import {
  compareCandidateFocusIcon,
  compareCandidatesIcon,
  emptyResultsWorkspaceIllustration,
  provenanceIconForText,
} from "../utils/appSvgAssets";
import { compareActionIcon } from "../utils/iconography";
import { formatUiTimeFromUnixMs } from "../utils/uiTime";
import AppDataTableShell from "./ui/AppDataTableShell.vue";
import AppMetricCell from "./ui/AppMetricCell.vue";
import AppMetricStrip, { type AppMetricItem } from "./ui/AppMetricStrip.vue";
import AppTableDensityControl from "./ui/AppTableDensityControl.vue";
import AppTableViewPresetControl from "./ui/AppTableViewPresetControl.vue";
import type { TableViewPresetOption } from "./ui/tableViewPresets";
import {
  buildMetricColumnVisualRanges,
  resolveMetricCellVisual,
  type MetricCellVisual,
} from "../visualizations/metricCellVisualModel";

interface TopColumn {
  key: string;
  label: string;
  sort: "" | "asc" | "desc";
  alignLeft: boolean;
}

interface TopCell {
  text: string;
  className: string;
  title: string;
}

interface TopRow {
  rowId?: string;
  selected: boolean;
  cells: TopCell[];
}

type TopViewPresetId = "default" | "risk" | "score";

const TOP_VIEW_PRESETS: readonly TableViewPresetOption[] = [
  {
    id: "default",
    label: "Default",
    summary: "Original shortlist column order.",
  },
  {
    id: "risk",
    label: "Risk",
    summary: "Prioritize PnL, Sharpe, drawdown, trades, and exposure.",
  },
  {
    id: "score",
    label: "Score",
    summary: "Prioritize objective and derived score columns.",
  },
];

const TOP_VIEW_PRESET_KEYS: Record<TopViewPresetId, readonly string[]> = {
  default: [],
  risk: [
    "rank",
    "pnl",
    "ret_pct",
    "sharpe",
    "max_dd",
    "trades",
    "exposure",
    "fast",
    "slow",
  ],
  score: [
    "rank",
    "score",
    "maximize_value",
    "minimize_value",
    "violation",
    "active_mean",
    "active_sortino",
    "drawdown_chronicity",
    "robustness_margin",
    "fast",
    "slow",
    "pnl",
    "sharpe",
    "max_dd",
  ],
};

const analysisStore = useAnalysisStore();
const workflowStore = useWorkflowStore();
const { topStage } = useUiReadiness();
const pinCompareIcon = compareActionIcon("pin");
const clearCompareIcon = compareActionIcon("clear");
const replaceCompareIcon = compareActionIcon("replace");
const removeCompareIcon = compareActionIcon("remove");
const {
  bestKicker,
  bestStats,
  bestSummary,
  filterMinPnl,
  filterMinSharpe,
  filterMaxDd,
  filterMinTrades,
  filterMinExposure,
  pinnedTopCandidates,
  selectedTopDecisionNote,
  selectedTopProvenanceChips,
  selectedTopSummary,
  selectedTopTitle,
  canClearTopFilters,
  topRows,
  topColumns,
  topTableRows,
  topFilterMeta,
  topSelectionGuardrail,
  topStaleCues,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore);
const { resultsTab } = storeToRefs(workflowStore);

const { mountReady } = useShellMount("#topVueShellMount");

const minPnlDraft = ref("");
const minSharpeDraft = ref("");
const maxDdDraft = ref("");
const minTradesDraft = ref("");
const minExposureDraft = ref("");

const visible = computed(() => String(resultsTab.value || "").trim() === "top");
const renderedColumns = computed<TopColumn[]>(
  () => topColumns.value as TopColumn[],
);
const renderedRows = computed<TopRow[]>(() => topTableRows.value as TopRow[]);
const {
  columnOptions,
  columnVisibility,
  visibleCount: visibleColumnCount,
  hiddenCount: hiddenColumnCount,
  canRestoreDefaults,
  leadVisibleColumnKey,
  allColumnKeys,
  orderedColumns,
  orderedVisibleColumnKeys,
  columnWidths,
  toggleColumnVisibility,
  restoreDefaultColumns,
  canHideColumn,
  applyColumnViewPreset,
  setColumnWidth,
  resizeColumnWidth,
  clearColumnWidth,
  columnWidthStyle,
} = usePersistedMatrixColumnState(renderedColumns, TOP_RESULTS_TABLE_STATE_KEY);
const { tableDensity } = usePersistedTableDensity(
  TOP_RESULTS_TABLE_DENSITY_KEY,
);
const { headerGroups, rowModelRows, visibleLeafColumns, toggleSortColumn } =
  useHeadlessMatrixTable(orderedColumns, renderedRows, {
    columnVisibility,
    columnWidths,
    onSortColumn: (columnKey) => {
      analysisStore.sortTopTable(columnKey);
    },
  });
const topMetricVisualRanges = computed(() =>
  buildMetricColumnVisualRanges(orderedColumns.value, renderedRows.value),
);
const hasTopResults = computed(() => Number(topRows.value || 0) > 0);
const topDisabledForRun = computed(() =>
  String(topFilterMeta.value || "")
    .toLowerCase()
    .includes("top k disabled"),
);
const filtersActive = computed(() =>
  [
    minPnlDraft.value,
    minSharpeDraft.value,
    maxDdDraft.value,
    minTradesDraft.value,
    minExposureDraft.value,
  ].some((value) => String(value || "").trim().length > 0),
);
const emptyTitle = computed(() => {
  if (topStage.value === "no_data")
    return "Load data to unlock ranked results.";
  if (topStage.value === "data_loaded_no_run") return "No ranked runs yet.";
  if (topDisabledForRun.value) return "This run returned only the best winner.";
  return filtersActive.value
    ? "No rows match the current filters."
    : "No ranked runs yet.";
});
const emptyBody = computed(() =>
  topStage.value === "no_data"
    ? "Load a dataset first, then run an optimization to populate the ranking table."
    : topStage.value === "data_loaded_no_run"
      ? "Your dataset is ready. Run an optimization or load prior analysis results to populate the ranked table."
      : topDisabledForRun.value
        ? "Increase Advanced > Top K above 0 and rerun optimization to populate the ranked shortlist table."
        : filtersActive.value
          ? "Broaden or clear the active filters to repopulate the ranking table."
          : "Run an optimization or load prior analysis results to populate the ranked table.",
);
const {
  scrollElementRef: tableScrollElementRef,
  virtualRows,
  topSpacerPx: virtualTopSpacerPx,
  bottomSpacerPx: virtualBottomSpacerPx,
  scrollToTop,
} = useVirtualTableRows(rowModelRows);

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value));

const topStageLabel = computed(() => {
  if (topStage.value === "no_data") return "Awaiting dataset";
  if (topStage.value === "data_loaded_no_run") return "Awaiting optimization";
  if (topDisabledForRun.value) return "Winner only";
  return hasTopResults.value ? "Ranked shortlist ready" : "No matching rows";
});

const topSummaryChips = computed(() => {
  return [
    topStageLabel.value,
    `${topRows.value} rows`,
    filtersActive.value ? "Filters active" : "All rows",
    topDisabledForRun.value ? "Top-k disabled" : "",
  ].filter((chip) => String(chip || "").trim().length > 0);
});
const columnVisibilitySummary = computed(
  () => `Columns ${visibleColumnCount.value}/${renderedColumns.value.length}`,
);
const topTableShellSummary = computed(() => {
  const rowCount = renderedRows.value.length;
  return `${rowCount} row${rowCount === 1 ? "" : "s"} · ${columnVisibilitySummary.value}`;
});
const currentCandidateRankLabel = computed(() =>
  selectedTopIndex.value >= 0 ? `#${selectedTopIndex.value + 1}` : "None",
);
const topDecisionStats = computed<AppMetricItem[]>(() => [
  { key: "selected", label: "Selected", value: currentCandidateRankLabel.value },
  { key: "rows", label: "Rows", value: String(topRows.value || 0) },
  { key: "pinned", label: "Pinned", value: String(pinnedTopCandidates.value.length || 0) },
  { key: "visible-cols", label: "Visible cols", value: `${visibleColumnCount.value}/${renderedColumns.value.length}` },
  { key: "density", label: "Density", value: tableDensity.value },
]);
const columnVisibilityHint = computed(() => {
  if (hiddenColumnCount.value > 0) {
    return `${hiddenColumnCount.value} hidden columns. Restore defaults to return to the full shortlist layout.`;
  }
  return "Hide lower-signal metrics to keep the shortlist focused without changing the selected candidate context.";
});
function topViewPresetFor(presetId: TopViewPresetId): MatrixColumnViewPreset {
  if (presetId === "default") {
    return {
      visibleColumnKeys: allColumnKeys.value,
      columnOrderKeys: allColumnKeys.value,
    };
  }
  const orderedKeys = TOP_VIEW_PRESET_KEYS[presetId].filter((key) =>
    allColumnKeys.value.includes(key),
  );
  const fallbackKeys = allColumnKeys.value.filter(
    (key) => !orderedKeys.includes(key),
  );
  return {
    visibleColumnKeys: orderedKeys.length ? orderedKeys : allColumnKeys.value,
    columnOrderKeys: [...orderedKeys, ...fallbackKeys],
  };
}

const activeTopViewPresetId = computed<TopViewPresetId>(() => {
  const visibleKey = orderedVisibleColumnKeys.value.join("|");
  if (visibleKey === topViewPresetFor("risk").visibleColumnKeys?.join("|"))
    return "risk";
  if (visibleKey === topViewPresetFor("score").visibleColumnKeys?.join("|"))
    return "score";
  return "default";
});

function applyTopViewPreset(presetId: string): void {
  if (presetId !== "default" && presetId !== "risk" && presetId !== "score")
    return;
  applyColumnViewPreset(topViewPresetFor(presetId));
}
const selectedTopIndex = computed(() =>
  renderedRows.value.findIndex((row) => row.selected),
);
const fallbackSelectedTopTitle = computed(() => {
  const selectedIndex = selectedTopIndex.value;
  if (selectedIndex < 0) return "No ranked candidate selected.";
  const selectedRow = renderedRows.value[selectedIndex];
  const title = String(
    selectedRow?.cells?.[1]?.text || selectedRow?.cells?.[0]?.text || "",
  ).trim();
  return title
    ? `Rank #${selectedIndex + 1} · ${title}`
    : `Rank #${selectedIndex + 1}`;
});
const fallbackSelectedTopProvenanceChips = computed(() => {
  const chips: string[] = [];
  if (selectedTopIndex.value === 0 && hasTopResults.value)
    chips.push("Current leader");
  if (filtersActive.value) chips.push("Filtered shortlist");
  const kicker = String(bestKicker.value || "").trim();
  if (kicker && kicker !== "—") chips.push(kicker);
  for (const stat of bestStats.value.slice(0, 2)) {
    const label = String(stat?.label || "").trim();
    const value = String(stat?.value || "").trim();
    if (!label || !value) continue;
    chips.push(`${label} ${value}`);
  }
  return chips;
});
const fallbackSelectedTopDecisionNote = computed(() => {
  const summary = String(bestSummary.value || "").trim();
  if (!summary || summary === "-") {
    if (selectedTopIndex.value < 0)
      return "Select a ranked candidate to lock a review target before moving into Candidate Detail or Validation.";
    return "Use the current candidate as the working review target, then pin it if you need a side-by-side compare stack.";
  }
  return summary;
});
const fallbackSelectedTopSummary = computed(() => {
  const selectedIndex = selectedTopIndex.value;
  if (selectedIndex < 0) {
    return hasTopResults.value
      ? "No ranked candidate selected."
      : "No ranked candidates are available.";
  }
  const selectedRow = renderedRows.value[selectedIndex];
  if (!selectedRow) {
    return hasTopResults.value
      ? "No ranked candidate selected."
      : "No ranked candidates are available.";
  }
  const leadCells = selectedRow.cells
    .map((cell) => String(cell.text || "").trim())
    .filter((value) => value.length > 0)
    .slice(0, 3);
  const leadSummary = leadCells.join(" · ");
  return leadSummary
    ? `Selected candidate ${selectedIndex + 1} of ${renderedRows.value.length}: ${leadSummary}.`
    : `Selected candidate ${selectedIndex + 1} of ${renderedRows.value.length}.`;
});
const hasExplicitSelectedTopState = computed(() => {
  const explicitTitle = String(selectedTopTitle.value || "").trim();
  const explicitSummary = String(selectedTopSummary.value || "").trim();
  const explicitDecisionNote = String(
    selectedTopDecisionNote.value || "",
  ).trim();
  const explicitChips = Array.isArray(selectedTopProvenanceChips.value)
    ? selectedTopProvenanceChips.value
        .map((value) => String(value || "").trim())
        .filter((value) => value.length > 0)
    : [];
  const explicitStaleCues = Array.isArray(topStaleCues.value)
    ? topStaleCues.value
        .map((value) => String(value || "").trim())
        .filter((value) => value.length > 0)
    : [];
  return (
    explicitChips.length > 0 ||
    explicitStaleCues.length > 0 ||
    explicitSummary.startsWith("Selected candidate ") ||
    (explicitTitle.length > 0 &&
      explicitTitle !== "No ranked candidate selected.") ||
    (explicitSummary.length > 0 &&
      explicitSummary !== "No ranked candidate selected." &&
      explicitSummary !== "No ranked candidates are available.") ||
    (explicitDecisionNote.length > 0 &&
      explicitDecisionNote !==
        "Select a ranked candidate to lock a review target before moving into Candidate Detail or Validation.")
  );
});
const displayedSelectedTopTitle = computed(() => {
  const explicit = String(selectedTopTitle.value || "").trim();
  return hasExplicitSelectedTopState.value
    ? explicit || fallbackSelectedTopTitle.value
    : fallbackSelectedTopTitle.value;
});
const displayedSelectedTopSummary = computed(() => {
  const explicit = String(selectedTopSummary.value || "").trim();
  return hasExplicitSelectedTopState.value
    ? explicit || fallbackSelectedTopSummary.value
    : fallbackSelectedTopSummary.value;
});
const displayedSelectedTopDecisionNote = computed(() => {
  const explicit = String(selectedTopDecisionNote.value || "").trim();
  return hasExplicitSelectedTopState.value
    ? explicit || fallbackSelectedTopDecisionNote.value
    : fallbackSelectedTopDecisionNote.value;
});
const displayedSelectedTopProvenanceChips = computed(() => {
  const explicit = Array.isArray(selectedTopProvenanceChips.value)
    ? selectedTopProvenanceChips.value
        .map((value) => String(value || "").trim())
        .filter((value) => value.length > 0)
    : [];
  return hasExplicitSelectedTopState.value && explicit.length
    ? explicit
    : fallbackSelectedTopProvenanceChips.value;
});
const displayedTopStaleCues = computed(() =>
  Array.isArray(topStaleCues.value)
    ? topStaleCues.value
        .map((value) => String(value || "").trim())
        .filter(
          (value, index, items) =>
            value.length > 0 && value !== "—" && items.indexOf(value) === index,
        )
    : [],
);
const hasTopSelectionGuardrail = computed(() => {
  const next = String(topSelectionGuardrail.value || "").trim();
  return next.length > 0 && next !== "—";
});
const pinnedTopCandidateIds = computed(
  () =>
    new Set(
      pinnedTopCandidates.value
        .map((candidate) => String(candidate.id || "").trim())
        .filter((value) => value.length > 0),
    ),
);
const selectedTopRowId = computed(() => {
  const selectedRow =
    selectedTopIndex.value >= 0
      ? renderedRows.value[selectedTopIndex.value]
      : null;
  return String(selectedRow?.rowId || "").trim();
});
const canPinSelectedTopCandidate = computed(
  () =>
    selectedTopIndex.value >= 0 &&
    !!selectedTopRowId.value &&
    !pinnedTopCandidateIds.value.has(selectedTopRowId.value),
);
const canReplaceWithCurrent = computed(
  () => selectedTopIndex.value >= 0 && !!selectedTopRowId.value,
);
const canOpenCandidateDetail = computed(
  () => selectedTopIndex.value >= 0 && hasTopResults.value,
);

function rowPinned(rowId: string | undefined): boolean {
  const next = String(rowId || "").trim();
  return next.length > 0 && pinnedTopCandidateIds.value.has(next);
}

function pinCellLabel(rowId: string | undefined): string {
  return rowPinned(rowId) ? "Pinned" : "Pin";
}

function provenanceIcon(label: string | null | undefined) {
  return provenanceIconForText(label);
}

function reviewOverview(): void {
  workflowStore.selectResultsTab("overview");
}

function openCandidateDetail(): void {
  if (!canOpenCandidateDetail.value) return;
  workflowStore.selectWorkflowStep("analyze");
  workflowStore.selectResultsTab("drill");
}

function topColumnToggleId(columnKey: string): string {
  return `topColumnToggle_${String(columnKey || "").replace(/[^a-zA-Z0-9_-]+/g, "_")}`;
}

watch(
  () =>
    [
      String(filterMinPnl.value || ""),
      String(filterMinSharpe.value || ""),
      String(filterMaxDd.value || ""),
      String(filterMinTrades.value || ""),
      String(filterMinExposure.value || ""),
    ] as const,
  ([nextMinPnl, nextMinSharpe, nextMaxDd, nextMinTrades, nextMinExposure]) => {
    if (minPnlDraft.value !== nextMinPnl) minPnlDraft.value = nextMinPnl;
    if (minSharpeDraft.value !== nextMinSharpe)
      minSharpeDraft.value = nextMinSharpe;
    if (maxDdDraft.value !== nextMaxDd) maxDdDraft.value = nextMaxDd;
    if (minTradesDraft.value !== nextMinTrades)
      minTradesDraft.value = nextMinTrades;
    if (minExposureDraft.value !== nextMinExposure)
      minExposureDraft.value = nextMinExposure;
  },
  { immediate: true },
);

function setInputValue(id: string, nextValue: string): void {
  analysisStore.setInputField(id, nextValue);
}

const {
  queueInputValue: queueFilterInputValue,
  flushQueuedInputValuesNow: flushQueuedFilterInputsNow,
} = useQueuedInputWriter(setInputValue, { waitMs: 130, maxWaitMs: 360 });

function queueFilterInput(id: string, nextValue: string): void {
  queueFilterInputValue(id, nextValue);
}

const resizingTopColumnKey = ref("");
let topResizeStartX = 0;
let topResizeStartWidth = 0;

function clickTopHeader(key: string): void {
  const sortKey = String(key || "").trim();
  if (!sortKey) return;
  toggleSortColumn(sortKey);
}

function headerMeta(header: Header<TopRow, unknown>): MatrixColumnMeta {
  return header.column.columnDef.meta as MatrixColumnMeta;
}

function topCellPayload(cell: Cell<TopRow, unknown>): TopCell {
  return cell.getValue() as TopCell;
}

function sortStateToAria(
  sort: TopColumn["sort"],
): "ascending" | "descending" | "none" {
  if (sort === "asc") return "ascending";
  if (sort === "desc") return "descending";
  return "none";
}

function topHeaderStyle(
  header: Header<TopRow, unknown>,
): Record<string, string> {
  return columnWidthStyle(headerMeta(header).columnKey);
}

function topCellStyle(cell: Cell<TopRow, unknown>): Record<string, string> {
  const meta = cell.column.columnDef.meta as MatrixColumnMeta;
  return columnWidthStyle(meta.columnKey);
}

function topCellVisual(cell: Cell<TopRow, unknown>): MetricCellVisual | null {
  const meta = cell.column.columnDef.meta as MatrixColumnMeta;
  return resolveMetricCellVisual(
    meta.columnKey,
    topCellPayload(cell),
    topMetricVisualRanges.value,
  );
}

function finishTopColumnResize(): void {
  resizingTopColumnKey.value = "";
  window.removeEventListener("pointermove", onTopColumnResizeMove);
  window.removeEventListener("pointerup", finishTopColumnResize);
  window.removeEventListener("pointercancel", finishTopColumnResize);
}

function onTopColumnResizeMove(event: PointerEvent): void {
  const columnKey = resizingTopColumnKey.value;
  if (!columnKey) return;
  setColumnWidth(
    columnKey,
    topResizeStartWidth + event.clientX - topResizeStartX,
  );
}

function startTopColumnResize(
  event: PointerEvent,
  header: Header<TopRow, unknown>,
): void {
  const columnKey = headerMeta(header).columnKey;
  if (!columnKey) return;
  event.preventDefault();
  event.stopPropagation();
  resizingTopColumnKey.value = columnKey;
  topResizeStartX = event.clientX;
  topResizeStartWidth =
    columnWidths.value[columnKey] ??
    (event.currentTarget instanceof HTMLElement
      ? event.currentTarget.closest("th")?.getBoundingClientRect().width
      : 128) ??
    128;
  window.addEventListener("pointermove", onTopColumnResizeMove);
  window.addEventListener("pointerup", finishTopColumnResize);
  window.addEventListener("pointercancel", finishTopColumnResize);
}

function onTopColumnResizeKeydown(
  event: KeyboardEvent,
  header: Header<TopRow, unknown>,
): void {
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

function clickTopRow(index: number): void {
  if (!Number.isInteger(index) || index < 0) return;
  workflowStore.selectResultsTab("drill");
  analysisStore.selectTopRow(index);
}

function pinTopRow(index: number): void {
  analysisStore.pinTopRow(index);
}

function focusAdjacentTopRow(
  currentTarget: EventTarget | null,
  direction: "prev" | "next" | "first" | "last",
): void {
  const row = currentTarget instanceof HTMLElement ? currentTarget : null;
  const tbody = row?.parentElement;
  if (!row || !tbody) return;
  const rows = Array.from(
    tbody.querySelectorAll<HTMLElement>('tr[data-top-row="1"]'),
  );
  if (!rows.length) return;
  const currentIndex = rows.indexOf(row);
  if (currentIndex < 0) return;
  let nextIndex = currentIndex;
  if (direction === "prev") nextIndex = Math.max(0, currentIndex - 1);
  else if (direction === "next")
    nextIndex = Math.min(rows.length - 1, currentIndex + 1);
  else if (direction === "first") nextIndex = 0;
  else nextIndex = rows.length - 1;
  rows[nextIndex]?.focus();
}

function onTopRowKeydown(index: number, event: KeyboardEvent): void {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    event.stopPropagation();
    clickTopRow(index);
    return;
  }
  if (event.key.toLowerCase() === "p") {
    event.preventDefault();
    event.stopPropagation();
    pinTopRow(index);
    return;
  }
  if (event.key === "ArrowUp") {
    event.preventDefault();
    event.stopPropagation();
    focusAdjacentTopRow(event.currentTarget, "prev");
    return;
  }
  if (event.key === "ArrowDown") {
    event.preventDefault();
    event.stopPropagation();
    focusAdjacentTopRow(event.currentTarget, "next");
    return;
  }
  if (event.key === "Home") {
    event.preventDefault();
    event.stopPropagation();
    focusAdjacentTopRow(event.currentTarget, "first");
    return;
  }
  if (event.key === "End") {
    event.preventDefault();
    event.stopPropagation();
    focusAdjacentTopRow(event.currentTarget, "last");
  }
}

onMounted(async () => {
  await nextTick();
  scrollToTop();
});

onBeforeUnmount(() => {
  finishTopColumnResize();
  flushQueuedFilterInputsNow();
});
</script>

<template>
  <Teleport v-if="mountReady" to="#topVueShellMount">
    <section class="topPanelVueRoot topVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Ranking</div>
          <h2 class="panelTitle">Top Results</h2>
          <div class="panelSubtitle">
            Filter the ranked shortlist, inspect the best candidates, and send a
            selected row into the deeper analysis views.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">
          {{ topRows }} rows · {{ stamp }}
        </div>
      </header>

      <div class="panelSummaryChips" aria-label="Top results summary">
        <div
          v-for="chip in topSummaryChips"
          :key="chip"
          class="panelSummaryChip"
        >
          {{ chip }}
        </div>
      </div>

      <section v-if="topStage === 'ready'" class="pane panelProminentPane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Filters</div>
            <h3 class="panelPaneTitle">Shortlist gates</h3>
            <div class="panelSectionLead">
              Tighten the ranked list by PnL, Sharpe, drawdown, trades, and
              exposure before choosing a candidate.
            </div>
          </div>
        </div>
        <div class="gridFive">
          <div>
            <label for="vueFilterMinPnl">Min PnL</label>
            <input
              id="vueFilterMinPnl"
              type="number"
              step="any"
              :value="minPnlDraft"
              @input="
                minPnlDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueFilterInput('filterMinPnl', minPnlDraft);
              "
            />
          </div>
          <div>
            <label for="vueFilterMinSharpe">Min Sharpe</label>
            <input
              id="vueFilterMinSharpe"
              type="number"
              step="any"
              :value="minSharpeDraft"
              @input="
                minSharpeDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueFilterInput('filterMinSharpe', minSharpeDraft);
              "
            />
          </div>
          <div>
            <label for="vueFilterMaxDd">Max MaxDD</label>
            <input
              id="vueFilterMaxDd"
              type="number"
              step="any"
              :value="maxDdDraft"
              @input="
                maxDdDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueFilterInput('filterMaxDd', maxDdDraft);
              "
            />
          </div>
          <div>
            <label for="vueFilterMinTrades">Min trades</label>
            <input
              id="vueFilterMinTrades"
              type="number"
              min="0"
              step="1"
              :value="minTradesDraft"
              @input="
                minTradesDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueFilterInput('filterMinTrades', minTradesDraft);
              "
            />
          </div>
          <div>
            <label for="vueFilterMinExposure">Min exposure</label>
            <input
              id="vueFilterMinExposure"
              type="number"
              step="any"
              :value="minExposureDraft"
              @input="
                minExposureDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueFilterInput('filterMinExposure', minExposureDraft);
              "
            />
          </div>
        </div>
        <div class="actions">
          <button
            type="button"
            data-qa-action="top.filters.clear"
            :disabled="!canClearTopFilters"
            @click="analysisStore.clearTopFilters()"
          >
            Clear filters
          </button>
        </div>
        <div class="panelAdvisory mono">{{ topFilterMeta }}</div>
        <div
          v-if="hasTopSelectionGuardrail"
          class="panelAdvisory surfaceWarning mono"
        >
          {{ topSelectionGuardrail }}
        </div>
      </section>

      <section class="pane topReviewPane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Table</div>
            <h3 class="panelPaneTitle">Ranked candidates</h3>
            <div class="panelSectionLead">
              Sort the shortlist by the current metric, then choose a row to
              update the rest of the analysis workspace.
            </div>
          </div>
        </div>
        <div
          v-if="topStage === 'ready'"
          id="topSelectionStatus"
          class="stickySelectionSummary selectionRail topSelectionSummary"
          role="status"
          aria-live="polite"
          aria-atomic="true"
        >
          <div class="stickySelectionSummaryTitleRow">
            <AppVisualIcon
              :icon="compareCandidateFocusIcon"
              class="sectionGlyph"
            />
            <div class="stickySelectionSummaryTitle">Current candidate</div>
          </div>
          <div class="compareCardTitle">{{ displayedSelectedTopTitle }}</div>
          <div class="stickySelectionSummaryBody">
            {{ displayedSelectedTopSummary }}
          </div>
          <div
            class="panelSummaryChips"
            aria-label="Current candidate provenance"
          >
            <span
              v-for="chip in displayedSelectedTopProvenanceChips"
              :key="`top_provenance_${chip}`"
              class="panelSummaryChip mono withIcon"
            >
              <AppVisualIcon
                v-if="provenanceIcon(chip)"
                :icon="provenanceIcon(chip)"
                class="chipIcon"
              />
              {{ chip }}
            </span>
          </div>
          <AppMetricStrip
            class="currentCandidateStats"
            :items="topDecisionStats"
            aria-label="Current candidate table state"
            min-item-width="5.6rem"
            size="compact"
            surface="sunken"
          />
          <div class="panelMuted">{{ displayedSelectedTopDecisionNote }}</div>
          <div class="panelMuted topSelectionPathNote">
            Use Candidate Detail for deeper proof and Overview for baseline
            framing. Pareto and Heatmap stay optional lenses once the shortlist
            decision is made.
          </div>
          <div class="actions stickySelectionActions">
            <button
              type="button"
              class="primary"
              data-qa-action="top.selected.openCandidateDetail"
              :disabled="!canOpenCandidateDetail"
              @click="openCandidateDetail()"
            >
              <span>Open Candidate Detail</span>
            </button>
            <button
              type="button"
              data-qa-action="top.selected.reviewOverview"
              :disabled="!hasTopResults"
              @click="reviewOverview()"
            >
              <span>Review Overview</span>
            </button>
            <button
              type="button"
              data-qa-action="top.selected.pin"
              :disabled="!canPinSelectedTopCandidate"
              @click="analysisStore.pinSelectedTopRow()"
            >
              <AppVisualIcon :icon="pinCompareIcon" class="actionIcon" />
              <span>Pin current candidate</span>
            </button>
            <button
              type="button"
              data-qa-action="top.pins.clear"
              :disabled="pinnedTopCandidates.length === 0"
              @click="analysisStore.clearPinnedTopCandidates()"
            >
              <AppVisualIcon :icon="clearCompareIcon" class="actionIcon" />
              <span>Clear compare</span>
            </button>
          </div>
        </div>
        <div v-if="displayedTopStaleCues.length > 0" class="staleCueStack">
          <div
            v-for="cue in displayedTopStaleCues"
            :key="cue"
            class="panelAdvisory surfaceWarning mono staleCueRow"
          >
            <AppVisualIcon
              v-if="provenanceIcon(cue)"
              :icon="provenanceIcon(cue)"
              class="chipIcon"
            />
            <span>{{ cue }}</span>
          </div>
        </div>
        <div
          v-if="topStage !== 'ready' || !hasTopResults"
          class="emptyState topEmptyState"
        >
          <img
            class="emptyStateIllustration"
            :src="emptyResultsWorkspaceIllustration"
            alt=""
            aria-hidden="true"
            draggable="false"
          />
          <div class="emptyTitle">{{ emptyTitle }}</div>
          <div class="emptyBody">{{ emptyBody }}</div>
          <div
            v-if="topStage === 'ready' && (filtersActive || canClearTopFilters)"
            class="emptyActions"
          >
            <button
              type="button"
              :disabled="!canClearTopFilters"
              @click="analysisStore.clearTopFilters()"
            >
              Clear filters
            </button>
          </div>
          <div class="panelMuted mono panelNumeric">
            {{ topFilterMeta || statusLine }}
          </div>
          <div
            v-if="topStage === 'ready' && hasTopSelectionGuardrail"
            class="panelMuted mono panelNumeric"
          >
            {{ topSelectionGuardrail }}
          </div>
        </div>

        <AppDataTableShell
          v-else
          title="Ranked candidate rows"
          :summary="topTableShellSummary"
          :row-count="renderedRows.length"
          :viewport="false"
          :density="tableDensity"
          toolbar-label="Ranked candidate table controls"
          aria-label="Top optimization results table shell"
          keyboard-hint="Use arrow keys to move between rows, Enter to open Candidate Detail, and P to pin a candidate."
        >
          <template #controls>
            <div class="tableControlBar">
              <AppTableViewPresetControl
                :presets="TOP_VIEW_PRESETS"
                :active-preset-id="activeTopViewPresetId"
                label="Top view"
                qa-prefix="top.table.view"
                @apply="applyTopViewPreset"
              />
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
                    aria-label="Visible top result columns"
                  >
                    <label
                      v-for="column in columnOptions"
                      :key="column.key"
                      class="columnToggle"
                      :class="{
                        disabled: !canHideColumn(column.key) && column.visible,
                      }"
                      :for="topColumnToggleId(column.key)"
                    >
                      <input
                        :id="topColumnToggleId(column.key)"
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
              <AppTableDensityControl
                v-model:density="tableDensity"
                label="Top density"
                qa-prefix="top.table.density"
              />
            </div>
          </template>
          <div
            ref="tableScrollElementRef"
            class="tableWrap virtualTableViewport overflowShadowY overflowShadowX surfaceWorking"
            tabindex="0"
            aria-label="Top optimization results table"
            aria-describedby="topSelectionStatus"
          >
            <table
              class="topTableVue"
              role="grid"
              aria-describedby="topSelectionStatus"
            >
              <caption class="srOnly">
                Top optimization results ranked by the selected table sort
                order.
              </caption>
              <thead role="rowgroup">
                <tr
                  v-for="headerGroup in headerGroups"
                  :key="headerGroup.id"
                  role="row"
                >
                  <th
                    v-for="header in headerGroup.headers"
                    :key="header.id"
                    :class="{ left: headerMeta(header).alignLeft }"
                    :aria-sort="sortStateToAria(headerMeta(header).sort)"
                    :style="topHeaderStyle(header)"
                    scope="col"
                    role="columnheader"
                  >
                    <button
                      v-if="!header.isPlaceholder"
                      type="button"
                      class="headerButton"
                      :class="{ left: headerMeta(header).alignLeft }"
                      @click="clickTopHeader(headerMeta(header).columnKey)"
                    >
                      <span>{{ headerMeta(header).columnLabel }}</span>
                      <span
                        v-if="headerMeta(header).sort === 'asc'"
                        class="sortGlyph"
                        aria-hidden="true"
                        >↑</span
                      >
                      <span
                        v-else-if="headerMeta(header).sort === 'desc'"
                        class="sortGlyph"
                        aria-hidden="true"
                        >↓</span
                      >
                    </button>
                    <button
                      v-if="!header.isPlaceholder"
                      type="button"
                      class="columnResizeHandle"
                      :class="{
                        active:
                          resizingTopColumnKey === headerMeta(header).columnKey,
                      }"
                      :aria-label="`Resize ${headerMeta(header).columnLabel} column`"
                      title="Drag to resize. Use Left/Right arrows to adjust, Delete to reset."
                      @click.stop
                      @pointerdown="startTopColumnResize($event, header)"
                      @keydown.stop="onTopColumnResizeKeydown($event, header)"
                    ></button>
                  </th>
                </tr>
              </thead>
              <tbody role="rowgroup">
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
                <tr
                  v-for="virtualRow in virtualRows"
                  :key="virtualRow.row.id"
                  :class="{ selected: virtualRow.row.original.selected }"
                  data-top-row="1"
                  role="row"
                  :aria-selected="virtualRow.row.original.selected"
                  tabindex="0"
                  @click="clickTopRow(virtualRow.row.index)"
                  @keydown="onTopRowKeydown(virtualRow.row.index, $event)"
                >
                  <template
                    v-for="(
                      cell, visibleCellIndex
                    ) in virtualRow.row.getVisibleCells()"
                    :key="cell.id"
                  >
                    <td
                      :class="[
                        topCellPayload(cell).className,
                        {
                          leadVisibleColumn:
                            cell.column.id === leadVisibleColumnKey,
                        },
                      ]"
                      :style="topCellStyle(cell)"
                      :data-column-key="
                        String(
                          (cell.column.columnDef.meta as MatrixColumnMeta)
                            ?.columnKey || '',
                        )
                      "
                      :title="topCellPayload(cell).title || undefined"
                      role="gridcell"
                    >
                      <AppMetricCell
                        :text="topCellPayload(cell).text"
                        :visual="topCellVisual(cell)"
                        :align="
                          (cell.column.columnDef.meta as MatrixColumnMeta)
                            .alignLeft
                            ? 'left'
                            : 'right'
                        "
                      >
                        <button
                          v-if="visibleCellIndex === 0"
                          type="button"
                          class="pinCellBtn"
                          :aria-label="`${pinCellLabel(virtualRow.row.original.rowId)} candidate ${virtualRow.row.index + 1}`"
                          :disabled="rowPinned(virtualRow.row.original.rowId)"
                          @click.stop="pinTopRow(virtualRow.row.index)"
                        >
                          {{ pinCellLabel(virtualRow.row.original.rowId) }}
                        </button>
                      </AppMetricCell>
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
        <section
          v-if="pinnedTopCandidates.length > 0"
          class="compareTray surfaceSunken"
        >
          <div class="panelSectionHeader">
            <div class="panelHeadCopy">
              <div class="panelEyebrow">Compare</div>
              <h4 class="panelPaneTitle compareHeading">
                <AppVisualIcon
                  :icon="compareCandidatesIcon"
                  class="sectionGlyph"
                />
                <span>Pinned candidates</span>
              </h4>
              <div class="panelSectionLead">
                Keep comparison available without letting it outweigh shortlist
                scanning and candidate choice.
              </div>
            </div>
          </div>
          <div class="compareTrayGrid">
            <article
              v-for="candidate in pinnedTopCandidates"
              :key="candidate.id"
              class="compareCard surfaceFocus"
            >
              <div class="compareCardMeta mono">#{{ candidate.rank }}</div>
              <div class="compareCardTitle">{{ candidate.title }}</div>
              <div class="compareCardSummary">{{ candidate.summary }}</div>
              <div
                v-if="candidate.badges.length > 0"
                class="panelSummaryChips"
                aria-label="Pinned candidate context"
              >
                <span
                  v-for="badge in candidate.badges"
                  :key="`${candidate.id}_${badge}`"
                  class="panelSummaryChip mono withIcon"
                >
                  <AppVisualIcon
                    v-if="provenanceIcon(badge)"
                    :icon="provenanceIcon(badge)"
                    class="chipIcon"
                  />
                  {{ badge }}
                </span>
              </div>
              <div v-if="candidate.detail" class="panelMuted">
                {{ candidate.detail }}
              </div>
              <div
                v-if="candidate.staleReason"
                class="panelAdvisory surfaceWarning mono"
              >
                {{ candidate.staleReason }}
              </div>
              <div class="actions">
                <button
                  type="button"
                  :disabled="!canReplaceWithCurrent"
                  @click="analysisStore.replacePinnedTopCandidate(candidate.id)"
                >
                  <AppVisualIcon
                    :icon="replaceCompareIcon"
                    class="actionIcon"
                  />
                  <span>Replace with current</span>
                </button>
                <button
                  type="button"
                  @click="analysisStore.unpinTopCandidate(candidate.id)"
                >
                  <AppVisualIcon :icon="removeCompareIcon" class="actionIcon" />
                  <span>Remove</span>
                </button>
              </div>
            </article>
          </div>
        </section>
      </section>

      <div
        class="panelStatus mono panelNumeric"
        role="status"
        aria-live="polite"
      >
        {{ statusLine }}
      </div>
    </section>
  </Teleport>
</template>

<style scoped>
.topPanelVueRoot {
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
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(
    in oklab,
    var(--surface-working) 95%,
    var(--panel-footer)
  );
  padding: var(--density-pane-y) var(--density-pane-x);
  min-width: 0;
}

.topReviewPane {
  display: grid;
  gap: 0.65rem;
}

.gridFive {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.6rem;
}

.actions {
  margin-top: 0.65rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: calc(var(--density-control-y) - 0.13rem)
    calc(var(--density-control-x) - 0.15rem);
  font-size: 0.76rem;
  display: inline-flex;
  align-items: center;
  gap: 0.38rem;
}

.actionIcon {
  width: 0.9rem;
  height: 0.9rem;
  flex: 0 0 auto;
}

.tableWrap {
  overflow: auto;
  border: 1px solid color-mix(in oklab, var(--border) 90%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-working) 96%, var(--panel));
}

.virtualTableViewport {
  max-height: clamp(18rem, 52vh, 34rem);
}

.columnDetails {
  margin-bottom: 0;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-sunken) 94%, var(--panel));
}

.tableControlBar {
  width: 100%;
  min-width: 0;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.65rem;
}

.columnDetails[open] {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 38%, var(--border));
  background: color-mix(
    in oklab,
    var(--surface-working) 94%,
    var(--panel-footer)
  );
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
  color: var(--muted-strong);
}

.columnDetailsSummary::-webkit-details-marker {
  display: none;
}

.columnDetailsBody {
  border-top: 1px solid color-mix(in oklab, var(--border) 78%, transparent);
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

.topEmptyState {
  min-height: 260px;
  display: grid;
  align-content: center;
  gap: 0.65rem;
}

.emptyStateIllustration {
  width: clamp(88px, 18vw, 124px);
  height: auto;
  justify-self: start;
  opacity: 0.92;
}

.topTableVue {
  width: 100%;
  border-collapse: collapse;
  min-width: 760px;
}

.topTableVue th,
.topTableVue td {
  border-bottom: 1px solid var(--border);
  padding: var(--density-table-y) var(--density-table-x);
  text-align: right;
  vertical-align: top;
}

.topTableVue th {
  position: relative;
  user-select: none;
  font-size: 0.72rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--muted-soft);
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

.topTableVue th.left,
.topTableVue td.left {
  text-align: left;
}

.topTableVue td.leadVisibleColumn {
  background: linear-gradient(
    90deg,
    color-mix(in oklab, var(--surface-overlay) 82%, transparent),
    transparent 92%
  );
}

.topTableVue tbody tr {
  cursor: pointer;
}

.topTableVue tbody tr:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: -2px;
}

.topTableVue tbody tr.selected {
  background: color-mix(
    in oklab,
    var(--selection-fill-strong) 84%,
    var(--surface-working)
  );
}

.topTableVue tbody tr.selected td:first-child {
  box-shadow: inset 3px 0 0 var(--selection-rail);
}

.topSelectionSummary {
  margin-bottom: 0;
  top: -0.1rem;
}

.topTableVue tbody tr.spacerRow {
  pointer-events: none;
}

.topTableVue tbody tr.spacerRow td {
  border-bottom: none;
  padding: 0;
}

.pinCellBtn {
  padding: calc(var(--density-chip-y) + 0.06rem)
    calc(var(--density-chip-x) - 0.02rem);
  border-radius: 999px;
  font-size: 0.7rem;
  line-height: 1;
  white-space: nowrap;
}

.stickySelectionActions {
  margin-top: 0;
}

.topSelectionPathNote {
  margin-top: 0.15rem;
}

.stickySelectionSummaryTitleRow,
.compareHeading {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.staleCueStack {
  display: grid;
  gap: 0.5rem;
  margin-bottom: 0;
}

.staleCueRow {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.compareTray {
  margin-bottom: 0;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-sunken) 94%, var(--panel));
  padding: var(--density-pane-y) var(--density-pane-x);
}

.compareTrayGrid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.6rem;
}

.compareCard {
  border: 1px solid
    color-mix(in oklab, var(--panel-focus-edge) 72%, var(--border));
  border-radius: 12px;
  padding: calc(var(--density-pane-y) - 0.08rem)
    calc(var(--density-pane-x) - 0.05rem);
  min-width: 0;
  display: grid;
  gap: 0.35rem;
  box-shadow: none;
}

.compareCardMeta {
  font-size: 0.72rem;
  color: var(--muted);
}

.compareCardTitle {
  font-size: 0.84rem;
  font-weight: 650;
  line-height: var(--leading-tight);
}

.compareCardSummary {
  font-size: 0.78rem;
  line-height: var(--leading-compact);
  color: var(--muted-strong);
}

.panelSummaryChip.withIcon {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}

.chipIcon {
  width: 0.95rem;
  height: 0.95rem;
  flex: 0 0 auto;
}

.sectionGlyph {
  width: 1rem;
  height: 1rem;
  flex: 0 0 auto;
}

.sortGlyph {
  margin-left: 0.25rem;
  font-size: 0.72rem;
  opacity: 0.8;
}

.headerButton {
  width: 100%;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  display: inline-flex;
  justify-content: flex-end;
  align-items: center;
  gap: 0.25rem;
  font: inherit;
}

.headerButton.left {
  justify-content: flex-start;
}

.headerButton:hover:not(:disabled) {
  background: transparent;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1100px) {
  .gridFive {
    grid-template-columns: 1fr 1fr;
  }

  .columnToggleGrid {
    grid-template-columns: 1fr 1fr;
  }

  .compareTrayGrid {
    grid-template-columns: 1fr 1fr;
  }
}

@media (max-width: 680px) {
  .tableControlBar {
    display: grid;
    justify-content: stretch;
  }

  .gridFive {
    grid-template-columns: 1fr;
  }

  .columnToggleGrid {
    grid-template-columns: 1fr;
  }

  .compareTrayGrid {
    grid-template-columns: 1fr;
  }
}
</style>
