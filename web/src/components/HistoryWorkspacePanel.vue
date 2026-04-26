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
import { useDebounceFn } from "@vueuse/core";
import { useShellMount } from "../composables/useShellMount";
import { usePersistedTableDensity } from "../composables/usePersistedTableDensity";
import { useUiReadiness } from "../composables/useUiReadiness";
import { useVirtualTableRows } from "../composables/useVirtualTableRows";
import { useHistoryStore } from "../stores/historyStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { emptyArtifactLibraryIllustration } from "../utils/appSvgAssets";
import {
  formatRouteProvenanceCompact,
  type RouteProvenanceSummary,
} from "../utils/core/runArtifactReview";
import { HISTORY_SAVED_RUNS_TABLE_DENSITY_KEY } from "../utils/core/storageKeys";
import { formatUiBinaryBytes } from "../utils/uiFormat";
import { formatUiTimeFromUnixMs } from "../utils/uiTime";
import ConfirmationDialog from "./ConfirmationDialog.vue";
import OptionsExpressionPanel from "./OptionsExpressionPanel.vue";
import AppDataTableShell from "./ui/AppDataTableShell.vue";
import AppTableDensityControl from "./ui/AppTableDensityControl.vue";

const historyStore = useHistoryStore();
const workflowStore = useWorkflowStore();
const { historyStage } = useUiReadiness();
const {
  sessionRunsRaw,
  sessionSelectedValue,
  savedRunsVisible,
  savedSelectedValue,
  savedRunsSearch,
  savedRunsTagsDraft,
  savedRunCount,
  savedRunsMeta,
  savedRunCompareOptions,
  savedRunCompareSelected,
  savedRunCompareSummary,
  savedRunDetailTitle,
  savedRunDetailMeta,
  savedRunDetailStrategy,
  savedRunDetailConfigFingerprint,
  savedRunDetailDataset,
  savedRunDetailTimeframe,
  savedRunDetailValidation,
  savedRunDetailProvenance,
  savedRunDetailLineage,
  savedRunDetailMetricSurface,
  savedRunDetailMetricSurfaceDetail,
  savedRunDetailAdmission,
  savedRunDetailAdmissionDetail,
  savedRunDetailReport,
  savedRunDetailSize,
  savedRunDetailTags,
  savedRunDetailMlBundle,
  savedRunDetailMlScoreCache,
  savedRunDetailMlFeatureSchema,
  savedRunDetailMlScoreContract,
  savedRunDetailMlScoreNote,
  savedRunDetailMlPolicy,
  savedRunDetailMlSweep,
  savedRunDetailMlVerdict,
  savedRunDetailMlCandidates,
  savedRunDetailParams,
  canApplyHistory,
  canExportPreset,
  canImportPreset,
  canClearHistory,
  canSaveSavedRunTags,
  canRefreshSavedRuns,
  canLoadSavedRun,
  canApplySavedRun,
  canRerunSavedRun,
  canOpenSavedRunReport,
  canRevealSavedRun,
  canDeleteSavedRun,
  canOpenRunsDir,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(historyStore);
const { workspace, workflow } = storeToRefs(workflowStore);

interface SessionRunListEntry {
  ts: string;
  backend: string;
  pairs: number;
}

interface SavedRunListEntry {
  ts: string;
  path: string;
  bytes: number;
  strategyKind: string;
  hasValidation: boolean;
  reportXlsxPath: string;
  reportXlsxExists?: boolean | null;
  csvPath: string;
  mlScoreContractSummary?: string;
  mlScoreNote?: string;
  routeProvenance?: RouteProvenanceSummary | null;
  tags: string[];
}

const { mountReady } = useShellMount("#historyVueShellMount", {
  paneSelector: '[data-vue-panel="history"]',
  paneDatasetKey: "vueHistory",
});
const sessionValue = ref("");
const savedValue = ref("");
const searchDraft = ref("");
const tagsDraft = ref("");
const compareDraft = ref("");
const queuedSearchValue = ref<string | null>(null);
const queuedTagsValue = ref<string | null>(null);
const pendingAction = ref<"clearSession" | "deleteSavedRun" | null>(null);

const commitSavedRunsSearch = useDebounceFn(
  (value: string) => {
    queuedSearchValue.value = null;
    historyStore.setSavedRunsSearch(value);
  },
  110,
  { maxWait: 420 },
);

const commitSavedRunsTags = useDebounceFn(
  (value: string) => {
    queuedTagsValue.value = null;
    historyStore.setSavedRunsTagsDraft(value);
  },
  110,
  { maxWait: 420 },
);

const visible = computed(() => {
  const ws = String(workspace.value || "").trim();
  const wf = String(workflow.value || "").trim();
  return ws === "library" || wf === "history";
});

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value));

