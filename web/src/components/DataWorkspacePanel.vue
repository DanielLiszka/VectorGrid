<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
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
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(dataStore)
const { controlsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

const csvPathDraft = ref('')
const marketDataDirDraft = ref('')
const timeframeDraft = ref('')

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

watch(
  csvPath,
  (value) => {
    const next = String(value || '')
    if (csvPathDraft.value !== next) csvPathDraft.value = next
  },
  { immediate: true },
)

watch(
  marketDataDir,
  (value) => {
    const next = String(value || '')
    if (marketDataDirDraft.value !== next) marketDataDirDraft.value = next
  },
  { immediate: true },
)

watch(
  dataTimeframe,
  (value) => {
    const next = String(value || '')
    if (timeframeDraft.value !== next) timeframeDraft.value = next
  },
  { immediate: true },
)

function setInputValue(id: string, nextValue: string): void {
  dataStore.setInputField(id, nextValue)
}

function setSelectValue(id: string, nextValue: string): void {
  dataStore.setSelectField(id, nextValue)
}

function browseCsv(): void {
  dataStore.browseCsv()
}

function openPolygonModal(): void {
  dataStore.openPolygon()
}

function loadCsvFromDraft(): void {
  dataStore.loadCsv(String(csvPathDraft.value || '').trim())
}

function applyTimeframeFromDraft(): void {
  dataStore.applyTimeframe(String(timeframeDraft.value || '').trim())
}

function setMarketDataDirFromDraft(): void {
  dataStore.setMarketDataDir(String(marketDataDirDraft.value || '').trim(), !!marketDataMoveExisting.value)
}

function setCheckboxValue(id: string, checked: boolean): void {
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

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="data"]') as HTMLElement | null
  const mount = document.getElementById('dataVueShellMount')
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueData = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
})

onBeforeUnmount(() => {
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
        <div class="title">Data Source</div>
        <div class="meta mono">{{ rowLabel }} rows · {{ stamp }}</div>
      </header>

      <section class="pane">
        <h3>CSV input</h3>
        <label for="vueCsvPath">CSV path</label>
        <div class="inputWithActions">
          <input
            id="vueCsvPath"
            type="text"
            :value="csvPathDraft"
            placeholder="C:\\path\\to\\candles.csv"
            @input="csvPathDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('csvPath', csvPathDraft)"
          />
          <div class="actions">
            <button type="button" :disabled="!canBrowseCsv" @click="browseCsv">Browse</button>
            <button type="button" :disabled="!canPolygonOpen" @click="openPolygonModal">Polygon</button>
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
        <h3>Sample datasets</h3>
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

      <section class="pane">
        <h3>Storage and cache</h3>
        <label for="vueMarketDataDir">Market data folder</label>
        <div class="inputWithActions">
          <input
            id="vueMarketDataDir"
            type="text"
            :value="marketDataDirDraft"
            placeholder="Default (AppData)"
            @input="marketDataDirDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('marketDataDir', marketDataDirDraft)"
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
        <h3>Dataset context</h3>
        <div class="stats">
          <div><span class="k">Loaded ID</span><span class="v mono">{{ dataId }}</span></div>
          <div><span class="k">Rows</span><span class="v mono">{{ dataLen }}</span></div>
          <div><span class="k">Dataset</span><span class="v mono">{{ datasetLabel }}</span></div>
          <div><span class="k">Range</span><span class="v mono">{{ datasetRange }}</span></div>
        </div>
      </section>

      <section class="pane">
        <h3>Timeframe</h3>
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
        <h3>Columns</h3>
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
  align-items: center;
  gap: 0.75rem;
}

.title {
  font-size: 0.95rem;
  font-weight: 650;
}

.meta {
  font-size: 0.78rem;
  color: var(--muted);
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.pane h3 {
  margin: 0 0 0.55rem;
  font-size: 0.86rem;
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
}
</style>

<style>
[data-vue-panel='data'][data-vue-data='1'] > :not(#dataVueShellMount) {
  display: none !important;
}
</style>


