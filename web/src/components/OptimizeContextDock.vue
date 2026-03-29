<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useCommandActionStore } from '../stores/commandActionStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const commandActionStore = useCommandActionStore()
const store = useOptimizeStore()
const {
  strategyKind,
  backend,
  objective,
  mode,
  fastStart,
  fastEnd,
  slowStart,
  slowEnd,
  fastMaType,
  slowMaType,
  pairsPreview,
  gpuStatus,
  progressMeta,
  runDisabled,
  validateDisabled,
  updatedAtUnixMs,
} = storeToRefs(store)

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

const rangeLabel = computed(() => `F ${fastStart.value || '?'}..${fastEnd.value || '?'} | S ${slowStart.value || '?'}..${slowEnd.value || '?'}`)
const maLabel = computed(() => `${(fastMaType.value || '?').toUpperCase()} / ${(slowMaType.value || '?').toUpperCase()}`)

function runOptimizationAction(): void {
  commandActionStore.runOptimization()
}

function runValidationAction(): void {
  commandActionStore.validateOptimization()
}
</script>

<template>
  <aside class="optDock" aria-label="Optimize context dock">
    <header class="dockHeader">
      <h2 class="title" style="margin: 0;">Optimize Context</h2>
      <div class="stamp">{{ stamp }}</div>
    </header>
    <div class="rows">
      <div class="row"><span>Strategy</span><strong>{{ strategyKind || '—' }}</strong></div>
      <div class="row"><span>Backend</span><strong>{{ backend || '—' }}</strong></div>
      <div class="row"><span>Objective</span><strong>{{ objective || '—' }} · {{ mode || '—' }}</strong></div>
      <div class="row"><span>Ranges</span><strong class="mono">{{ rangeLabel }}</strong></div>
      <div class="row"><span>MA Types</span><strong class="mono">{{ maLabel }}</strong></div>
      <div class="row"><span>Pairs</span><strong class="mono">{{ pairsPreview }}</strong></div>
      <div class="row"><span>GPU</span><strong class="mono">{{ gpuStatus }}</strong></div>
      <div class="row"><span>Progress</span><strong class="mono">{{ progressMeta || '—' }}</strong></div>
    </div>
    <div class="actions">
      <button type="button" :disabled="runDisabled" @click="runOptimizationAction">Run</button>
      <button type="button" :disabled="validateDisabled" @click="runValidationAction">Validate</button>
    </div>
  </aside>
</template>

<style scoped>
.optDock {
  --dock-accent: var(--warning);
  --dock-accent-ink: var(--warning-ink);
  --dock-accent-soft: var(--warning-soft);
  --dock-surface: linear-gradient(
    160deg,
    color-mix(in oklab, var(--surface-focus) 90%, var(--panel-footer)),
    color-mix(in oklab, var(--warning-soft) 34%, var(--surface-sticky))
  );
  --dock-border: color-mix(in oklab, var(--dock-accent) 40%, var(--border));
  --dock-field-border: color-mix(in oklab, var(--dock-accent) 34%, var(--border));
  --dock-field-surface: color-mix(in oklab, var(--dock-accent-soft) 48%, var(--surface-inline));
  --dock-text: var(--text);
  --dock-title: color-mix(in oklab, var(--dock-accent) 74%, var(--dock-text));
  --dock-stamp: color-mix(in oklab, var(--dock-accent-ink) 78%, var(--dock-text));
  --dock-label: color-mix(in oklab, var(--dock-accent) 62%, var(--dock-text));
  --dock-action-text: color-mix(in oklab, var(--dock-accent-ink) 82%, var(--dock-text));
  position: fixed;
  left: 10px;
  bottom: 212px;
  z-index: 4038;
  width: min(380px, calc(100vw - 20px));
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
  .optDock {
    left: 8px;
    right: 8px;
    width: auto;
    bottom: 190px;
  }
}
</style>