const visibleSessionRuns = computed<SessionRunListEntry[]>(() =>
  (sessionRunsRaw.value as SessionRunListEntry[]).filter(
    (entry) => String(entry.ts || "").trim().length > 0,
  ),
);

const visibleSavedRuns = computed<SavedRunListEntry[]>(
  () => savedRunsVisible.value as SavedRunListEntry[],
);
const {
  scrollElementRef: savedRunsScrollElementRef,
  virtualRows: virtualSavedRunRows,
  topSpacerPx: savedRunsTopSpacerPx,
  bottomSpacerPx: savedRunsBottomSpacerPx,
  scrollToTop: scrollSavedRunsToTop,
} = useVirtualTableRows(visibleSavedRuns, {
  rowHeightPx: 84,
  overscan: 8,
  getItemKey: (_index, row) => String(row.path || _index).trim() || _index,
});
const { tableDensity: savedRunsTableDensity } = usePersistedTableDensity(
  HISTORY_SAVED_RUNS_TABLE_DENSITY_KEY,
);
const historySummaryChips = computed(() => {
  const chips = [
    `${visibleSessionRuns.value.length} session runs`,
    `${savedRunCount.value} saved runs`,
  ];
  if (String(savedRunDetailTitle.value || "").trim().length > 0) {
    chips.push(`Selected ${String(savedRunDetailTitle.value || "").trim()}`);
  }
  return chips;
});
const savedRunsSearchSummary = computed(() => {
  const query = String(searchDraft.value || "").trim();
  const visibleCount = visibleSavedRuns.value.length;
  const total = savedRunCount.value;
  const localHelp =
    "Search filters the saved-run library only. Use Ctrl+K for global commands.";
  if (!query) return `${total} saved runs available. ${localHelp}`;
  const label = visibleCount === 1 ? "match" : "matches";
  return `${visibleCount} ${label} for “${query}”. ${localHelp}`;
});
const sessionRunListSummary = computed(() => {
  const count = visibleSessionRuns.value.length;
  return `${count} run${count === 1 ? "" : "s"} captured in this session`;
});

function basename(value: string): string {
  const next = String(value || "").trim();
  if (!next) return "";
  const parts = next.split(/[\\/]+/g);
  return String(parts[parts.length - 1] || "").trim();
}

function formatBytesCompact(bytes: number): string {
  return formatUiBinaryBytes(bytes);
}

function sessionRunMeta(entry: SessionRunListEntry): string {
  const backend = String(entry.backend || "").trim() || "unknown backend";
  const pairs = Math.max(0, Math.floor(Number(entry.pairs || 0)));
  return `${backend} · ${pairs} pairs`;
}

function savedRunRouteSummary(entry: SavedRunListEntry): string {
  const summary = formatRouteProvenanceCompact(entry.routeProvenance ?? null);
  return summary === "-" ? "" : summary;
}

function savedRunMlScoreContract(entry: SavedRunListEntry): string {
  return String(entry.mlScoreContractSummary || "").trim();
}

function savedRunMlScoreNote(entry: SavedRunListEntry): string {
  return String(entry.mlScoreNote || "").trim();
}

function hasSavedRunReport(entry: SavedRunListEntry): boolean {
  const reportPath = String(entry.reportXlsxPath || "").trim();
  if (!reportPath) return false;
  return entry.reportXlsxExists == null ? true : !!entry.reportXlsxExists;
}

watch(
  () =>
    [
      String(sessionSelectedValue.value || "").trim(),
      String(savedSelectedValue.value || "").trim(),
      String(savedRunCompareSelected.value || "").trim(),
      String(savedRunsSearch.value || ""),
      String(savedRunsTagsDraft.value || ""),
    ] as const,
  ([
    nextSessionValue,
    nextSavedValue,
    nextCompareValue,
    nextSavedRunsSearch,
    nextSavedRunsTagsDraft,
  ]) => {
    if (sessionValue.value !== nextSessionValue)
      sessionValue.value = nextSessionValue;
    if (savedValue.value !== nextSavedValue) savedValue.value = nextSavedValue;
    if (compareDraft.value !== nextCompareValue)
      compareDraft.value = nextCompareValue;
    if (searchDraft.value !== nextSavedRunsSearch)
      searchDraft.value = nextSavedRunsSearch;
    if (tagsDraft.value !== nextSavedRunsTagsDraft)
      tagsDraft.value = nextSavedRunsTagsDraft;
  },
  { immediate: true },
);

