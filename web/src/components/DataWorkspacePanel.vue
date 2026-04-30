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
import { useShellMount } from "../composables/useShellMount";
import { useQueuedInputWriter } from "../composables/useQueuedInputWriter";
import ContextHelpTip from "./ContextHelpTip.vue";
import {
  AppButton,
  AppMetricStrip,
  AppPanel,
  AppStatusChip,
  AppToolbar,
} from "./ui";
import { useDataStore } from "../stores/dataStore";
import { useWorkflowStore } from "../stores/workflowStore";
import { formatUiTimeFromUnixMs } from "../utils/uiTime";

const dataStore = useDataStore();
const workflowStore = useWorkflowStore();
const {
  csvPath,
  recentCsvPaths,
  sampleCsvOptions,
  sampleCsvVisible,
  marketDataDir,
  marketDataDirHint,
  dataId,
  datasetLabel,
  datasetRange,
  dataLen,
  dataTimeframe,
  dataTimeframeHint,
  autoRestoreDataset,
  datasetContext,
  dataFields,
  dataFieldHint,
  canBrowseCsv,
  canPolygonOpen,
  canLoadCsv,
  canClearData,
  canOpenMarketDataDir,
  canApplyTimeframe,
  polygonOpen,
  polygonApiKey,
  polygonMarket,
  polygonSymbol,
  polygonFrom,
  polygonTo,
  polygonTimeframe,
  polygonAdjusted,
  polygonCacheDir,
  polygonHasApiKey,
  polygonDatasets,
  polygonSelectedPath,
  polygonSelectedMeta,
  polygonCacheMeta,
  canPolygonRefresh,
  canPolygonSaveApiKey,
  canPolygonClearApiKey,
  canPolygonDownload,
  canPolygonLoadSelected,
  canPolygonValidateSelected,
  canPolygonRepairSelected,
  statusLine,
  baseDataMeta,
  updatedAtUnixMs,
} = storeToRefs(dataStore);
const { controlsTab } = storeToRefs(workflowStore);

const { mountReady } = useShellMount("#dataVueShellMount", {
  paneSelector: '[data-vue-panel="data"]',
  paneDatasetKey: "vueData",
});
const polygonSymbolInput = ref<HTMLInputElement | null>(null);
const polygonApiKeyInput = ref<HTMLInputElement | null>(null);

const csvPathDraft = ref("");
const marketDataDirDraft = ref("");
const timeframeDraft = ref("");
const polygonApiKeyDraft = ref("");
const polygonMarketDraft = ref("stocks");
const polygonSymbolDraft = ref("");
const polygonFromDraft = ref("");
const polygonToDraft = ref("");
const polygonTimeframeDraft = ref("1m");
const polygonSelectedPathDraft = ref("");

const visible = computed(
  () => String(controlsTab.value || "").trim() === "data",
);

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value));
const selectedPolygonDatasetSummary = computed(() => {
  const selectedPath = String(
    polygonSelectedPathDraft.value || polygonSelectedPath.value || "",
  ).trim();
  if (!selectedPath) return "No cached Polygon dataset selected.";
  const selected = polygonDatasets.value.find(
    (entry) => entry.csvPath === selectedPath,
  );
  if (!selected) return `Selected cached Polygon dataset ${selectedPath}.`;
  const label = String(
    selected.displaySymbol ||
      selected.polygonTicker ||
      selected.csvPath ||
      "dataset",
  ).trim();
  const parts = [label];
  if (String(selected.timeframe || "").trim()) {
    parts.push(String(selected.timeframe).trim());
  }
  if (Number.isFinite(selected.rowCount)) {
    parts.push(`${selected.rowCount} rows`);
  }
  return `Selected cached Polygon dataset ${parts.join(" · ")}.`;
});

const polygonAdjustedDisabled = computed(
  () => String(polygonMarketDraft.value || "stocks").trim() !== "stocks",
);
const polygonAdjustedHint = computed(() =>
  polygonAdjustedDisabled.value
    ? "Split adjustment only applies to stock aggregate bars."
    : "Stock aggregates default to split-adjusted bars.",
);

