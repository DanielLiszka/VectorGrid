<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useDataStore } from '../stores/dataStore'
import { useWorkflowStore } from '../stores/workflowStore'

const dataStore = useDataStore()
const workflowStore = useWorkflowStore()
const {
  csvPath,
  recentCsvPaths,
  sampleCsvOptions,
  sampleCsvVisible,
  marketDataDir,
  marketDataMoveExisting,
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
  canBrowseMarketDataDir,
  canSetMarketDataDir,
  canResetMarketDataDir,
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
  updatedAtUnixMs,
} = storeToRefs(dataStore)
const { controlsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)
const polygonSymbolInput = ref<HTMLInputElement | null>(null)
const polygonApiKeyInput = ref<HTMLInputElement | null>(null)

const csvPathDraft = ref('')
const marketDataDirDraft = ref('')
const timeframeDraft = ref('')
const polygonApiKeyDraft = ref('')
const polygonMarketDraft = ref('stocks')
const polygonSymbolDraft = ref('')
const polygonFromDraft = ref('')
const polygonToDraft = ref('')
const polygonTimeframeDraft = ref('1m')
const polygonSelectedPathDraft = ref('')

const visible = computed(() => String(controlsTab.value || '').trim() === 'data')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

const rowLabel = computed(() => {
  const rows = String(dataLen.value || '').trim()
  return rows || '—'
})
const dataSummaryChips = computed(() => {
  const chips: string[] = []
  const dataset = String(datasetLabel.value || '').trim()
  const timeframe = String(dataTimeframe.value || '').trim()
  const loadedId = String(dataId.value || '').trim()
  const fieldCount = dataFields.value.length
  if (dataset) chips.push(`Dataset: ${dataset}`)
  chips.push(`Rows: ${rowLabel.value}`)
  if (timeframe) chips.push(`Timeframe: ${timeframe}`)
  if (loadedId) chips.push(`Loaded ID: ${loadedId}`)
  if (fieldCount > 0) chips.push(`Fields: ${fieldCount}`)
  return chips
})

watch(
  () => [
    String(csvPath.value || ''),
    String(marketDataDir.value || ''),
    String(dataTimeframe.value || ''),
    String(polygonApiKey.value || ''),
    String(polygonMarket.value || 'stocks'),
    String(polygonSymbol.value || ''),
    String(polygonFrom.value || ''),
    String(polygonTo.value || ''),
    String(polygonTimeframe.value || '1m'),
    String(polygonSelectedPath.value || ''),
  ] as const,
  ([nextCsvPath, nextMarketDataDir, nextTimeframe, nextPolygonApiKey, nextPolygonMarket, nextPolygonSymbol, nextPolygonFrom, nextPolygonTo, nextPolygonTimeframe, nextPolygonSelectedPath]) => {
    if (csvPathDraft.value !== nextCsvPath) csvPathDraft.value = nextCsvPath
    if (marketDataDirDraft.value !== nextMarketDataDir) marketDataDirDraft.value = nextMarketDataDir
    if (timeframeDraft.value !== nextTimeframe) timeframeDraft.value = nextTimeframe
    if (polygonApiKeyDraft.value !== nextPolygonApiKey) polygonApiKeyDraft.value = nextPolygonApiKey
    if (polygonMarketDraft.value !== nextPolygonMarket) polygonMarketDraft.value = nextPolygonMarket
    if (polygonSymbolDraft.value !== nextPolygonSymbol) polygonSymbolDraft.value = nextPolygonSymbol
    if (polygonFromDraft.value !== nextPolygonFrom) polygonFromDraft.value = nextPolygonFrom
    if (polygonToDraft.value !== nextPolygonTo) polygonToDraft.value = nextPolygonTo
    if (polygonTimeframeDraft.value !== nextPolygonTimeframe) polygonTimeframeDraft.value = nextPolygonTimeframe
    if (polygonSelectedPathDraft.value !== nextPolygonSelectedPath) polygonSelectedPathDraft.value = nextPolygonSelectedPath
  },
  { immediate: true },
)

watch(
  () => Boolean(polygonOpen.value),
  async (open) => {
    if (!open) return
    await nextTick()
    const focusTarget = polygonSymbolInput.value ?? polygonApiKeyInput.value
    focusTarget?.focus()
  },
)

function setInputValue(id: string, nextValue: string): void {
  dataStore.setInputField(id, nextValue)
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } = useQueuedInputWriter(setInputValue)

function setSelectValue(id: string, nextValue: string): void {
  dataStore.setSelectField(id, nextValue)
}

function browseCsv(): void {
  dataStore.browseCsv()
}