function setSelectValue(id: string, nextValue: string): void {
  historyStore.setSelectField(id, nextValue);
}

function saveSavedRunTags(): void {
  historyStore.saveSavedRunTags(
    String(savedValue.value || "").trim(),
    String(tagsDraft.value || ""),
  );
}

function loadSavedRun(): void {
  historyStore.loadSavedRun(String(savedValue.value || "").trim());
}

function applySavedRun(): void {
  historyStore.applySavedRun(String(savedValue.value || "").trim());
}

function rerunSavedRun(): void {
  historyStore.rerunSavedRun(String(savedValue.value || "").trim());
}

function openSavedRunReport(): void {
  historyStore.openSavedRunReport(String(savedValue.value || "").trim());
}

function revealSavedRun(): void {
  historyStore.revealSavedRun(String(savedValue.value || "").trim());
}

const pendingDeleteTarget = computed(() =>
  String(savedValue.value || "").trim(),
);

const confirmationTitle = computed(() => {
  if (pendingAction.value === "clearSession") return "Clear session history";
  if (pendingAction.value === "deleteSavedRun") return "Delete saved run";
  return "";
});

const confirmationBody = computed(() => {
  if (pendingAction.value === "clearSession") {
    return "This removes the in-memory run list for the current session. Saved run artifacts stay on disk.";
  }
  if (pendingAction.value === "deleteSavedRun") {
    return pendingDeleteTarget.value
      ? `Delete this saved run artifact from the library?\n\n${pendingDeleteTarget.value}\n\nThis cannot be undone from the run library.`
      : "Select a saved run before deleting it from the library.";
  }
  return "";
});

const confirmationConfirmLabel = computed(() => {
  if (pendingAction.value === "clearSession") return "Confirm clear";
  if (pendingAction.value === "deleteSavedRun") return "Delete artifact";
  return "Confirm";
});

const canConfirmPendingAction = computed(() => {
  if (pendingAction.value === "clearSession")
    return Boolean(canClearHistory.value);
  if (pendingAction.value === "deleteSavedRun") {
    return Boolean(canDeleteSavedRun.value && pendingDeleteTarget.value);
  }
  return false;
});

function queueClearSessionConfirmation(): void {
  pendingAction.value = "clearSession";
}

function queueDeleteSavedRunConfirmation(): void {
  if (!pendingDeleteTarget.value) return;
  pendingAction.value = "deleteSavedRun";
}

function cancelPendingAction(): void {
  pendingAction.value = null;
}

function confirmPendingAction(): void {
  const action = pendingAction.value;
  pendingAction.value = null;
  if (action === "clearSession") {
    historyStore.clearSession();
    return;
  }
  if (action === "deleteSavedRun" && pendingDeleteTarget.value) {
    historyStore.deleteSavedRun(pendingDeleteTarget.value);
  }
}

function onSessionSelect(value: string): void {
  sessionValue.value = value;
  setSelectValue("historySelect", value);
}

function onSavedSelect(value: string): void {
  savedValue.value = value;
  historyStore.selectSavedRun(value);
}

function onSavedRunCompareSelect(value: string): void {
  compareDraft.value = value;
  historyStore.setSelectField("savedRunCompareSelect", value);
}

function onSavedSearch(value: string): void {
  searchDraft.value = value;
  queuedSearchValue.value = value;
  commitSavedRunsSearch(value);
}

function onSavedTags(value: string): void {
  tagsDraft.value = value;
  queuedTagsValue.value = value;
  commitSavedRunsTags(value);
}

function moveRunListFocus(
  currentTarget: EventTarget | null,
  direction: "prev" | "next" | "first" | "last",
): void {
  const button = currentTarget instanceof HTMLElement ? currentTarget : null;
  const list = button?.closest("ul.runList");
  if (!button || !list) return;
  const buttons = Array.from(
    list.querySelectorAll<HTMLButtonElement>(
      "button.runListButton:not(:disabled)",
    ),
  );
  if (!buttons.length) return;
  const currentIndex = buttons.indexOf(button as HTMLButtonElement);
  if (currentIndex < 0) return;
  let nextIndex = currentIndex;
  if (direction === "prev") nextIndex = Math.max(0, currentIndex - 1);
  else if (direction === "next")
    nextIndex = Math.min(buttons.length - 1, currentIndex + 1);
  else if (direction === "first") nextIndex = 0;
  else nextIndex = buttons.length - 1;
  buttons[nextIndex]?.focus();
}

