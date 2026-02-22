<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useDistributedStore } from '../stores/distributedStore'
import { useWorkflowStore } from '../stores/workflowStore'

const distributedStore = useDistributedStore()
const workflowState = useWorkflowStore()
const {
  leaderMode,
  leaderModeHint,
  workerStatus,
  workerBindAddr,
  workersCount,
  assetPathCount,
  tileCsvPath,
  tilesPerWorker,
  outputPreview,
  statusLine,
  isRunning,
  canCancel,
  updatedAtUnixMs,
} = storeToRefs(distributedStore)
const { controlsTab, workspace, workflow } = storeToRefs(workflowState)

const visible = computed(() => {
  const ws = String(workspace.value || '').trim()
  const wf = String(workflow.value || '').trim()
  const activeControlsTab = String(controlsTab.value || '').trim() || 'data'
  return ws === 'workflow' && wf === 'optimize' && (activeControlsTab === 'distributed' || isRunning.value)
})

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

const tileCsvLabel = computed(() => tileCsvPath.value || '(not set)')

function setLeaderMode(next: string): void {
  const value = String(next || 'assets').trim() || 'assets'
  distributedStore.setSelectField('distLeaderMode', value)
}
</script>

<template>
  <aside v-if="visible" class="distDock">
    <header class="dockHeader">
      <div class="title">Distributed Context</div>
      <div class="stamp">{{ stamp }}</div>
    </header>
    <div class="rows">
      <div class="row">
        <span>Mode</span>
        <select :value="leaderMode" @change="setLeaderMode(String(($event.target as HTMLSelectElement).value || 'assets'))">
          <option value="assets">assets</option>
          <option value="tiles">tiles</option>
        </select>
      </div>
      <div class="row"><span>Worker</span><strong class="mono">{{ workerStatus }}</strong></div>
      <div class="row"><span>Bind</span><strong class="mono">{{ workerBindAddr || '(default)' }}</strong></div>
      <div class="row"><span>Workers</span><strong class="mono">{{ workersCount }}</strong></div>
      <div class="row"><span>Assets</span><strong class="mono">{{ assetPathCount }}</strong></div>
      <div class="row"><span>Tile CSV</span><strong class="mono">{{ tileCsvLabel }}</strong></div>
      <div class="row"><span>Tiles/W</span><strong class="mono">{{ tilesPerWorker }}</strong></div>
      <div class="row"><span>Hint</span><strong class="mono">{{ leaderModeHint }}</strong></div>
      <div class="row"><span>Status</span><strong class="mono">{{ statusLine }}</strong></div>
      <div class="row"><span>Output</span><strong class="mono">{{ outputPreview }}</strong></div>
    </div>
    <div class="actions">
      <button type="button" @click="distributedStore.generateToken()">Gen token</button>
      <button type="button" @click="distributedStore.startWorker()">Start worker</button>
      <button type="button" @click="distributedStore.stopWorker()">Stop worker</button>
      <button type="button" @click="distributedStore.refreshWorkerStatus()">Worker status</button>
      <button type="button" @click="distributedStore.fillFromPolygon()">Fill assets</button>
      <button type="button" @click="distributedStore.useCurrentCsv()">Use current csv</button>
      <button type="button" @click="distributedStore.runAssets()">Run assets</button>
      <button type="button" @click="distributedStore.runTiles()">Run tiles</button>
      <button type="button" :disabled="!canCancel" @click="distributedStore.cancel()">Cancel run</button>
    </div>
  </aside>
</template>

<style scoped>
.distDock {
  position: fixed;
  left: 10px;
  bottom: 412px;
  z-index: 4036;
  width: min(440px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid rgba(244, 114, 182, 0.42);
  background: linear-gradient(158deg, rgba(15, 23, 42, 0.94), rgba(80, 7, 36, 0.9));
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
  color: #f9a8d4;
}

.stamp {
  font-size: 0.72rem;
  color: #fbcfe8;
}

.rows {
  display: grid;
  gap: 4px;
}

.row {
  display: grid;
  grid-template-columns: 74px 1fr;
  gap: 6px;
  align-items: baseline;
}

.row span {
  font-size: 0.72rem;
  color: #f472b6;
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

.row select {
  width: 100%;
  border: 1px solid rgba(244, 114, 182, 0.38);
  border-radius: 8px;
  padding: 3px 6px;
  font-size: 0.75rem;
  background: rgba(80, 7, 36, 0.82);
  color: #fce7f3;
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.actions button {
  border: 1px solid rgba(244, 114, 182, 0.4);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: rgba(80, 7, 36, 0.82);
  color: #fce7f3;
}

.actions button:disabled {
  opacity: 0.55;
}

@media (max-width: 760px) {
  .distDock {
    left: 8px;
    right: 8px;
    width: auto;
    bottom: 8px;
  }
}
</style>