function parseDateInputMs(value: string): number | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(String(value || "").trim());
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const ms = Date.UTC(year, month - 1, day);
  if (!Number.isFinite(ms)) return null;
  const parsed = new Date(ms);
  if (
    parsed.getUTCFullYear() !== year ||
    parsed.getUTCMonth() !== month - 1 ||
    parsed.getUTCDate() !== day
  ) {
    return null;
  }
  return ms;
}

const polygonDateRangeHint = computed(() => {
  const symbol = String(polygonSymbolDraft.value || "").trim();
  const from = String(polygonFromDraft.value || "").trim();
  const to = String(polygonToDraft.value || "").trim();
  if (!symbol || !from || !to) {
    return "Choose a symbol and date range. Downloads cache 1m bars; selected timeframes load from local resampling.";
  }
  const fromMs = parseDateInputMs(from);
  const toMs = parseDateInputMs(to);
  if (fromMs === null || toMs === null) return "Dates must use YYYY-MM-DD.";
  if (toMs < fromMs) return "End date must be on or after start date.";
  const days = Math.max(1, Math.floor((toMs - fromMs) / 86_400_000) + 1);
  const requests = Math.max(1, Math.ceil(days / 30));
  const plural = requests === 1 ? "" : "s";
  return `About ${requests} Polygon request${plural} before cache reuse. Free REST keys are rate-limited; long ranges run in 30-day chunks.`;
});

const rowLabel = computed(() => {
  const rows = String(dataLen.value || "").trim();
  return rows || "—";
});
const effectiveTimeframe = computed(() => {
  const current = String(dataTimeframe.value || "").trim();
  if (current) return current;
  return String(baseDataMeta.value?.timeframe || "").trim();
});
const dataSummaryChips = computed(() => {
  const chips: string[] = [];
  const dataset = String(datasetLabel.value || "").trim();
  const timeframe = String(effectiveTimeframe.value || "").trim();
  const baseTimeframe = String(baseDataMeta.value?.timeframe || "").trim();
  const loadedId = String(dataId.value || "").trim();
  const fieldCount = dataFields.value.length;
  if (dataset) chips.push(`Dataset: ${dataset}`);
  chips.push(`Rows: ${rowLabel.value}`);
  if (timeframe) chips.push(`Timeframe: ${timeframe}`);
  if (timeframe && baseTimeframe && timeframe !== baseTimeframe) {
    chips.push(`Base timeframe: ${baseTimeframe}`);
  }
  if (loadedId) chips.push(`Loaded ID: ${loadedId}`);
  if (fieldCount > 0) chips.push(`Fields: ${fieldCount}`);
  return chips;
});
const datasetMetricItems = computed(() => [
  { key: "loaded_id", label: "Loaded ID", value: String(dataId.value || "—") },
  { key: "rows", label: "Rows", value: String(dataLen.value || "—") },
  {
    key: "dataset",
    label: "Dataset",
    value: String(datasetLabel.value || "—"),
  },
  { key: "range", label: "Range", value: String(datasetRange.value || "—") },
]);

watch(
  () =>
    [
      String(csvPath.value || ""),
      String(marketDataDir.value || ""),
      String(dataTimeframe.value || ""),
      String(polygonApiKey.value || ""),
      String(polygonMarket.value || "stocks"),
      String(polygonSymbol.value || ""),
      String(polygonFrom.value || ""),
      String(polygonTo.value || ""),
      String(polygonTimeframe.value || "1m"),
      String(polygonSelectedPath.value || ""),
    ] as const,
  ([
    nextCsvPath,
    nextMarketDataDir,
    nextTimeframe,
    nextPolygonApiKey,
    nextPolygonMarket,
    nextPolygonSymbol,
    nextPolygonFrom,
    nextPolygonTo,
    nextPolygonTimeframe,
    nextPolygonSelectedPath,
  ]) => {
    if (csvPathDraft.value !== nextCsvPath) csvPathDraft.value = nextCsvPath;
    if (marketDataDirDraft.value !== nextMarketDataDir)
      marketDataDirDraft.value = nextMarketDataDir;
    if (timeframeDraft.value !== nextTimeframe)
      timeframeDraft.value = nextTimeframe;
    if (polygonApiKeyDraft.value !== nextPolygonApiKey)
      polygonApiKeyDraft.value = nextPolygonApiKey;
    if (polygonMarketDraft.value !== nextPolygonMarket)
      polygonMarketDraft.value = nextPolygonMarket;
    if (polygonSymbolDraft.value !== nextPolygonSymbol)
      polygonSymbolDraft.value = nextPolygonSymbol;
    if (polygonFromDraft.value !== nextPolygonFrom)
      polygonFromDraft.value = nextPolygonFrom;
    if (polygonToDraft.value !== nextPolygonTo)
      polygonToDraft.value = nextPolygonTo;
    if (polygonTimeframeDraft.value !== nextPolygonTimeframe)
      polygonTimeframeDraft.value = nextPolygonTimeframe;
    if (polygonSelectedPathDraft.value !== nextPolygonSelectedPath)
      polygonSelectedPathDraft.value = nextPolygonSelectedPath;
  },
  { immediate: true },
);