function onRunListKeydown(event: KeyboardEvent): void {
  if (event.key === "ArrowUp") {
    event.preventDefault();
    moveRunListFocus(event.currentTarget, "prev");
    return;
  }
  if (event.key === "ArrowDown") {
    event.preventDefault();
    moveRunListFocus(event.currentTarget, "next");
    return;
  }
  if (event.key === "Home") {
    event.preventDefault();
    moveRunListFocus(event.currentTarget, "first");
    return;
  }
  if (event.key === "End") {
    event.preventDefault();
    moveRunListFocus(event.currentTarget, "last");
  }
}

onMounted(async () => {
  await nextTick();
  historyStore.refreshSessionHistory();
  scrollSavedRunsToTop();
});

onBeforeUnmount(() => {
  if (queuedSearchValue.value != null) {
    historyStore.setSavedRunsSearch(queuedSearchValue.value);
    queuedSearchValue.value = null;
  }
  if (queuedTagsValue.value != null) {
    historyStore.setSavedRunsTagsDraft(queuedTagsValue.value);
    queuedTagsValue.value = null;
  }
  pendingAction.value = null;
});
</script>

<template>
  <Teleport v-if="mountReady" to="#historyVueShellMount">
    <section
      class="historyPanelVueRoot historyVueShellMountHost"
      v-show="visible"
    >
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Library</div>
          <h2 class="panelTitle">Run History</h2>
          <div class="panelSubtitle">
            Session artifacts are transient. Save, reload, rerun, and inspect
            the runs worth keeping from the current workflow.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">
          {{ savedRunCount }} saved · {{ stamp }}
        </div>
      </header>
      <div class="panelSummaryChips" aria-label="Run history summary">
        <span
          v-for="chip in historySummaryChips"
          :key="`history_summary_${chip}`"
          class="panelSummaryChip mono"
        >
          {{ chip }}
        </span>
      </div>

      <section v-if="historyStage === 'no_data'" class="emptyState">
        <img
          class="emptyStateIllustration"
          :src="emptyArtifactLibraryIllustration"
          alt=""
          aria-hidden="true"
          draggable="false"
        />
        <div class="emptyTitle">Library is empty.</div>
        <div class="emptyBody">
          Save or rerun an optimization to build a library of artifacts you can
          inspect here.
        </div>
        <div
          class="panelStatus mono panelNumeric"
          role="status"
          aria-live="polite"
        >
          {{ statusLine }}
        </div>
        <div class="actions emptyStateActions">
          <button
            type="button"
            data-qa-action="library.savedRun.refresh"
            :disabled="!canRefreshSavedRuns"
            @click="historyStore.refreshSavedRuns()"
          >
            Refresh saved
          </button>
          <button
            type="button"
            data-qa-action="library.runsDir.open"
            :disabled="!canOpenRunsDir"
            @click="historyStore.openRunsDir()"
          >
            Runs folder
          </button>
        </div>
      </section>

      <section
        v-else-if="historyStage === 'results_available_no_selection'"
        class="emptyState"
      >
        <div class="emptyTitle">Select a saved run to inspect.</div>
        <div class="emptyBody">
          Your run library has artifacts available. Pick one from Saved Runs to
          inspect metadata, compare outputs, or rerun it.
        </div>
        <div class="panelMuted mono panelNumeric">{{ savedRunsMeta }}</div>
      </section>

      <div v-if="historyStage !== 'no_data'" class="grid">
        <section class="pane historyPaneSession">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Session recall</div>
              <h3 class="panelPaneTitle">Session History</h3>
              <div class="panelSectionLead">
                These runs live only for the active session unless you export or
                save them elsewhere.
              </div>
            </div>
          </div>
          <AppDataTableShell
            title="Session run artifacts"
            :summary="sessionRunListSummary"
            :row-count="visibleSessionRuns.length"
            :viewport="false"
            aria-label="Session history runs shell"
            empty-title="No session runs have been captured yet."
            empty-body="Run or apply an optimization to populate this session recall list."
            keyboard-hint="Use arrow keys, Home, and End to move through session run artifacts."
          >
            <ul
              class="runList overflowShadowY surfaceWorking"
              aria-label="Session history runs"
            >
              <li
                v-for="entry in visibleSessionRuns"
                :key="`session_run_${entry.ts}`"
                class="runListItem"
              >
                <button
                  type="button"
                  class="runListButton"
                  :class="{ active: sessionValue === entry.ts }"
                  :aria-pressed="sessionValue === entry.ts"
                  @click="onSessionSelect(entry.ts)"
                  @keydown="onRunListKeydown"
                >
                  <div class="runListPrimary">
                    <span class="mono">{{ entry.ts }}</span>
                  </div>
                  <div class="runListSecondary panelMuted">
                    {{ sessionRunMeta(entry) }}
                  </div>
                </button>
              </li>
            </ul>
          </AppDataTableShell>

          <div class="actions">
            <button
              type="button"
              :disabled="!canApplyHistory"
              @click="historyStore.applySession()"
            >
              Apply
            </button>
            <button
              type="button"
              :disabled="!canExportPreset"
              @click="historyStore.exportPreset()"
            >
              Export preset
            </button>
            <button
              type="button"
              :disabled="!canImportPreset"
              @click="historyStore.importPreset()"
            >
              Import preset
            </button>
            <button
              type="button"
              :disabled="!canClearHistory"
              @click="queueClearSessionConfirmation"
            >
              Clear
            </button>
          </div>
        </section>

        <section class="pane historyPaneSaved panelProminentPane">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Run library</div>
              <h3 class="panelPaneTitle">Saved Runs</h3>
              <div class="panelSectionLead">
                Search by strategy, dataset, or tag, then choose one artifact to
                load, rerun, or inspect in detail below.
              </div>
            </div>
          </div>
          <div
            class="sectionMeta panelMuted metaLine mono panelNumeric"
            role="status"
            aria-live="polite"
          >
            {{ savedRunsMeta }}
          </div>
          <div class="panelMuted searchScopeMeta">
            {{ savedRunsSearchSummary }}
          </div>
          <AppDataTableShell
            title="Saved run artifacts"
            :summary="savedRunsSearchSummary"
            :row-count="visibleSavedRuns.length"
            :viewport="false"
            :density="savedRunsTableDensity"
            toolbar-label="Saved run library controls"
            aria-label="Saved run artifacts shell"
            empty-title="No saved runs match the current filters."
            empty-body="Broaden the saved-run search, clear tag filters, or refresh the run library."
            keyboard-hint="Use arrow keys, Home, and End to move through saved run artifacts."
          >
            <template #controls>
              <div class="historyTableToolbar">
                <div class="historySearchFields">
                  <div>
                    <label for="vueSavedRunsSearch">Saved run search</label>
                    <input
                      id="vueSavedRunsSearch"
                      type="text"
                      :value="searchDraft"
                      placeholder="strategy, dataset, tag, artifact…"
                      @input="
                        onSavedSearch(
                          String(($event.target as HTMLInputElement).value || ''),
                        )
                      "
                    />
                    <div class="panelMuted fieldHint">
                      Filters Saved Runs only. Use Ctrl+K for global commands and
                      navigation.
                    </div>
                  </div>
                  <div>
                    <label for="vueSavedRunsTags">Tags</label>
                    <div class="inputWithAction">
                      <input
                        id="vueSavedRunsTags"
                        type="text"
                        :value="tagsDraft"
                        placeholder="aapl,1m,robust"
                        @input="
                          onSavedTags(
                            String(($event.target as HTMLInputElement).value || ''),
                          )
                        "
                      />
                      <button
                        type="button"
                        :disabled="!canSaveSavedRunTags"
                        @click="saveSavedRunTags"
                      >
                        Save
                      </button>
                    </div>
                  </div>
                </div>
                <AppTableDensityControl
                  v-model:density="savedRunsTableDensity"
                  label="Library density"
                  qa-prefix="history.savedRuns.density"
                />
              </div>
            </template>
            <ul
              ref="savedRunsScrollElementRef"
              class="runList savedRunList overflowShadowY surfaceWorking"
              :class="{ historyVirtualListViewport: true }"
              aria-label="Saved run artifacts"
            >
              <li
                v-if="savedRunsTopSpacerPx > 0"
                class="runListSpacer"
                aria-hidden="true"
                :style="{ height: `${savedRunsTopSpacerPx}px` }"
              ></li>
              <li
                v-for="virtualRow in virtualSavedRunRows"
                :key="`saved_run_${virtualRow.row.path}`"
                class="runListItem"
              >
                <button
                  type="button"
                  class="runListButton"
                  data-saved-run="1"
                  :class="{ active: savedValue === virtualRow.row.path }"
                  :aria-pressed="savedValue === virtualRow.row.path"
                  :aria-posinset="virtualRow.index + 1"
                  :aria-setsize="visibleSavedRuns.length"
                  @click="onSavedSelect(virtualRow.row.path)"
                  @keydown="onRunListKeydown"
                >
                  <div class="runListPrimary">
                    <span class="mono">{{
                      virtualRow.row.ts ||
                      basename(virtualRow.row.path) ||
                      "Saved run"
                    }}</span>
                    <span class="pill">{{
                      virtualRow.row.strategyKind || "Unknown"
                    }}</span>
                  </div>
                  <div class="runListSecondary panelMuted">
                    {{
                      basename(virtualRow.row.csvPath) || "Dataset unavailable"
                    }}
                    <span v-if="formatBytesCompact(virtualRow.row.bytes)">
                      · {{ formatBytesCompact(virtualRow.row.bytes) }}
                    </span>
                  </div>
                  <div class="pillRow">
                    <span
                      class="pill"
                      :class="{ good: virtualRow.row.hasValidation }"
                    >
                      {{
                        virtualRow.row.hasValidation
                          ? "validation"
                          : "no validation"
                      }}
                    </span>
                    <span
                      class="pill"
                      :class="{ good: hasSavedRunReport(virtualRow.row) }"
                    >
                      {{
                        hasSavedRunReport(virtualRow.row) ? "xlsx" : "artifact only"
                      }}
                    </span>
                    <span
                      v-if="
                        virtualRow.row.strategyKind === 'MlSignal' &&
                        savedRunMlScoreContract(virtualRow.row)
                      "
                      class="pill subtle"
                      :title="
                        savedRunMlScoreNote(virtualRow.row) ||
                        savedRunMlScoreContract(virtualRow.row)
                      "
                    >
                      {{ savedRunMlScoreContract(virtualRow.row) }}
                    </span>
                    <span
                      v-if="savedRunRouteSummary(virtualRow.row)"
                      class="pill subtle routePill"
                      :title="savedRunRouteSummary(virtualRow.row)"
                    >
                      {{ savedRunRouteSummary(virtualRow.row) }}
                    </span>
                    <span
                      v-for="tag in virtualRow.row.tags.slice(0, 3)"
                      :key="`saved_tag_${virtualRow.row.path}_${tag}`"
                      class="pill subtle"
                    >
                      {{ tag }}
                    </span>
                  </div>
                </button>
              </li>
              <li
                v-if="savedRunsBottomSpacerPx > 0"
                class="runListSpacer"
                aria-hidden="true"
                :style="{ height: `${savedRunsBottomSpacerPx}px` }"
              ></li>
            </ul>
          </AppDataTableShell>

          <div class="actions">
            <button
              type="button"
              data-qa-action="library.savedRun.refresh"
              :disabled="!canRefreshSavedRuns"
              @click="historyStore.refreshSavedRuns()"
            >
              Refresh
            </button>
            <button
              type="button"
              data-qa-action="library.savedRun.load"
              :disabled="!canLoadSavedRun"
              @click="loadSavedRun"
            >
              Load
            </button>
            <button
              type="button"
              data-qa-action="library.savedRun.apply"
              :disabled="!canApplySavedRun"
              @click="applySavedRun"
            >
              Apply
            </button>
            <button
              type="button"
              data-qa-action="library.savedRun.rerun"
              :disabled="!canRerunSavedRun"
              @click="rerunSavedRun"
            >
              Rerun
            </button>
            <button
              type="button"
              data-qa-action="library.savedRun.openReport"
              :disabled="!canOpenSavedRunReport"
              @click="openSavedRunReport"
            >
              Open report
            </button>
            <button
              type="button"
              data-qa-action="library.savedRun.reveal"
              :disabled="!canRevealSavedRun"
              @click="revealSavedRun"
            >
              Reveal
            </button>
            <button
              type="button"
              data-qa-action="library.savedRun.delete"
              :disabled="!canDeleteSavedRun"
              @click="queueDeleteSavedRunConfirmation"
            >
              Delete
            </button>
            <button
              type="button"
              data-qa-action="library.runsDir.open"
              :disabled="!canOpenRunsDir"
              @click="historyStore.openRunsDir()"
            >
              Runs folder
            </button>
          </div>
        </section>
      </div>

      <section
        v-if="historyStage === 'ready'"
        class="detailCard panelProminentPane surfaceFocus selectionRail"
      >
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Selected run</div>
            <div class="panelSectionLead">
              Use this snapshot as the decision surface for report review,
              compare work, and rerun actions.
            </div>
          </div>
        </div>
        <div class="detailTitle">{{ savedRunDetailTitle }}</div>
        <div class="detailMeta panelMuted">{{ savedRunDetailMeta }}</div>
        <div class="detailGrid">
          <div>
            <span class="panelMetricKey">Strategy</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailStrategy
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Config Fingerprint</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailConfigFingerprint
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Dataset</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailDataset
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Timeframe</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailTimeframe
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Validation</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailValidation
            }}</span>
          </div>
          <div class="full">
            <span class="panelMetricKey">Execution Route</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailProvenance
            }}</span>
          </div>
          <div v-if="savedRunDetailLineage !== '—'" class="full">
            <span class="panelMetricKey">Lineage</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailLineage
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Metric Surface</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailMetricSurface
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Admission</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailAdmission
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Report</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailReport
            }}</span>
          </div>
          <div>
            <span class="panelMetricKey">Size</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailSize
            }}</span>
          </div>
          <template v-if="savedRunDetailStrategy === 'MlSignal'">
            <div>
              <span class="panelMetricKey">Bundle</span
              ><span class="panelMetricValue mono panelNumeric">{{
                savedRunDetailMlBundle
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Score Cache</span
              ><span class="panelMetricValue mono panelNumeric">{{
                savedRunDetailMlScoreCache
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Feature Schema</span
              ><span class="panelMetricValue mono panelNumeric">{{
                savedRunDetailMlFeatureSchema
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Score Contract</span
              ><span class="panelMetricValue mono panelNumeric">{{
                savedRunDetailMlScoreContract
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Policy</span
              ><span class="panelMetricValue mono panelNumeric">{{
                savedRunDetailMlPolicy
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Sweep</span
              ><span class="panelMetricValue mono panelNumeric">{{
                savedRunDetailMlSweep
              }}</span>
            </div>
            <div>
              <span class="panelMetricKey">Verdict</span
              ><span class="panelMetricValue mono panelNumeric">{{
                savedRunDetailMlVerdict
              }}</span>
            </div>
            <div class="full">
              <span class="panelMetricKey">Top Candidates</span>
              <pre class="panelMetricValue mono panelNumeric inlineBlock">{{
                savedRunDetailMlCandidates
              }}</pre>
            </div>
            <div v-if="savedRunDetailMlScoreNote !== '—'" class="full">
              <span class="panelMetricKey">Score Note</span
              ><span class="panelMetricValue mono panelNumeric">{{
                savedRunDetailMlScoreNote
              }}</span>
            </div>
          </template>
          <div class="full">
            <span class="panelMetricKey">Tags</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailTags
            }}</span>
          </div>
          <div v-if="savedRunDetailMetricSurfaceDetail !== '—'" class="full">
            <span class="panelMetricKey">Metric Detail</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailMetricSurfaceDetail
            }}</span>
          </div>
          <div v-if="savedRunDetailAdmissionDetail !== '—'" class="full">
            <span class="panelMetricKey">Admission Detail</span
            ><span class="panelMetricValue mono panelNumeric">{{
              savedRunDetailAdmissionDetail
            }}</span>
          </div>
        </div>
        <details class="panelDetails">
          <summary>Request snapshot</summary>
          <div class="panelDetailsBody">
            <div class="panelMuted">
              The full request payload stays available for troubleshooting or
              manual rerun inspection.
            </div>
            <pre class="mono">{{ savedRunDetailParams }}</pre>
          </div>
        </details>
        <details class="panelDetails">
          <summary>Saved run compare</summary>
          <div class="panelDetailsBody compareDetailsBody">
            <div class="panelMuted">
              Compare this saved artifact against another saved run without
              switching the current workflow winner.
            </div>
            <label for="historySavedRunCompareSelect">Compare against</label>
            <select
              id="historySavedRunCompareSelect"
              :value="compareDraft"
              @change="
                onSavedRunCompareSelect(
                  String(($event.target as HTMLSelectElement).value || ''),
                )
              "
            >
              <option
                v-for="option in savedRunCompareOptions"
                :key="`saved_compare_${option.value}_${option.label}`"
                :value="option.value"
                :disabled="option.disabled"
              >
                {{ option.label }}
              </option>
            </select>
            <pre class="mono">{{ savedRunCompareSummary }}</pre>
          </div>
        </details>
      </section>

      <OptionsExpressionPanel
        v-if="historyStage === 'ready'"
        :parent-artifact-path="String(savedValue || '').trim()"
        :parent-strategy-kind="savedRunDetailStrategy"
        :parent-dataset-label="savedRunDetailDataset"
      />

      <div
        class="panelStatus mono panelNumeric"
        role="status"
        aria-live="polite"
      >
        {{ statusLine }}
      </div>
      <ConfirmationDialog
        id-base="history_confirmation"
        :open="pendingAction !== null"
        :title="confirmationTitle"
        :body="confirmationBody"
        :confirm-label="confirmationConfirmLabel"
        :confirm-disabled="!canConfirmPendingAction"
        danger
        @cancel="cancelPendingAction"
        @confirm="confirmPendingAction"
      />
    </section>
  </Teleport>
</template>

<style scoped>
.historyPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.grid {
  display: grid;
  grid-template-columns: minmax(0, 0.85fr) minmax(0, 1.15fr);
  gap: 0.8rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.historyPaneSession {
  background: color-mix(
    in oklab,
    var(--panel-footer) 92%,
    var(--surface-sunken)
  );
}

.historyPaneSaved {
  background: color-mix(
    in oklab,
    var(--surface-working) 94%,
    var(--panel-footer)
  );
  border-color: color-mix(in oklab, var(--panel-focus-edge) 54%, var(--border));
}

.compareDetailsBody {
  display: grid;
  gap: 0.55rem;
}

.sectionMeta {
  margin-bottom: 0.55rem;
  font-size: 0.78rem;
}

.historyTableToolbar {
  width: 100%;
  min-width: 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.65rem;
  align-items: start;
}

.historySearchFields {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.6rem;
}

.inputWithAction {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.45rem;
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

.emptyStateActions {
  justify-content: center;
}

.metaLine {
  margin-top: 0.55rem;
  font-size: 0.78rem;
}

.searchScopeMeta {
  margin-top: 0.35rem;
  margin-bottom: 0.55rem;
  font-size: 0.76rem;
}

.fieldHint {
  margin-top: 0.28rem;
  font-size: 0.74rem;
}

.runList {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 260px;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-working) 96%, var(--panel));
}

.savedRunList {
  max-height: 320px;
}

.historyVirtualListViewport {
  max-height: clamp(18rem, 42vh, 20rem);
}

.runListItem + .runListItem {
  border-top: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
}

.runListSpacer {
  pointer-events: none;
  border-top: none !important;
}

.runListButton {
  width: 100%;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: inherit;
  padding: 0.7rem 0.75rem;
  display: grid;
  gap: 0.35rem;
  text-align: left;
  transition:
    background-color var(--motion-fast) var(--ease-standard),
    box-shadow var(--motion-fast) var(--ease-standard),
    transform var(--motion-fast) var(--ease-standard);
}

.runListButton:hover:not(:disabled) {
  background: color-mix(in oklab, var(--panel) 92%, var(--surface));
  transform: translateX(1px);
}

.runListButton:focus-visible {
  outline: none;
  box-shadow: inset 0 0 0 2px var(--focus-border);
}

.runListButton.active {
  background: color-mix(
    in oklab,
    var(--selection-fill-strong) 82%,
    var(--surface-working)
  );
  box-shadow: inset 3px 0 0 var(--selection-rail);
}

.runListPrimary {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.45rem;
}

.runListSecondary {
  font-size: 0.78rem;
}

.pillRow {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}

.pill {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 0.16rem 0.45rem;
  font-size: 0.68rem;
  line-height: 1.2;
  background: color-mix(in oklab, var(--panel) 88%, var(--surface));
}

.pill.good {
  border-color: color-mix(in oklab, var(--accent) 62%, var(--border));
  background: color-mix(in oklab, var(--accent-soft) 52%, var(--surface));
}

.pill.subtle {
  color: var(--muted);
}

.routePill {
  max-width: 100%;
  display: inline-block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.listEmpty {
  border: 1px dashed var(--border);
  border-radius: 12px;
  padding: 0.75rem;
  background: color-mix(in oklab, var(--surface) 95%, var(--panel));
  font-size: 0.8rem;
}

.emptyStateIllustration {
  width: clamp(88px, 16vw, 124px);
  height: auto;
  margin-bottom: 0.2rem;
  opacity: 0.92;
}

.detailCard {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: linear-gradient(
    145deg,
    color-mix(in oklab, var(--surface-focus) 92%, var(--panel-footer)),
    color-mix(in oklab, var(--panel-footer) 94%, var(--surface-working))
  );
  padding: 0.75rem 0.8rem;
  box-shadow: var(--shadow-focus);
}

.detailTitle {
  font-weight: 600;
}

.detailMeta {
  margin-top: 0.2rem;
  font-size: 0.82rem;
}

.detailGrid {
  margin-top: 0.6rem;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.4rem 0.75rem;
}

.detailGrid .full {
  grid-column: 1 / -1;
}

.mono {
  font-family: var(--mono);
}

pre {
  margin-top: 0.35rem;
  max-height: 260px;
  overflow: auto;
}

@media (max-width: 980px) {
  .grid {
    grid-template-columns: 1fr;
  }

  .historyTableToolbar,
  .historySearchFields {
    grid-template-columns: 1fr;
  }

  .detailGrid {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel="history"][data-vue-history="1"] > :not(#historyVueShellMount) {
  display: none !important;
}
</style>