function togglePolygonSection(): void {
  if (polygonOpen.value) {
    dataStore.closePolygon()
    return
  }
  dataStore.openPolygon()
}

function loadCsvFromDraft(): void {
  flushQueuedInputValuesNow()
  dataStore.loadCsv(String(csvPathDraft.value || '').trim())
}

function applyTimeframeFromDraft(): void {
  dataStore.applyTimeframe(String(timeframeDraft.value || '').trim())
}

function setMarketDataDirFromDraft(): void {
  flushQueuedInputValuesNow()
  dataStore.setMarketDataDir(String(marketDataDirDraft.value || '').trim(), !!marketDataMoveExisting.value)
}

function setCheckboxValue(id: string, checked: boolean): void {
  dataStore.setCheckboxField(id, checked)
}

function setPolygonCheckboxValue(id: string, checked: boolean): void {
  dataStore.setCheckboxField(id, checked)
}

function applyRecentPath(path: string): void {
  const nextValue = String(path || '').trim()
  if (!nextValue) return
  csvPathDraft.value = nextValue
  setInputValue('csvPath', nextValue)
}

function loadSampleCsv(path: string, index: number): void {
  dataStore.loadSampleCsv(index, path)
}

function selectPolygonDataset(path: string): void {
  polygonSelectedPathDraft.value = String(path || '').trim()
  setInputValue('polygonSelectedPath', polygonSelectedPathDraft.value)
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="data"]') as HTMLElement | null
  const mount = document.querySelector('#dataVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueData = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  dataStore.hydrateStartupState()
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueData
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#dataVueShellMount">
    <section class="dataPanelVueRoot dataVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="headCopy">
          <div class="title">Data Source</div>
          <div class="subtitle">Load a dataset, choose storage behavior, and confirm the working timeframe before you optimize.</div>
        </div>
        <div class="meta mono">{{ rowLabel }} rows · {{ stamp }}</div>
      </header>
      <div class="summaryChips" aria-label="Dataset summary">
        <span v-for="chip in dataSummaryChips" :key="`data_summary_${chip}`" class="summaryChip mono">{{ chip }}</span>
      </div>

      <section class="pane paneProminent">
        <div class="paneHeader">
          <div>
            <div class="sectionEyebrow">Import</div>
            <h3>CSV input</h3>
          </div>
        </div>
        <label for="vueCsvPath">CSV path</label>
        <div class="inputWithActions">
          <input
            id="vueCsvPath"
            type="text"
            :value="csvPathDraft"
            placeholder="C:\\path\\to\\candles.csv"
            @input="csvPathDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('csvPath', csvPathDraft)"
            @change="commitInputValue('csvPath', csvPathDraft)"
          />
          <div class="actions">
            <button type="button" :disabled="!canBrowseCsv" @click="browseCsv">Browse</button>
            <button type="button" :disabled="!canPolygonOpen" @click="togglePolygonSection">{{ polygonOpen ? 'Hide Polygon' : 'Polygon' }}</button>
            <button type="button" class="primary" :disabled="!canLoadCsv" @click="loadCsvFromDraft">Load CSV</button>
            <button type="button" :disabled="!canClearData" @click="dataStore.clearData()">Clear</button>
          </div>
        </div>
        <div class="recentWrap" v-if="recentCsvPaths.length">
          <div class="k">Recent</div>
          <div class="chips">
            <button
              v-for="path in recentCsvPaths"
              :key="`recent_${path}`"
              type="button"
              class="chip mono"
              @click="applyRecentPath(path)"
            >
              {{ path }}
            </button>
          </div>
        </div>
      </section>

      <section class="pane" v-if="sampleCsvVisible && sampleCsvOptions.length">
        <div class="paneHeader">
          <div>
            <div class="sectionEyebrow">Quick Start</div>
            <h3>Sample datasets</h3>
          </div>
        </div>
        <div class="actions">
          <button
            v-for="(sample, index) in sampleCsvOptions"
            :key="`sample_${index}_${sample.label}`"
            type="button"
            :disabled="sample.disabled"
            @click="loadSampleCsv(sample.path, index)"
          >
            {{ sample.label }}
          </button>
        </div>
      </section>

      <section class="pane paneProminent" v-if="polygonOpen">
        <div class="paneHeader">
          <div>
            <div class="sectionEyebrow">Market Download</div>
            <h3>Polygon</h3>
          </div>
          <div class="actions">
            <button type="button" :disabled="!canPolygonRefresh" @click="dataStore.refreshPolygon()">Refresh cache</button>
            <button type="button" @click="dataStore.closePolygon()">Close</button>
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
                @input="polygonApiKeyDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('polygonApiKey', polygonApiKeyDraft)"
                @change="commitInputValue('polygonApiKey', polygonApiKeyDraft)"
              />
              <div class="actions">
                <button type="button" :disabled="!canPolygonSaveApiKey" @click="dataStore.savePolygonApiKey()">Save key</button>
                <button type="button" :disabled="!canPolygonClearApiKey" @click="dataStore.clearPolygonApiKey()">Clear key</button>
              </div>
            </div>
            <div class="muted mono">{{ polygonHasApiKey ? 'API key saved in local Polygon cache settings.' : 'Save an API key to enable download and live Polygon workflows.' }}</div>
          </div>

          <div>
            <label for="vuePolygonCacheDir">Cache folder</label>
            <input id="vuePolygonCacheDir" type="text" :value="polygonCacheDir" readonly />
            <div class="muted mono">Downloaded 1m data is cached here and reused automatically.</div>
          </div>
        </div>

        <div class="gridTwo">
          <div>
            <label for="vuePolygonMarket">Market</label>
            <select
              id="vuePolygonMarket"
              :value="polygonMarketDraft"
              @change="polygonMarketDraft = String(($event.target as HTMLSelectElement).value || 'stocks'); setSelectValue('polygonMarket', polygonMarketDraft)"
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
              @change="polygonTimeframeDraft = String(($event.target as HTMLSelectElement).value || '1m'); setSelectValue('polygonTimeframe', polygonTimeframeDraft)"
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
              @input="polygonSymbolDraft = String(($event.target as HTMLInputElement).value || '').toUpperCase(); queueInputValue('polygonSymbol', polygonSymbolDraft)"
              @change="commitInputValue('polygonSymbol', polygonSymbolDraft)"
            />
          </div>
          <div class="rowInline">
            <label class="inlineCheck">
              <input
                type="checkbox"
                :checked="polygonAdjusted"
                @change="setPolygonCheckboxValue('polygonAdjusted', ($event.target as HTMLInputElement).checked)"
              />
              Adjusted prices
            </label>
          </div>
        </div>

        <div class="gridTwo">
          <div>
            <label for="vuePolygonFrom">From</label>
            <input
              id="vuePolygonFrom"
              type="date"
              :value="polygonFromDraft"
              @input="polygonFromDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('polygonFrom', polygonFromDraft)"
              @change="commitInputValue('polygonFrom', polygonFromDraft)"
            />
          </div>
          <div>
            <label for="vuePolygonTo">To</label>
            <input
              id="vuePolygonTo"
              type="date"
              :value="polygonToDraft"
              @input="polygonToDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('polygonTo', polygonToDraft)"
              @change="commitInputValue('polygonTo', polygonToDraft)"
            />
          </div>
        </div>

        <div class="actions">
          <button type="button" class="primary" :disabled="!canPolygonDownload" @click="dataStore.downloadPolygonDataset()">Download and load</button>
        </div>

        <div class="pane subtlePane">
          <div class="paneHeader">
            <div>
              <div class="sectionEyebrow">Cache</div>
              <h3>Cached datasets</h3>
            </div>
          </div>
          <div v-if="polygonDatasets.length" class="stackList">
            <button
              v-for="dataset in polygonDatasets"
              :key="`polygon_dataset_${dataset.csvPath}`"
              type="button"
              class="listCard"
              :data-selected="polygonSelectedPath === dataset.csvPath ? '1' : '0'"
              @click="selectPolygonDataset(dataset.csvPath)"
            >
              <span class="mono">{{ dataset.displaySymbol || dataset.polygonTicker || dataset.csvPath }}</span>
              <span class="muted mono">{{ dataset.timeframe }} · {{ dataset.rowCount }} rows</span>
            </button>
          </div>
          <div v-else class="muted">No cached Polygon datasets yet. Save a key, choose a symbol/date range, then download.</div>
          <div class="muted mono">{{ polygonSelectedMeta }}</div>
          <div class="actions">
            <button type="button" :disabled="!canPolygonLoadSelected" @click="dataStore.loadPolygonDataset(polygonSelectedPathDraft)">Load selected</button>
            <button type="button" :disabled="!canPolygonValidateSelected" @click="dataStore.validatePolygonDataset(polygonSelectedPathDraft)">Validate selected</button>
            <button type="button" :disabled="!canPolygonRepairSelected" @click="dataStore.repairPolygonDataset(polygonSelectedPathDraft)">Repair selected</button>
          </div>
        </div>
      </section>

      <section class="pane">
        <div class="paneHeader">
          <div>
            <div class="sectionEyebrow">Storage</div>
            <h3>Storage and cache</h3>
          </div>
        </div>
        <label for="vueMarketDataDir">Market data folder</label>
        <div class="inputWithActions">
          <input
            id="vueMarketDataDir"
            type="text"
            :value="marketDataDirDraft"
            placeholder="Default (AppData)"
            @input="marketDataDirDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('marketDataDir', marketDataDirDraft)"
            @change="commitInputValue('marketDataDir', marketDataDirDraft)"
          />
          <div class="actions">
            <button type="button" :disabled="!canBrowseMarketDataDir" @click="dataStore.browseMarketDataDir()">Browse</button>
            <button type="button" :disabled="!canSetMarketDataDir" @click="setMarketDataDirFromDraft">Set</button>
          </div>
        </div>
        <div class="rowInline">
          <label class="inlineCheck">
            <input
              type="checkbox"
              :checked="marketDataMoveExisting"
              @change="setCheckboxValue('marketDataMoveExisting', ($event.target as HTMLInputElement).checked)"
            />
            Move existing Polygon cache
          </label>
          <div class="actions">
            <button type="button" :disabled="!canResetMarketDataDir" @click="dataStore.resetMarketDataDir()">Reset</button>
            <button type="button" :disabled="!canOpenMarketDataDir" @click="dataStore.openMarketDataDir()">Open</button>
          </div>
        </div>
        <div class="muted mono">{{ marketDataDirHint }}</div>
      </section>

      <section class="pane">
        <div class="paneHeader">
          <div>
            <div class="sectionEyebrow">Context</div>
            <h3>Dataset context</h3>
          </div>
        </div>
        <div class="stats">
          <div><span class="k">Loaded ID</span><span class="v mono">{{ dataId }}</span></div>
          <div><span class="k">Rows</span><span class="v mono">{{ dataLen }}</span></div>
          <div><span class="k">Dataset</span><span class="v mono">{{ datasetLabel }}</span></div>
          <div><span class="k">Range</span><span class="v mono">{{ datasetRange }}</span></div>
        </div>
      </section>

      <section class="pane">
        <div class="paneHeader">
          <div>
            <div class="sectionEyebrow">Resolution</div>
            <h3>Timeframe</h3>
          </div>
        </div>
        <div class="inputWithActions">
          <select
            id="vueDataTimeframe"
            :value="timeframeDraft"
            @change="timeframeDraft = String(($event.target as HTMLSelectElement).value || ''); setSelectValue('dataTimeframe', timeframeDraft)"
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
            <button type="button" :disabled="!canApplyTimeframe" @click="applyTimeframeFromDraft">Apply timeframe</button>
          </div>
        </div>
        <label class="inlineCheck">
          <input
            type="checkbox"
            :checked="autoRestoreDataset"
            @change="setCheckboxValue('autoRestoreDataset', ($event.target as HTMLInputElement).checked)"
          />
          Restore last dataset and timeframe on startup
        </label>
        <div class="muted mono">{{ dataTimeframeHint }}</div>
        <div class="muted mono">{{ datasetContext }}</div>
      </section>

      <section class="pane">
        <div class="paneHeader">
          <div>
            <div class="sectionEyebrow">Schema</div>
            <h3>Columns</h3>
          </div>
        </div>
        <div class="badges">
          <span class="badge mono" v-for="field in dataFields" :key="`field_${field}`">{{ field }}</span>
          <span v-if="!dataFields.length" class="muted">No columns available.</span>
        </div>
        <div class="muted mono">{{ dataFieldHint || '—' }}</div>
      </section>

      <div class="status mono">{{ statusLine }}</div>
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

.headCopy {
  display: grid;
  gap: 0.18rem;
}

.title {
  font-size: 1rem;
  font-weight: 650;
}

.subtitle {
  font-size: 0.82rem;
  line-height: 1.35;
  color: var(--muted);
}

.meta {
  font-size: 0.78rem;
  color: var(--muted);
}

.summaryChips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.42rem;
}

.summaryChip {
  border: 1px solid var(--border);
  border-radius: 999px;
  background: color-mix(in oklab, var(--panel) 86%, var(--surface));
  padding: 0.22rem 0.52rem;
  font-size: 0.72rem;
  color: var(--muted);
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

.paneHeader {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
  margin-bottom: 0.65rem;
}

.sectionEyebrow {
  font-size: 0.69rem;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--muted);
}

.pane h3 {
  margin: 0.12rem 0 0;
  font-size: 0.92rem;
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

.listCard[data-selected='1'] {
  border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
  box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 24%, transparent);
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

  .paneHeader {
    flex-direction: column;
  }
}
</style>

<style>
[data-vue-panel='data'][data-vue-data='1'] > :not(#dataVueShellMount) {
  display: none !important;
}
</style>



