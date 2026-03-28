<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const store = useValidationStore()
const workflowStore = useWorkflowStore()
const {
  validationMeta,
  validationOutputPreview,
  crossMeta,
  tableRowCount,
  minPsr,
  minDsr,
  cscvSlices,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(store)

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

const gatesLabel = computed(() => {
  const parts: string[] = []
  if (minPsr.value) parts.push(`PSR≥${minPsr.value}`)
  if (minDsr.value) parts.push(`DSR≥${minDsr.value}`)
  if (cscvSlices.value) parts.push(`CSCV=${cscvSlices.value}`)
  return parts.join(' · ') || '—'
})

function openValidationWorkspace(): void {
  workflowStore.selectWorkflowStep('validate')
}
</script>

<template>
  <aside class="valDock">
    <header class="dockHeader">
      <h2 class="title" style="margin: 0;">Validation Context</h2>
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
      <button type="button" @click="openValidationWorkspace">Open validation</button>
    </div>
  </aside>
</template>

<style scoped>
.valDock {
  --dock-accent: var(--ok);
  --dock-accent-ink: var(--ok-ink);
  --dock-accent-soft: var(--ok-soft);
  --dock-surface: linear-gradient(
    160deg,
    color-mix(in oklab, var(--surface-focus) 90%, var(--panel-footer)),
    color-mix(in oklab, var(--ok-soft) 34%, var(--surface-sticky))
  );
  --dock-border: color-mix(in oklab, var(--dock-accent) 40%, var(--border));
  --dock-field-border: color-mix(in oklab, var(--dock-accent) 34%, var(--border));
  --dock-field-surface: color-mix(in oklab, var(--dock-accent-soft) 48%, var(--surface-inline));
  --dock-text: var(--text);
  --dock-title: color-mix(in oklab, var(--dock-accent) 74%, var(--dock-text));
  --dock-stamp: color-mix(in oklab, var(--dock-accent-ink) 76%, var(--dock-text));
  --dock-label: color-mix(in oklab, var(--dock-accent) 62%, var(--dock-text));
  --dock-action-text: color-mix(in oklab, var(--dock-accent-ink) 82%, var(--dock-text));
  position: fixed;
  right: 10px;
  bottom: 10px;
  z-index: 4038;
  width: min(410px, calc(100vw - 20px));
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
  grid-template-columns: 76px 1fr;
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
  .valDock {
    right: 8px;
    left: 8px;
    width: auto;
    bottom: 8px;
  }
}
</style>


