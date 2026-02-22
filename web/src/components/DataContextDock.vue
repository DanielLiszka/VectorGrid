<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useDataStore } from '../stores/dataStore'
import { useCommandActionStore } from '../stores/commandActionStore'

const commandActionStore = useCommandActionStore()
const store = useDataStore()
const { dataId, datasetLabel, datasetRange, dataLen, statusLine, updatedAtUnixMs } = storeToRefs(store)

const isLoaded = computed(() => dataId.value !== '—' && dataId.value !== '')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  const d = new Date(ms)
  return d.toLocaleTimeString([], { hour12: false })
})

function browseCsvAction(): void {
  commandActionStore.browseCsv()
}

function loadDataAction(): void {
  commandActionStore.loadData()
}

function runOptimizationAction(): void {
  commandActionStore.runOptimization()
}
</script>

<template>
  <aside class="dataDock">
    <header class="dockHeader">
      <div class="title">Data Context</div>
      <div class="stamp">{{ stamp }}</div>
    </header>
    <div class="rows">
      <div class="row"><span>ID</span><strong class="mono">{{ dataId }}</strong></div>
      <div class="row"><span>Dataset</span><strong class="mono">{{ datasetLabel }}</strong></div>
      <div class="row"><span>Range</span><strong class="mono">{{ datasetRange }}</strong></div>
      <div class="row"><span>Rows</span><strong class="mono">{{ dataLen }}</strong></div>
      <div class="row"><span>Status</span><strong class="mono">{{ statusLine }}</strong></div>
    </div>
    <div class="actions">
      <button type="button" @click="browseCsvAction">Browse CSV</button>
      <button type="button" @click="loadDataAction">Load</button>
      <button type="button" :disabled="!isLoaded" @click="runOptimizationAction">Run</button>
    </div>
  </aside>
</template>

<style scoped>
.dataDock {
  position: fixed;
  left: 10px;
  bottom: 10px;
  z-index: 4040;
  width: min(360px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid rgba(14, 116, 144, 0.42);
  background: linear-gradient(160deg, rgba(15, 23, 42, 0.94), rgba(2, 44, 34, 0.9));
  color: #e2e8f0;
  box-shadow: 0 18px 40px rgba(2, 6, 23, 0.45);
  padding: 10px;
  display: grid;
  gap: 8px;
  pointer-events: auto;
}

.dockHeader {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #67e8f9;
}

.stamp {
  font-size: 0.72rem;
  color: #93c5fd;
}

.rows {
  display: grid;
  gap: 4px;
}

.row {
  display: grid;
  grid-template-columns: 88px 1fr;
  gap: 6px;
  align-items: baseline;
}

.row span {
  font-size: 0.72rem;
  color: #7dd3fc;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.row strong {
  font-size: 0.78rem;
  color: #e2e8f0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
}

.actions {
  display: flex;
  gap: 6px;
}

.actions button {
  border: 1px solid rgba(56, 189, 248, 0.36);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: rgba(7, 33, 53, 0.82);
  color: #e0f2fe;
}

.actions button:disabled {
  opacity: 0.55;
}

@media (max-width: 760px) {
  .dataDock {
    left: 8px;
    right: 8px;
    width: auto;
    bottom: 8px;
  }
}
</style>


