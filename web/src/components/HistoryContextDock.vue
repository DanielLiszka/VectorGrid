<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useHistoryStore } from '../stores/historyStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const history = useHistoryStore()
const workflowState = useWorkflowStore()
const {
  sessionRunCount,
  sessionSelectedLabel,
  savedRunCount,
  savedSelectedValue,
  savedSelectedLabel,
  compareRunCount,
  savedRunsMeta,
  savedRunDetailTitle,
  savedRunDetailMeta,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(history)
const { workspace, workflow } = storeToRefs(workflowState)

const visible = computed(() => {
  const ws = String(workspace.value || '').trim()
  const wf = String(workflow.value || '').trim()
  return ws === 'library' || wf === 'history'
})

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

</script>

<template>
  <aside v-if="visible" class="historyDock">
    <header class="dockHeader">
      <h2 class="title" style="margin: 0;">History Context</h2>
      <div class="stamp">{{ stamp }}</div>
    </header>
    <div class="rows">
      <div class="row"><span>Session</span><strong class="mono">{{ sessionRunCount }} runs</strong></div>
      <div class="row"><span>Selected</span><strong class="mono">{{ sessionSelectedLabel }}</strong></div>
      <div class="row"><span>Saved</span><strong class="mono">{{ savedRunCount }} runs</strong></div>
      <div class="row"><span>Saved Sel</span><strong class="mono">{{ savedSelectedLabel }}</strong></div>
      <div class="row"><span>Compare</span><strong class="mono">{{ compareRunCount }} choices</strong></div>
      <div class="row"><span>Saved Meta</span><strong class="mono">{{ savedRunsMeta }}</strong></div>
      <div class="row"><span>Detail</span><strong class="mono">{{ savedRunDetailTitle }}</strong></div>
      <div class="row"><span>Hint</span><strong class="mono">{{ savedRunDetailMeta }}</strong></div>
      <div class="row"><span>Status</span><strong class="mono">{{ statusLine }}</strong></div>
    </div>
    <div class="actions">
      <button type="button" @click="history.applySession()">Apply session</button>
      <button type="button" @click="history.refreshSavedRuns()">Refresh saved</button>
      <button type="button" @click="history.loadSavedRun(savedSelectedValue)">Load saved</button>
      <button type="button" @click="history.applySavedRun(savedSelectedValue)">Apply saved</button>
      <button type="button" @click="history.clearSession()">Clear session</button>
      <button type="button" @click="history.openRunsDir()">Open runs dir</button>
    </div>
  </aside>
</template>

<style scoped>
.historyDock {
  position: fixed;
  right: 10px;
  bottom: 442px;
  z-index: 4036;
  width: min(430px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid rgba(125, 211, 252, 0.42);
  background: linear-gradient(158deg, rgba(15, 23, 42, 0.94), rgba(15, 60, 86, 0.9));
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
  color: #7dd3fc;
}

.stamp {
  font-size: 0.72rem;
  color: #bae6fd;
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
  color: #38bdf8;
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
  border: 1px solid rgba(125, 211, 252, 0.38);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: rgba(15, 60, 86, 0.82);
  color: #e0f2fe;
}

@media (max-width: 760px) {
  .historyDock {
    left: 8px;
    right: 8px;
    width: auto;
    bottom: 8px;
  }
}

@media (max-width: 1180px) {
  .historyDock {
    display: none;
  }
}
</style>

