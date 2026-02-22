<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useAnalysisStore } from '../stores/analysisStore'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useWorkflowStore } from '../stores/workflowStore'

const analysis = useAnalysisStore()
const commandActionStore = useCommandActionStore()
const workflowState = useWorkflowStore()
const {
  bestTitle,
  bestMeta,
  topRows,
  topFilterMeta,
  drillStatus,
  stabilityStatus,
  heatmapMeta,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysis)
const { resultsTab, workspace, workflow } = storeToRefs(workflowState)

const visible = computed(() => {
  const ws = String(workspace.value || '').trim()
  const wf = String(workflow.value || '').trim()
  if (ws === 'aggregate') return true
  return wf === 'optimize' || wf === 'validate' || wf === 'analyze'
})

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

const activeResultsTab = computed(() => String(resultsTab.value || '').trim() || 'overview')

function exportTopCsvAction(): void {
  commandActionStore.exportTopCsv()
}

function runOptimizationAction(): void {
  commandActionStore.runOptimization()
}

function runValidationAction(): void {
  commandActionStore.validateOptimization()
}

function clickResultsTab(tab: string): void {
  workflowState.selectResultsTab(tab)
}
</script>

<template>
  <aside v-if="visible" class="analysisDock">
    <header class="dockHeader">
      <div class="title">Analysis Context</div>
      <div class="stamp">{{ stamp }}</div>
    </header>
    <div class="rows">
      <div class="row"><span>Tab</span><strong class="mono">{{ activeResultsTab }}</strong></div>
      <div class="row"><span>Best</span><strong class="mono">{{ bestTitle }}</strong></div>
      <div class="row"><span>Meta</span><strong class="mono">{{ bestMeta }}</strong></div>
      <div class="row"><span>Top Rows</span><strong class="mono">{{ topRows }} · {{ topFilterMeta }}</strong></div>
      <div class="row"><span>Drill</span><strong class="mono">{{ drillStatus }}</strong></div>
      <div class="row"><span>Stability</span><strong class="mono">{{ stabilityStatus }}</strong></div>
      <div class="row"><span>Heatmap</span><strong class="mono">{{ heatmapMeta }}</strong></div>
      <div class="row"><span>Status</span><strong class="mono">{{ statusLine }}</strong></div>
    </div>
    <div class="actions">
      <button type="button" @click="clickResultsTab('overview')">Overview</button>
      <button type="button" @click="clickResultsTab('top')">Top</button>
      <button type="button" @click="clickResultsTab('drill')">Drill</button>
      <button type="button" @click="runOptimizationAction">Run</button>
      <button type="button" @click="runValidationAction">Validate</button>
      <button type="button" @click="exportTopCsvAction">Export top CSV</button>
    </div>
  </aside>
</template>

<style scoped>
.analysisDock {
  position: fixed;
  right: 10px;
  bottom: 224px;
  z-index: 4037;
  width: min(430px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid rgba(45, 212, 191, 0.42);
  background: linear-gradient(158deg, rgba(15, 23, 42, 0.94), rgba(4, 47, 46, 0.9));
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
  color: #5eead4;
}

.stamp {
  font-size: 0.72rem;
  color: #99f6e4;
}

.rows {
  display: grid;
  gap: 4px;
}

.row {
  display: grid;
  grid-template-columns: 78px 1fr;
  gap: 6px;
  align-items: baseline;
}

.row span {
  font-size: 0.72rem;
  color: #2dd4bf;
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
  flex-wrap: wrap;
  gap: 6px;
}

.actions button {
  border: 1px solid rgba(45, 212, 191, 0.38);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: rgba(4, 47, 46, 0.82);
  color: #ccfbf1;
}

@media (max-width: 760px) {
  .analysisDock {
    left: 8px;
    right: 8px;
    width: auto;
    bottom: 8px;
  }
}
</style>


