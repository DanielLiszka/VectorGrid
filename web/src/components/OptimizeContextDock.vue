<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import { storeToRefs } from 'pinia'
import { useStorage } from '@vueuse/core'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useCommandActionStore } from '../stores/commandActionStore'
import { OPTIMIZE_DOCK_OPEN_STORAGE_KEY } from '../utils/shellDockContract'
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
const dockOpen = useStorage<boolean>(OPTIMIZE_DOCK_OPEN_STORAGE_KEY, false)
const overlayRootRef = ref<HTMLElement | null>(null)
const launcherPos = useStorage<{ x: number; y: number }>('VTA_OPTIMIZE_DOCK_POS_V1', { x: 10, y: 10 })
const launcherPosInitialized = useStorage<boolean>('VTA_OPTIMIZE_DOCK_POS_INIT_V1', false)
const dragState = shallowRef<{
  pointerId: number
  startClientX: number
  startClientY: number
  startX: number
  startY: number
  dragging: boolean
} | null>(null)
const dockPanelId = 'vta-optimize-dock-panel'

const rangeLabel = computed(() => `F ${fastStart.value || '?'}..${fastEnd.value || '?'} | S ${slowStart.value || '?'}..${slowEnd.value || '?'}`)
const maLabel = computed(() => `${(fastMaType.value || '?').toUpperCase()} / ${(slowMaType.value || '?').toUpperCase()}`)
const quietMode = computed(() => !dockOpen.value)
const dragActive = computed(() => !!dragState.value?.dragging)
const quietState = computed(() => {
  const parts = [
    String(strategyKind.value || '').trim(),
    String(progressMeta.value || '').trim() || String(pairsPreview.value || '').trim(),
  ].filter((part) => part.length > 0)
  return parts.join(' · ') || 'Ready'
})
const rootStyle = computed(() => {
  const pos = normalizeLauncherPos(launcherPos.value) ?? { x: 10, y: 10 }
  return {
    left: `${Math.round(pos.x)}px`,
    top: `${Math.round(pos.y)}px`,
    right: 'auto',
    bottom: 'auto',
  }
})

function runOptimizationAction(): void {
  commandActionStore.runOptimization()
}

function runValidationAction(): void {
  commandActionStore.validateOptimization()
}

function toggleDock(): void {
  dockOpen.value = !dockOpen.value
}

function closeDock(): void {
  dockOpen.value = false
}

function normalizeLauncherPos(value: unknown): { x: number; y: number } | null {
  if (!value || typeof value !== 'object') return null
  const row = value as { x?: unknown; y?: unknown }
  const x = Number(row.x)
  const y = Number(row.y)
  if (!Number.isFinite(x) || !Number.isFinite(y)) return null
  return { x, y }
}

function clampLauncherPos(next: { x: number; y: number }): { x: number; y: number } {
  const margin = 8
  const width = Math.max(1, overlayRootRef.value?.offsetWidth ?? 140)
  const height = Math.max(1, overlayRootRef.value?.offsetHeight ?? 42)
  const maxX = Math.max(margin, window.innerWidth - width - margin)
  const maxY = Math.max(margin, window.innerHeight - height - margin)
  return {
    x: Math.min(maxX, Math.max(margin, Math.round(next.x))),
    y: Math.min(maxY, Math.max(margin, Math.round(next.y))),
  }
}

function setLauncherPos(next: { x: number; y: number }): void {
  launcherPos.value = clampLauncherPos(next)
}

function syncInitialLauncherPos(): void {
  const stored = normalizeLauncherPos(launcherPos.value)
  if (!launcherPosInitialized.value || !stored) {
    const height = Math.max(1, overlayRootRef.value?.offsetHeight ?? 42)
    setLauncherPos({
      x: Math.max(8, 10),
      y: Math.max(8, window.innerHeight - height - 212),
    })
    launcherPosInitialized.value = true
    return
  }
  setLauncherPos(stored)
}

function onViewportResize(): void {
  const stored = normalizeLauncherPos(launcherPos.value)
  if (!stored) return
  setLauncherPos(stored)
}

function beginDockDrag(event: PointerEvent): void {
  if (event.button !== 0) return
  const stored = normalizeLauncherPos(launcherPos.value) ?? { x: 10, y: 10 }
  dragState.value = {
    pointerId: event.pointerId,
    startClientX: event.clientX,
    startClientY: event.clientY,
    startX: stored.x,
    startY: stored.y,
    dragging: false,
  }
  const target = event.currentTarget as HTMLElement | null
  target?.setPointerCapture?.(event.pointerId)
}

