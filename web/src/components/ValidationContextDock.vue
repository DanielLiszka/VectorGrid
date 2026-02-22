<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useValidationStore } from '../stores/validationStore'
import { useCommandActionStore } from '../stores/commandActionStore'

const commandActionStore = useCommandActionStore()
const store = useValidationStore()
const {
  validationMeta,
  validationOutputPreview,
  crossMeta,
  tableRowCount,
  minPsr,
  minDsr,
  cscvSlices,
  statusLine,
  validateDisabled,
  updatedAtUnixMs,
} = storeToRefs(store)

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  const d = new Date(ms)
  return d.toLocaleTimeString([], { hour12: false })
})

const gatesLabel = computed(() => {
  const parts: string[] = []
  if (minPsr.value) parts.push(`PSR≥${minPsr.value}`)
  if (minDsr.value) parts.push(`DSR≥${minDsr.value}`)
  if (cscvSlices.value) parts.push(`CSCV=${cscvSlices.value}`)
  return parts.join(' · ') || '—'
})

function exportShortlistAction(): void {
  commandActionStore.exportValidationShortlistCsv()
}

function runValidationAction(): void {
  commandActionStore.validateOptimization()
}
</script>

<template>
  <aside class="valDock">
    <header class="dockHeader">
      <div class="title">Validation Context</div>
      <div class="stamp">{{ stamp }}</div>
    </header>
    <div class="rows">
      <div class="row"><span>Meta</span><strong class="mono">{{ validationMeta }}</strong></div>
      <div class="row"><span>Rows</span><strong class="mono">{{ tableRowCount }}</strong></div>
      <div class="row"><span>Cross</span><strong class="mono">{{ crossMeta }}</strong></div>
      <div class="row"><span>Gates</span><strong class="mono">{{ gatesLabel }}</strong></div>
      <div class="row"><span>Status</span><strong class="mono">{{ statusLine }}</strong></div>
      <div class="row"><span>Preview</span><strong class="mono">{{ validationOutputPreview }}</strong></div>
    </div>
    <div class="actions">
      <button type="button" :disabled="validateDisabled" @click="runValidationAction">Run validation</button>
      <button type="button" @click="exportShortlistAction">Export shortlist</button>
    </div>
  </aside>
</template>

<style scoped>
.valDock {
  position: fixed;
  right: 10px;
  bottom: 10px;
  z-index: 4038;
  width: min(410px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid rgba(34, 197, 94, 0.42);
  background: linear-gradient(160deg, rgba(15, 23, 42, 0.94), rgba(9, 48, 20, 0.9));
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
  color: #86efac;
}

.stamp {
  font-size: 0.72rem;
  color: #bbf7d0;
}

.rows {
  display: grid;
  gap: 4px;
}

.row {
  display: grid;
  grid-template-columns: 76px 1fr;
  gap: 6px;
  align-items: baseline;
}

.row span {
  font-size: 0.72rem;
  color: #4ade80;
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
  border: 1px solid rgba(74, 222, 128, 0.4);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: rgba(9, 48, 20, 0.82);
  color: #dcfce7;
}

.actions button:disabled {
  opacity: 0.55;
}

@media (max-width: 760px) {
  .valDock {
    right: 8px;
    left: 8px;
    width: auto;
    bottom: 8px;
  }
}
</style>