watch(
  () => Boolean(polygonOpen.value),
  async (open) => {
    if (!open) return;
    await nextTick();
    const focusTarget = polygonSymbolInput.value ?? polygonApiKeyInput.value;
    focusTarget?.focus();
  },
);

function setInputValue(id: string, nextValue: string): void {
  dataStore.setInputField(id, nextValue);
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } =
  useQueuedInputWriter(setInputValue);

function setSelectValue(id: string, nextValue: string): void {
  dataStore.setSelectField(id, nextValue);
}

function browseCsv(): void {
  dataStore.browseCsv();
}

function togglePolygonSection(): void {
  if (polygonOpen.value) {
    dataStore.closePolygon();
    return;
  }
  dataStore.openPolygon();
}

function loadCsvFromDraft(): void {
  flushQueuedInputValuesNow();
  dataStore.loadCsv(String(csvPathDraft.value || "").trim());
}

function applyTimeframeFromDraft(): void {
  dataStore.applyTimeframe(String(timeframeDraft.value || "").trim());
}

function openStorageSettings(): void {
  workflowStore.selectWorkspace("settings");
}

function setCheckboxValue(id: string, checked: boolean): void {
  dataStore.setCheckboxField(id, checked);
}

function setPolygonCheckboxValue(id: string, checked: boolean): void {
  dataStore.setCheckboxField(id, checked);
}

function applyRecentPath(path: string): void {
  const nextValue = String(path || "").trim();
  if (!nextValue) return;
  csvPathDraft.value = nextValue;
  setInputValue("csvPath", nextValue);
}

function loadSampleCsv(path: string, index: number): void {
  dataStore.loadSampleCsv(index, path);
}

function selectPolygonDataset(path: string): void {
  polygonSelectedPathDraft.value = String(path || "").trim();
  setInputValue("polygonSelectedPath", polygonSelectedPathDraft.value);
}

function polygonDatasetMetaId(index: number): string {
  return `polygon_dataset_meta_${index}`;
}

