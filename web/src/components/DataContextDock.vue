<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useDataStore } from '../stores/dataStore'
import { useCommandActionStore } from '../stores/commandActionStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const commandActionStore = useCommandActionStore()
const store = useDataStore()
const { dataId, datasetLabel, datasetRange, dataLen, statusLine, updatedAtUnixMs } = storeToRefs(store)

const isLoaded = computed(() => dataId.value !== '—' && dataId.value !== '')

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

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
      <h2 class="title" style="margin: 0;">Data Context</h2>
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
  --dock-accent: var(--accent);
  --dock-accent-ink: var(--selection-text);
  --dock-accent-soft: var(--accent-soft);
  --dock-surface: linear-gradient(
    160deg,
    color-mix(in oklab, var(--surface-focus) 90%, var(--panel-footer)),
    color-mix(in oklab, var(--accent-soft) 34%, var(--surface-sticky))
  );
  --dock-border: color-mix(in oklab, var(--dock-accent) 40%, var(--border));
  --dock-field-border: color-mix(in oklab, var(--dock-accent) 34%, var(--border));
  --dock-field-surface: color-mix(in oklab, var(--dock-accent-soft) 48%, var(--surface-inline));
  --dock-text: var(--text);
  --dock-title: color-mix(in oklab, var(--dock-accent) 76%, var(--dock-text));
  --dock-stamp: color-mix(in oklab, var(--dock-accent-ink) 74%, var(--dock-text));
  --dock-label: color-mix(in oklab, var(--dock-accent) 64%, var(--dock-text));
  --dock-action-text: color-mix(in oklab, var(--dock-accent-ink) 82%, var(--dock-text));
  position: fixed;
  left: 10px;
  bottom: 10px;
  z-index: 4040;
  width: min(360px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid var(--dock-border);
  background: var(--dock-surface);
  color: var(--dock-text);
  box-shadow: var(--shadow-focus);
  backdrop-filter: blur(12px);
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
  color: var(--dock-title);
}

.stamp {
  font-size: 0.72rem;
  color: var(--dock-stamp);
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
  color: var(--dock-label);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.row strong {
  font-size: 0.78rem;
  color: var(--dock-text);
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
  border: 1px solid var(--dock-field-border);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: var(--dock-field-surface);
  color: var(--dock-action-text);
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


