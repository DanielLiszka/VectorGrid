<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useCommandActionStore } from '../stores/commandActionStore'

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

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  const d = new Date(ms)
  return d.toLocaleTimeString([], { hour12: false })
})

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
  <aside class="optDock">
    <header class="dockHeader">
      <div class="title">Optimize Context</div>
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
  position: fixed;
  left: 10px;
  bottom: 212px;
  z-index: 4038;
  width: min(380px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid rgba(251, 191, 36, 0.42);
  background: linear-gradient(160deg, rgba(15, 23, 42, 0.94), rgba(66, 32, 6, 0.9));
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
  color: #fcd34d;
}

.stamp {
  font-size: 0.72rem;
  color: #fde68a;
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
  color: #fbbf24;
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
  border: 1px solid rgba(251, 191, 36, 0.38);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: rgba(66, 32, 6, 0.82);
  color: #fef3c7;
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
