<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useHistoryStore } from '../stores/historyStore'
import { useWorkflowStore } from '../stores/workflowStore'

const historyStore = useHistoryStore()
const workflowStore = useWorkflowStore()
const {
  sessionOptions,
  sessionSelectedValue,
  savedOptions,
  savedSelectedValue,
  savedRunsSearch,
  savedRunsTagsDraft,
  savedRunCount,
  savedRunsMeta,
  savedRunDetailTitle,
  savedRunDetailMeta,
  savedRunDetailStrategy,
  savedRunDetailDataset,
  savedRunDetailTimeframe,
  savedRunDetailValidation,
  savedRunDetailReport,
  savedRunDetailSize,
  savedRunDetailTags,
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
} = storeToRefs(historyStore)
const { workspace, workflow } = storeToRefs(workflowStore)

const mountReady = ref(false)
const sessionValue = ref('')
const savedValue = ref('')
const searchDraft = ref('')
const tagsDraft = ref('')
const panelRoot = ref<HTMLElement | null>(null)

const visible = computed(() => {
  const ws = String(workspace.value || '').trim()
  const wf = String(workflow.value || '').trim()
  return ws === 'library' || wf === 'history'
})

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

watch(
  sessionSelectedValue,
  (value) => {
    const next = String(value || '').trim()
    if (sessionValue.value !== next) sessionValue.value = next
  },
  { immediate: true },
)

watch(
  savedSelectedValue,
  (value) => {
    const next = String(value || '').trim()
    if (savedValue.value !== next) savedValue.value = next
  },
  { immediate: true },
)

watch(
  savedRunsSearch,
  (value) => {
    const next = String(value || '')
    if (searchDraft.value !== next) searchDraft.value = next
  },
  { immediate: true },
)

watch(
  savedRunsTagsDraft,
  (value) => {
    const next = String(value || '')
    if (tagsDraft.value !== next) tagsDraft.value = next
  },
  { immediate: true },
)

function setInputValue(id: string, nextValue: string): void {
  historyStore.setInputField(id, nextValue)
}

function setSelectValue(id: string, nextValue: string): void {
  historyStore.setSelectField(id, nextValue)
}

function saveSavedRunTags(): void {
  historyStore.saveSavedRunTags(String(savedValue.value || '').trim(), String(tagsDraft.value || ''))
}

function loadSavedRun(): void {
  historyStore.loadSavedRun(String(savedValue.value || '').trim())
}

function applySavedRun(): void {
  historyStore.applySavedRun(String(savedValue.value || '').trim())
}

function rerunSavedRun(): void {
  historyStore.rerunSavedRun(String(savedValue.value || '').trim())
}

function openSavedRunReport(): void {
  historyStore.openSavedRunReport(String(savedValue.value || '').trim())
}

function revealSavedRun(): void {
  historyStore.revealSavedRun(String(savedValue.value || '').trim())
}

function deleteSavedRun(): void {
  historyStore.deleteSavedRun(String(savedValue.value || '').trim())
}

function onSessionSelect(value: string): void {
  sessionValue.value = value
  setSelectValue('historySelect', value)
}

function onSavedSelect(value: string): void {
  savedValue.value = value
  setSelectValue('savedRunsSelect', value)
}

function onSavedSearch(value: string): void {
  searchDraft.value = value
  setInputValue('savedRunsSearch', value)
}

function onSavedTags(value: string): void {
  tagsDraft.value = value
  setInputValue('savedRunsTags', value)
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="history"]') as HTMLElement | null
  const mount = document.getElementById('historyVueShellMount')
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueHistory = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
})