function movePolygonDatasetFocus(
  currentTarget: EventTarget | null,
  direction: "prev" | "next" | "first" | "last",
): void {
  const button = currentTarget instanceof HTMLElement ? currentTarget : null;
  const list = button?.closest(".stackList");
  if (!button || !list) return;
  const buttons = Array.from(
    list.querySelectorAll<HTMLButtonElement>("button.listCard:not(:disabled)"),
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

function onPolygonDatasetKeydown(event: KeyboardEvent): void {
  if (event.key === "ArrowUp") {
    event.preventDefault();
    movePolygonDatasetFocus(event.currentTarget, "prev");
    return;
  }
  if (event.key === "ArrowDown") {
    event.preventDefault();
    movePolygonDatasetFocus(event.currentTarget, "next");
    return;
  }
  if (event.key === "Home") {
    event.preventDefault();
    movePolygonDatasetFocus(event.currentTarget, "first");
    return;
  }
  if (event.key === "End") {
    event.preventDefault();
    movePolygonDatasetFocus(event.currentTarget, "last");
  }
}

onMounted(async () => {
  await nextTick();
  dataStore.hydrateStartupState();
});

onBeforeUnmount(() => {
  flushQueuedInputValuesNow();
});
</script>

<template>
  <Teleport v-if="mountReady" to="#dataVueShellMount">
    <section class="dataPanelVueRoot dataVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Dataset</div>
          <h2 class="panelTitle">Data Source</h2>
          <div class="panelSubtitle">
            Load a dataset, choose storage behavior, and confirm the working
            timeframe before you optimize.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">
          {{ rowLabel }} rows · {{ stamp }}
        </div>
      </header>
      <div class="panelSummaryChips" aria-label="Dataset summary">
        <AppStatusChip
          v-for="chip in dataSummaryChips"
          :key="`data_summary_${chip}`"
          class="panelSummaryChip mono"
          tone="muted"
          >{{ chip }}</AppStatusChip
        >
      </div>

      <AppPanel eyebrow="Import" title="CSV input" variant="prominent">
        <label for="vueCsvPath">CSV path</label>
        <div class="inputWithActions">
          <input
            id="vueCsvPath"
            type="text"
            :value="csvPathDraft"
            placeholder="C:\\path\\to\\candles.csv"
            @input="
              csvPathDraft = String(
                ($event.target as HTMLInputElement).value || '',
              );
              queueInputValue('csvPath', csvPathDraft);
            "
            @change="commitInputValue('csvPath', csvPathDraft)"
          />
          <AppToolbar aria-label="CSV data actions">
            <AppButton
              size="sm"
              data-qa-action="data.csv.browse"
              :disabled="!canBrowseCsv"
              @click="browseCsv"
            >
              Browse
            </AppButton>
            <AppButton
              size="sm"
              data-qa-action="data.polygon.toggle"
              :disabled="!canPolygonOpen"
              @click="togglePolygonSection"
            >
              {{ polygonOpen ? "Hide Polygon" : "Polygon" }}
            </AppButton>
            <AppButton
              variant="primary"
              size="sm"
              data-qa-action="data.csv.loadDraft"
              :disabled="!canLoadCsv"
              @click="loadCsvFromDraft"
            >
              Load CSV
            </AppButton>
            <AppButton
              size="sm"
              data-qa-action="data.dataset.clear"
              :disabled="!canClearData"
              @click="dataStore.clearData()"
            >
              Clear
            </AppButton>
          </AppToolbar>
        </div>
        <div class="recentWrap" v-if="recentCsvPaths.length">
          <div class="k">Recent</div>
          <div class="chips">
            <button
              v-for="path in recentCsvPaths"
              :key="`recent_${path}`"
              type="button"
              class="chip mono"
              data-qa-action="data.csv.applyRecent"
              @click="applyRecentPath(path)"
            >
              {{ path }}
            </button>
          </div>
        </div>
      </AppPanel>

      <AppPanel
        v-if="sampleCsvVisible && sampleCsvOptions.length"
        eyebrow="Quick Start"
        title="Sample datasets"
      >
        <AppToolbar aria-label="Sample dataset actions">
          <AppButton
            v-for="(sample, index) in sampleCsvOptions"
            :key="`sample_${index}_${sample.label}`"
            size="sm"
            data-qa-action="data.sample.load"
            :disabled="sample.disabled"
            :title="sample.detail || sample.description || sample.label"
            @click="loadSampleCsv(sample.path, index)"
          >
            {{ sample.label }}
          </AppButton>
        </AppToolbar>
      </AppPanel>

      <section class="pane paneProminent" v-if="polygonOpen">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Market Download</div>
            <h3 class="panelPaneTitle">Polygon</h3>
          </div>
          <div class="actions">
            <button
              type="button"
              data-qa-action="data.polygon.refresh"
              :disabled="!canPolygonRefresh"
              @click="dataStore.refreshPolygon()"
            >
              Refresh cache
            </button>
            <button
              type="button"
              data-qa-action="data.polygon.close"
              @click="dataStore.closePolygon()"
            >
              Close
            </button>
          </div>
        </div>
        <div class="muted mono">{{ polygonCacheMeta }}</div>

        <div class="gridTwo">
          <div>
            <label for="vuePolygonApiKey">API key</label>
            <div class="inputWithActions">
              <input
                id="vuePolygonApiKey"
                ref="polygonApiKeyInput"
                type="password"
                :value="polygonApiKeyDraft"
                placeholder="Paste Polygon API key"
                @input="
                  polygonApiKeyDraft = String(
                    ($event.target as HTMLInputElement).value || '',
                  );
                  queueInputValue('polygonApiKey', polygonApiKeyDraft);
                "
                @change="commitInputValue('polygonApiKey', polygonApiKeyDraft)"
              />
              <div class="actions">
                <button
                  type="button"
                  :disabled="!canPolygonSaveApiKey"
                  @click="dataStore.savePolygonApiKey()"
                >
                  Save key
                </button>
                <button
                  type="button"
                  :disabled="!canPolygonClearApiKey"
                  @click="dataStore.clearPolygonApiKey()"
                >
                  Clear key
                </button>
              </div>
            </div>
            <div class="muted mono">
              {{
                polygonHasApiKey
                  ? "API key saved in secure system storage when available. Local Polygon settings remain a compatibility fallback."
                  : "Save an API key to enable Polygon historical download and live workflows."
              }}
            </div>
          </div>

          <div>
            <label for="vuePolygonCacheDir">Cache folder</label>
            <input
              id="vuePolygonCacheDir"
              type="text"
              :value="polygonCacheDir"
              readonly
            />
            <div class="muted mono">
              Downloaded 1m bars are cached here and reused automatically;
              selected timeframes load from local resampling.
            </div>
          </div>
        </div>

        <div class="gridTwo">
          <div>
            <label for="vuePolygonMarket">Market</label>
            <select
              id="vuePolygonMarket"
              :value="polygonMarketDraft"
              @change="
                polygonMarketDraft = String(
                  ($event.target as HTMLSelectElement).value || 'stocks',
                );
                setSelectValue('polygonMarket', polygonMarketDraft);
              "
            >
              <option value="stocks">Stocks</option>
              <option value="fx">FX</option>
              <option value="crypto">Crypto</option>
            </select>
          </div>
          <div>
            <label for="vuePolygonTimeframe">Timeframe</label>
            <select
              id="vuePolygonTimeframe"
              :value="polygonTimeframeDraft"
              @change="
                polygonTimeframeDraft = String(
                  ($event.target as HTMLSelectElement).value || '1m',
                );
                setSelectValue('polygonTimeframe', polygonTimeframeDraft);
              "
            >
              <option value="1m">1m</option>
              <option value="2m">2m</option>
              <option value="5m">5m</option>
              <option value="15m">15m</option>
              <option value="30m">30m</option>
              <option value="1h">1h</option>
              <option value="4h">4h</option>
              <option value="1d">1d</option>
            </select>
          </div>
        </div>

        <div class="gridTwo">
          <div>
            <label for="vuePolygonSymbol">Symbol</label>
            <input
              id="vuePolygonSymbol"
              ref="polygonSymbolInput"
              type="text"
              :value="polygonSymbolDraft"
              placeholder="AAPL"
              @input="
                polygonSymbolDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                ).toUpperCase();
                queueInputValue('polygonSymbol', polygonSymbolDraft);
              "
              @change="commitInputValue('polygonSymbol', polygonSymbolDraft)"
            />
          </div>
          <div class="rowInline">
            <label class="inlineCheck">
              <input
                type="checkbox"
                :checked="!polygonAdjustedDisabled && polygonAdjusted"
                :disabled="polygonAdjustedDisabled"
                @change="
                  setPolygonCheckboxValue(
                    'polygonAdjusted',
                    ($event.target as HTMLInputElement).checked,
                  )
                "
              />
              Adjusted prices
            </label>
            <div class="muted mono">{{ polygonAdjustedHint }}</div>
          </div>
        </div>

        <div class="gridTwo">
          <div>
            <label for="vuePolygonFrom">From</label>
            <input
              id="vuePolygonFrom"
              type="date"
              :value="polygonFromDraft"
              @input="
                polygonFromDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueInputValue('polygonFrom', polygonFromDraft);
              "
              @change="commitInputValue('polygonFrom', polygonFromDraft)"
            />
          </div>
          <div>
            <label for="vuePolygonTo">To</label>
            <input
              id="vuePolygonTo"
              type="date"
              :value="polygonToDraft"
              @input="
                polygonToDraft = String(
                  ($event.target as HTMLInputElement).value || '',
                );
                queueInputValue('polygonTo', polygonToDraft);
              "
              @change="commitInputValue('polygonTo', polygonToDraft)"
            />
          </div>
        </div>

        <div class="muted mono">{{ polygonDateRangeHint }}</div>

        <div class="actions">
          <button
            type="button"
            class="primary"
            :disabled="!canPolygonDownload"
            @click="dataStore.downloadPolygonDataset()"
          >
            Download and load
          </button>
        </div>

        <div class="pane subtlePane">
          <div class="panelSectionHeader">
            <div>
              <div class="panelEyebrow">Cache</div>
              <h3 class="panelPaneTitle">Cached datasets</h3>
            </div>
          </div>
          <div
            id="polygonSelectionStatus"
            class="srOnly"
            role="status"
            aria-live="polite"
          >
            {{ selectedPolygonDatasetSummary }}
          </div>
          <div
            v-if="polygonDatasets.length"
            class="stackList"
            role="group"
            aria-label="Cached Polygon datasets"
          >
            <button
              v-for="(dataset, index) in polygonDatasets"
              :key="`polygon_dataset_${dataset.csvPath}`"
              type="button"
              class="listCard"
              :data-selected="
                polygonSelectedPath === dataset.csvPath ? '1' : '0'
              "
              :aria-pressed="polygonSelectedPath === dataset.csvPath"
              :aria-describedby="`${polygonDatasetMetaId(index)} polygonSelectionStatus`"
              @click="selectPolygonDataset(dataset.csvPath)"
              @keydown="onPolygonDatasetKeydown"
            >
              <span class="mono">{{
                dataset.displaySymbol ||
                dataset.polygonTicker ||
                dataset.csvPath
              }}</span>
              <span :id="polygonDatasetMetaId(index)" class="muted mono"
                >{{ dataset.timeframe }} · {{ dataset.rowCount }} rows</span
              >
            </button>
          </div>
          <div v-else class="muted">
            No cached Polygon datasets yet. Save a key, choose a symbol/date
            range, then download.
          </div>
          <div class="muted mono">{{ polygonSelectedMeta }}</div>
          <div class="actions">
            <button
              type="button"
              :disabled="!canPolygonLoadSelected"
              @click="dataStore.loadPolygonDataset(polygonSelectedPathDraft)"
            >
              Load selected
            </button>
            <button
              type="button"
              :disabled="!canPolygonValidateSelected"
              @click="
                dataStore.validatePolygonDataset(polygonSelectedPathDraft)
              "
            >
              Validate selected
            </button>
            <button
              type="button"
              :disabled="!canPolygonRepairSelected"
              @click="dataStore.repairPolygonDataset(polygonSelectedPathDraft)"
            >
              Repair selected
            </button>
          </div>
        </div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Context</div>
            <h3 class="panelPaneTitle">Dataset context</h3>
          </div>
        </div>
        <AppMetricStrip
          :items="datasetMetricItems"
          aria-label="Dataset context metrics"
        />
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Resolution</div>
            <h3 class="panelPaneTitle">Timeframe</h3>
          </div>
        </div>
        <div class="inputWithActions">
          <label for="vueDataTimeframe" class="srOnly">Target timeframe</label>
          <select
            id="vueDataTimeframe"
            :value="timeframeDraft"
            @change="
              timeframeDraft = String(
                ($event.target as HTMLSelectElement).value || '',
              );
              setSelectValue('dataTimeframe', timeframeDraft);
            "
          >
            <option value="">As loaded</option>
            <option value="1m">1m</option>
            <option value="2m">2m</option>
            <option value="3m">3m</option>
            <option value="5m">5m</option>
            <option value="15m">15m</option>
            <option value="30m">30m</option>
            <option value="1h">1h</option>
            <option value="2h">2h</option>
            <option value="4h">4h</option>
            <option value="1d">1d</option>
          </select>
          <div class="actions">
            <button
              type="button"
              :disabled="!canApplyTimeframe"
              @click="applyTimeframeFromDraft"
            >
              Apply timeframe
            </button>
          </div>
        </div>
        <label class="inlineCheck">
          <input
            type="checkbox"
            :checked="autoRestoreDataset"
            @change="
              setCheckboxValue(
                'autoRestoreDataset',
                ($event.target as HTMLInputElement).checked,
              )
            "
          />
          Restore last dataset and timeframe on startup
        </label>
        <div class="muted mono">{{ dataTimeframeHint }}</div>
        <div class="muted mono">{{ datasetContext }}</div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Schema</div>
            <h3 class="panelPaneTitle">Columns</h3>
          </div>
        </div>
        <div class="badges">
          <span
            class="badge mono"
            v-for="field in dataFields"
            :key="`field_${field}`"
            >{{ field }}</span
          >
          <span v-if="!dataFields.length" class="muted"
            >No columns available.</span
          >
        </div>
        <div class="muted mono">{{ dataFieldHint || "—" }}</div>
      </section>

      <section class="pane subtlePane">
        <div class="panelSectionHeader">
          <div>
            <div class="panelEyebrow">Storage</div>
            <h3 class="panelPaneTitle">Storage and cache</h3>
          </div>
          <ContextHelpTip
            title="Storage controls"
            summary="Review the active market-data folder here and jump to Settings when you need to change or migrate it."
            detail="Directory-level storage management is centralized in Settings so the data step stays focused on loading, downloading, and validating datasets. Opening the folder remains available here for quick inspection."
            aria-label="Storage controls help"
          />
        </div>
        <label for="vueMarketDataDir">Market data folder</label>
        <div class="inputWithActions">
          <input
            id="vueMarketDataDir"
            type="text"
            :value="marketDataDirDraft"
            placeholder="Default (AppData)"
            readonly
          />
          <div class="actions">
            <button
              type="button"
              :disabled="!canOpenMarketDataDir"
              @click="dataStore.openMarketDataDir()"
            >
              Open folder
            </button>
            <button type="button" @click="openStorageSettings">
              Manage in Settings
            </button>
          </div>
        </div>
        <div class="muted mono">{{ marketDataDirHint }}</div>
      </section>

      <div class="status mono" role="status" aria-live="polite">
        {{ statusLine }}
      </div>
    </section>
  </Teleport>
</template>

<style scoped>
.dataPanelVueRoot {
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

.paneProminent {
  background: linear-gradient(
    135deg,
    color-mix(in oklab, var(--accent-soft) 56%, var(--panel-footer)),
    color-mix(in oklab, var(--panel-footer) 92%, var(--surface))
  );
}

.inputWithActions {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.45rem;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.actions .primary {
  border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
}

.rowInline {
  margin-top: 0.55rem;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.55rem;
}

.inlineCheck {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}

.stats {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem 0.8rem;
}

.gridTwo {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.6rem 0.8rem;
}

.k {
  display: block;
  font-size: 0.68rem;
  letter-spacing: 0.03em;
  text-transform: uppercase;
  color: var(--muted);
}

.v {
  display: block;
  margin-top: 0.1rem;
  font-size: 0.78rem;
  line-height: 1.15rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.recentWrap {
  margin-top: 0.55rem;
}

.chips {
  margin-top: 0.4rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.chip {
  max-width: 280px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.badges {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
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

.badge {
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 0.15rem 0.45rem;
  font-size: 0.74rem;
  background: var(--surface2);
}

.subtlePane {
  margin-top: 0.7rem;
  background: color-mix(in oklab, var(--surface2) 82%, white 18%);
}

.stackList {
  display: grid;
  gap: 0.45rem;
  margin-top: 0.55rem;
  margin-bottom: 0.55rem;
}

.listCard {
  width: 100%;
  display: grid;
  justify-items: start;
  gap: 0.18rem;
  text-align: left;
  border: 1px solid color-mix(in oklab, var(--border) 85%, transparent);
  border-radius: 0.8rem;
  padding: 0.65rem 0.8rem;
  background: color-mix(in oklab, var(--surface) 88%, white 12%);
}

.listCard[data-selected="1"] {
  border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
  box-shadow: inset 0 0 0 1px
    color-mix(in oklab, var(--accent) 24%, transparent);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .inputWithActions {
    grid-template-columns: 1fr;
  }

  .stats {
    grid-template-columns: 1fr;
  }

  .gridTwo {
    grid-template-columns: 1fr;
  }

  .panelSectionHeader {
    flex-direction: column;
  }
}
</style>

<style>
[data-vue-panel="data"][data-vue-data="1"] > :not(#dataVueShellMount) {
  display: none !important;
}
</style>