function moveDockDrag(event: PointerEvent): void {
  const state = dragState.value
  if (!state || state.pointerId !== event.pointerId) return
  const dx = event.clientX - state.startClientX
  const dy = event.clientY - state.startClientY
  const moved = Math.abs(dx) > 3 || Math.abs(dy) > 3
  if (!state.dragging && moved) {
    dragState.value = { ...state, dragging: true }
  }
  const active = dragState.value
  if (!active?.dragging) return
  setLauncherPos({ x: active.startX + dx, y: active.startY + dy })
  event.preventDefault()
}

function endDockDrag(event: PointerEvent): void {
  const state = dragState.value
  if (!state || state.pointerId !== event.pointerId) return
  const target = event.currentTarget as HTMLElement | null
  if (target?.hasPointerCapture?.(event.pointerId)) {
    target.releasePointerCapture(event.pointerId)
  }
  dragState.value = null
}

onMounted(() => {
  nextTick(() => {
    syncInitialLauncherPos()
    window.addEventListener('resize', onViewportResize, { passive: true })
  })
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', onViewportResize)
})
</script>

<template>
  <div ref="overlayRootRef" class="optDockRoot" :style="rootStyle">
    <div class="dockAnchor">
      <button
        class="dockToggle"
        :class="{ quiet: quietMode }"
        type="button"
        :aria-expanded="dockOpen ? 'true' : 'false'"
        :aria-controls="dockPanelId"
        @click="toggleDock"
      >
        <span class="dockToggleLabel">{{ quietMode ? 'Optimize' : 'Optimize Context' }}</span>
        <span class="dockToggleState">{{ quietState }}</span>
      </button>
    </div>

    <aside
      v-if="dockOpen"
      :id="dockPanelId"
      class="optDock"
      aria-label="Optimize context dock"
    >
      <header
        class="dockHeader"
        :class="{ dragging: dragActive }"
        @pointerdown="beginDockDrag"
        @pointermove="moveDockDrag"
        @pointerup="endDockDrag"
        @pointercancel="endDockDrag"
      >
        <h2 class="title" style="margin: 0;">Optimize Context</h2>
        <div class="headerActions">
          <div class="stamp">{{ stamp }}</div>
          <button
            class="panelCloseBtn"
            type="button"
            aria-label="Minimize optimize context"
            @click="closeDock"
            @pointerdown.stop
            @pointermove.stop
            @pointerup.stop
            @pointercancel.stop
          >
            Minimize
          </button>
        </div>
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
  </div>
</template>

<style scoped>
.optDockRoot {
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
  --dock-badge-border: color-mix(in oklab, var(--dock-accent-ink) 34%, var(--border));
  --dock-badge-surface: color-mix(in oklab, var(--dock-accent-soft) 44%, var(--surface-inline));
  --dock-surface-quiet: color-mix(in oklab, var(--surface-sticky) 94%, var(--panel-footer));
  --dock-action-text: color-mix(in oklab, var(--dock-accent-ink) 82%, var(--dock-text));
  position: fixed;
  z-index: 4038;
  pointer-events: none;
}

.optDockRoot > * {
  pointer-events: auto;
}

.dockAnchor {
  display: flex;
  justify-content: flex-start;
}

.dockToggle {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  border: 1px solid var(--dock-border);
  border-radius: 999px;
  background: var(--dock-surface-quiet);
  color: var(--dock-action-text);
  padding: 6px 10px;
  font-size: 0.75rem;
  font-weight: 650;
  letter-spacing: 0.04em;
  box-shadow: var(--shadow-inline);
  backdrop-filter: blur(10px);
  transition:
    opacity 140ms ease,
    transform 140ms ease,
    box-shadow 140ms ease,
    background-color 140ms ease;
}

.dockToggle.quiet {
  gap: 6px;
  padding: 5px 9px;
  background: color-mix(in oklab, var(--dock-surface-quiet) 94%, var(--panel));
  opacity: 0.84;
}

.dockToggle.quiet:hover {
  opacity: 1;
  transform: translateY(-1px);
}

.dockToggleLabel {
  text-transform: uppercase;
}

.dockToggleState {
  border: 1px solid var(--dock-badge-border);
  border-radius: 999px;
  padding: 1px 6px;
  font-size: 0.66rem;
  color: var(--dock-stamp);
  background: var(--dock-badge-surface);
}

.optDock {
  position: absolute;
  left: 0;
  bottom: calc(100% + 10px);
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
  gap: 0.6rem;
  cursor: grab;
  touch-action: none;
  user-select: none;
}

.dockHeader.dragging {
  cursor: grabbing;
}

.headerActions {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
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

.panelCloseBtn {
  border: 1px solid var(--dock-field-border);
  border-radius: 999px;
  padding: 0.18rem 0.5rem;
  font-size: 0.68rem;
  background: var(--dock-field-surface);
  color: var(--dock-action-text);
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
  .optDockRoot {
    max-width: calc(100vw - 16px);
  }

  .optDock {
    width: min(380px, calc(100vw - 16px));
  }
}
</style>