onBeforeUnmount(() => {
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueHistory
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#historyVueShellMount">
    <section class="historyPanelVueRoot historyVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Run History</div>
        <div class="meta mono">{{ savedRunCount }} saved · {{ stamp }}</div>
      </header>

      <div class="grid">
        <section class="pane">
          <h3>Session History</h3>
          <label for="vueHistorySelect">Run history</label>
          <select id="vueHistorySelect" :value="sessionValue" @change="onSessionSelect(String(($event.target as HTMLSelectElement).value || ''))">
            <option
              v-for="option in sessionOptions"
              :key="`session_${option.value}_${option.label}`"
              :value="option.value"
              :disabled="option.disabled"
            >
              {{ option.label }}
            </option>
          </select>

          <div class="actions">
            <button type="button" :disabled="!canApplyHistory" @click="historyStore.applySession()">Apply</button>
            <button type="button" :disabled="!canExportPreset" @click="historyStore.exportPreset()">Export preset</button>
            <button type="button" :disabled="!canImportPreset" @click="historyStore.importPreset()">Import preset</button>
            <button type="button" :disabled="!canClearHistory" @click="historyStore.clearSession()">Clear</button>
          </div>
        </section>

        <section class="pane">
          <h3>Saved Runs</h3>
          <div class="row">
            <div>
              <label for="vueSavedRunsSearch">Search</label>
              <input
                id="vueSavedRunsSearch"
                type="text"
                :value="searchDraft"
                placeholder="ts, strategy, tag, csv..."
                @input="onSavedSearch(String(($event.target as HTMLInputElement).value || ''))"
              />
            </div>
            <div>
              <label for="vueSavedRunsTags">Tags</label>
              <div class="inputWithAction">
                <input
                  id="vueSavedRunsTags"
                  type="text"
                  :value="tagsDraft"
                  placeholder="aapl,1m,robust"
                  @input="onSavedTags(String(($event.target as HTMLInputElement).value || ''))"
                />
                <button type="button" :disabled="!canSaveSavedRunTags" @click="saveSavedRunTags">Save</button>
              </div>
            </div>
          </div>

          <label for="vueSavedRunsSelect">Saved runs</label>
          <select id="vueSavedRunsSelect" :value="savedValue" @change="onSavedSelect(String(($event.target as HTMLSelectElement).value || ''))">
            <option
              v-for="option in savedOptions"
              :key="`saved_${option.value}_${option.label}`"
              :value="option.value"
              :disabled="option.disabled"
            >
              {{ option.label }}
            </option>
          </select>

          <div class="actions">
            <button type="button" :disabled="!canRefreshSavedRuns" @click="historyStore.refreshSavedRuns()">Refresh</button>
            <button type="button" :disabled="!canLoadSavedRun" @click="loadSavedRun">Load</button>
            <button type="button" :disabled="!canApplySavedRun" @click="applySavedRun">Apply</button>
            <button type="button" :disabled="!canRerunSavedRun" @click="rerunSavedRun">Rerun</button>
            <button type="button" :disabled="!canOpenSavedRunReport" @click="openSavedRunReport">Open report</button>
            <button type="button" :disabled="!canRevealSavedRun" @click="revealSavedRun">Reveal</button>
            <button type="button" :disabled="!canDeleteSavedRun" @click="deleteSavedRun">Delete</button>
            <button type="button" :disabled="!canOpenRunsDir" @click="historyStore.openRunsDir()">Runs folder</button>
          </div>

          <div class="muted metaLine mono">{{ savedRunsMeta }}</div>
        </section>
      </div>

      <section class="detailCard">
        <div class="detailTitle">{{ savedRunDetailTitle }}</div>
        <div class="detailMeta muted">{{ savedRunDetailMeta }}</div>
        <div class="detailGrid">
          <div><span class="k">Strategy</span><span class="v mono">{{ savedRunDetailStrategy }}</span></div>
          <div><span class="k">Dataset</span><span class="v mono">{{ savedRunDetailDataset }}</span></div>
          <div><span class="k">Timeframe</span><span class="v mono">{{ savedRunDetailTimeframe }}</span></div>
          <div><span class="k">Validation</span><span class="v mono">{{ savedRunDetailValidation }}</span></div>
          <div><span class="k">Report</span><span class="v mono">{{ savedRunDetailReport }}</span></div>
          <div><span class="k">Size</span><span class="v mono">{{ savedRunDetailSize }}</span></div>
          <div class="full"><span class="k">Tags</span><span class="v mono">{{ savedRunDetailTags }}</span></div>
        </div>
        <div class="k">Request snapshot</div>
        <pre class="mono">{{ savedRunDetailParams }}</pre>
      </section>

      <div class="status mono">{{ statusLine }}</div>
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

.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.8rem;
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

.row {
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

.metaLine {
  margin-top: 0.55rem;
  font-size: 0.78rem;
}

.detailCard {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface2);
  padding: 0.75rem 0.8rem;
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

.status {
  font-size: 0.78rem;
  color: var(--muted);
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

  .detailGrid {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='history'][data-vue-history='1'] > :not(#historyVueShellMount) {
  display: none !important;
}
</style>


