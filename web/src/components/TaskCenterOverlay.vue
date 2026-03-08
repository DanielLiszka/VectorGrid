<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'
import { storeToRefs } from 'pinia'
import { useStorage } from '@vueuse/core'
import { invokeCommand } from '../tauri/invoke'
import { useAnchoredOverlay } from '../composables/useAnchoredOverlay'
import {
  activeTasksCount,
  activeTasksSnapshot,
  cancelCommandFor,
  dismissToast,
  formatCommandLabel,
  markTaskCancelling,
  restoreTaskRunning,
  useTaskStore,
  type TaskToastItem,
  type TrackedTask,
} from '../stores/taskStore'

const taskStore = useTaskStore()
const { centerStatus, history, events, overallProgress, timeline, toasts } = storeToRefs(taskStore)

const expanded = useStorage<boolean>('VTA_TASK_CENTER_OPEN_V1', false)
const launcherPos = useStorage<{ x: number; y: number }>('VTA_TASK_CENTER_POS_V1', { x: 12, y: 12 })
const launcherPosInitialized = useStorage<boolean>('VTA_TASK_CENTER_POS_INIT_V1', false)
const overlayRootRef = ref<HTMLElement | null>(null)
const busyCancelIds = shallowRef<Set<string>>(new Set())
const taskCenterPanelId = 'vta-task-center-panel'
const { referenceRef, floatingRef, floatingStyles } = useAnchoredOverlay(expanded, {
  placement: 'bottom-end',
  offsetPx: 10,
  paddingPx: 10,
})
const dragState = shallowRef<{
  pointerId: number
  startClientX: number
  startClientY: number
  startX: number
  startY: number
  dragging: boolean
} | null>(null)
const ignoreNextToggle = ref(false)

function headItems<T>(items: T[], limit: number): T[] {
  return items.length <= limit ? items : items.slice(0, limit)
}

const activeTasks = computed(() => (expanded.value ? activeTasksSnapshot() : []))
const historyTasks = computed(() => headItems(history.value, 8))
const eventItems = computed(() => headItems(events.value, 8))
const timelineItems = computed(() => headItems(timeline.value, 8))
const toastItems = computed(() => headItems(toasts.value, 4))
const runningCount = computed(() => activeTasksCount())
const overall = computed(() => overallProgress.value)
const rootStyle = computed(() => {
  const pos = normalizeLauncherPos(launcherPos.value) ?? { x: 12, y: 12 }
  return {
    left: `${Math.round(pos.x)}px`,
    top: `${Math.round(pos.y)}px`,
    right: 'auto',
    bottom: 'auto',
  }
})
const toggleDragging = computed(() => !!dragState.value?.dragging)
const centerStatusLabel = computed(() => {
  const s = String(centerStatus.value || 'idle')
  if (s === 'running') return 'Running'
  if (s === 'queued') return 'Queued'
  if (s === 'cancelling') return 'Cancelling'
  if (s === 'success') return 'Done'
  if (s === 'failed') return 'Failed'
  if (s === 'cancelled') return 'Cancelled'
  return 'Idle'
})

function toggleExpanded(): void {
  if (ignoreNextToggle.value) {
    ignoreNextToggle.value = false
    return
  }
  expanded.value = !expanded.value
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
  const width = Math.max(1, overlayRootRef.value?.offsetWidth ?? 220)
  const height = Math.max(1, overlayRootRef.value?.offsetHeight ?? 48)
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
    const width = Math.max(1, overlayRootRef.value?.offsetWidth ?? 220)
    setLauncherPos({ x: window.innerWidth - width - 12, y: 12 })
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

function onTogglePointerDown(event: PointerEvent): void {
  if (event.button !== 0) return
  const stored = normalizeLauncherPos(launcherPos.value) ?? { x: 12, y: 12 }
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

function onTogglePointerMove(event: PointerEvent): void {
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

function releaseTogglePointer(event: PointerEvent): void {
  const state = dragState.value
  if (!state || state.pointerId !== event.pointerId) return
  const target = event.currentTarget as HTMLElement | null
  if (target?.hasPointerCapture?.(event.pointerId)) {
    target.releasePointerCapture(event.pointerId)
  }
  if (state.dragging) ignoreNextToggle.value = true
  dragState.value = null
}

function onTogglePointerUp(event: PointerEvent): void {
  releaseTogglePointer(event)
}

function onTogglePointerCancel(event: PointerEvent): void {
  releaseTogglePointer(event)
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

function statusLabel(task: TrackedTask): string {
  if (task.status === 'running') return 'Running'
  if (task.status === 'queued') return 'Queued'
  if (task.status === 'cancelling') return 'Cancelling'
  if (task.status === 'success') return 'Done'
  if (task.status === 'cancelled') return 'Cancelled'
  return 'Failed'
}

function statusClass(task: TrackedTask): string {
  if (task.status === 'running') return 'running'
  if (task.status === 'queued') return 'queued'
  if (task.status === 'cancelling') return 'cancelling'
  if (task.status === 'success') return 'success'
  if (task.status === 'cancelled') return 'cancelled'
  return 'failed'
}

function taskDetail(task: TrackedTask): string {
  const pct = task.progressPct
  if (task.message) return task.message
  if (task.phase && typeof pct === 'number' && Number.isFinite(pct)) return `${task.phase} (${pct.toFixed(1)}%)`
  if (task.phase) return task.phase
  if (task.error) return task.error
  return 'No details.'
}

function absoluteTime(unixMs: number): string {
  const d = new Date(unixMs)
  return d.toLocaleTimeString([], { hour12: false })
}

function durationText(task: TrackedTask): string {
  const ended = task.endedAt ?? Date.now()
  const ms = Math.max(0, ended - task.startedAt)
  if (ms < 1000) return `${ms}ms`
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`
  return `${(ms / 60_000).toFixed(1)}m`
}

function canCancel(task: TrackedTask): boolean {
  return (task.status === 'running' || task.status === 'queued' || task.status === 'cancelling') && cancelCommandFor(task.command) !== null
}

async function cancelTask(task: TrackedTask): Promise<void> {
  const cmd = cancelCommandFor(task.command)
  if (!cmd) return
  markTaskCancelling(task.id)
  setCancelBusy(task.id, true)
  try {
    await invokeCommand<void>(cmd)
  } catch {
    restoreTaskRunning(task.id)
    // Keep cancellation best-effort; error details are already surfaced by backend invoke failures.
  } finally {
    setCancelBusy(task.id, false)
  }
}

function isCancelBusy(taskId: string): boolean {
  return busyCancelIds.value.has(taskId)
}

function setCancelBusy(taskId: string, busy: boolean): void {
  const id = String(taskId || '').trim()
  if (!id) return
  const next = new Set(busyCancelIds.value)
  if (busy) next.add(id)
  else next.delete(id)
  busyCancelIds.value = next
}

function toastClass(toast: TaskToastItem): string {
  if (toast.level === 'success') return 'success'
  if (toast.level === 'warn') return 'warn'
  if (toast.level === 'error') return 'error'
  return 'info'
}
</script>

<template>
  <div ref="overlayRootRef" class="taskOverlayRoot" :style="rootStyle">
    <div ref="referenceRef" class="overlayAnchor">
      <button
        class="overlayToggle"
        :class="{ dragging: toggleDragging }"
        type="button"
        :aria-expanded="expanded ? 'true' : 'false'"
        :aria-controls="taskCenterPanelId"
        @click="toggleExpanded"
        @pointerdown="onTogglePointerDown"
        @pointermove="onTogglePointerMove"
        @pointerup="onTogglePointerUp"
        @pointercancel="onTogglePointerCancel"
      >
        <span class="toggleLabel">Task Center</span>
        <span class="toggleBadge">{{ runningCount }}</span>
        <span class="toggleState">{{ centerStatusLabel }}</span>
        <span v-if="overall" class="togglePct">{{ overall.pct.toFixed(0) }}%</span>
      </button>
    </div>

    <section v-if="toastItems.length > 0" class="toastStack">
      <article v-for="toast in toastItems" :key="toast.id" class="toastCard" :class="toastClass(toast)">
        <div class="toastBody">{{ toast.message }}</div>
        <button class="toastClose" type="button" @click="dismissToast(toast.id)">Close</button>
      </article>
    </section>

    <section v-if="expanded" :id="taskCenterPanelId" ref="floatingRef" class="taskPanel" :style="floatingStyles">
      <div v-if="overall" class="overallWrap">
        <div class="overallTitle">{{ overall.label }}</div>
        <div class="bar">
          <div class="fill" :style="{ width: `${overall.pct.toFixed(1)}%` }"></div>
        </div>
      </div>

      <div class="section">
        <div class="sectionTitle">Running</div>
        <div v-if="activeTasks.length === 0" class="muted">No active commands.</div>
        <div v-else class="taskList">
          <article v-for="task in activeTasks" :key="task.id" class="taskCard">
            <div class="taskRow">
              <div class="taskName">{{ formatCommandLabel(task.command) }}</div>
              <div class="taskStatus" :class="statusClass(task)">{{ statusLabel(task) }}</div>
            </div>
            <div class="taskMeta">{{ taskDetail(task) }}</div>
            <div v-if="Number.isFinite(task.progressPct)" class="bar thin">
              <div class="fill" :style="{ width: `${task.progressPct?.toFixed(1)}%` }"></div>
            </div>
            <div class="taskRow foot">
              <div class="time">{{ absoluteTime(task.startedAt) }} · {{ durationText(task) }}</div>
              <button
                v-if="canCancel(task)"
                class="cancelBtn"
                type="button"
                :disabled="isCancelBusy(task.id)"
                @click="cancelTask(task)"
              >
                {{ isCancelBusy(task.id) ? 'Cancelling...' : 'Cancel' }}
              </button>
            </div>
          </article>
        </div>
      </div>

      <div class="section">
        <div class="sectionTitle">Recent</div>
        <div v-if="historyTasks.length === 0" class="muted">No completed commands yet.</div>
        <div v-else class="miniList">
          <div v-for="task in historyTasks" :key="task.id" class="miniRow">
            <span class="miniName">{{ formatCommandLabel(task.command) }}</span>
            <span class="miniStatus" :class="statusClass(task)">{{ statusLabel(task) }}</span>
          </div>
        </div>
      </div>

      <div class="section">
        <div class="sectionTitle">Events</div>
        <div v-if="eventItems.length === 0" class="muted">No events yet.</div>
        <div v-else class="miniList">
          <div v-for="evt in eventItems" :key="evt.id" class="eventRow">
            <span class="eventTime">{{ absoluteTime(evt.atUnixMs) }}</span>
            <span class="eventText">{{ evt.message }}</span>
          </div>
        </div>
      </div>

      <div class="section">
        <div class="sectionTitle">Timeline</div>
        <div v-if="timelineItems.length === 0" class="muted">No timeline entries yet.</div>
        <div v-else class="miniList">
          <div v-for="entry in timelineItems" :key="entry.id" class="eventRow">
            <span class="eventTime">{{ absoluteTime(entry.atUnixMs) }}</span>
            <span class="eventText">{{ entry.message }}</span>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.taskOverlayRoot {
  position: fixed;
  top: 12px;
  right: 12px;
  z-index: 4000;
  pointer-events: none;
  display: grid;
  gap: 8px;
  justify-items: end;
}

.taskOverlayRoot > * {
  pointer-events: auto;
}

.overlayAnchor {
  display: flex;
  justify-content: flex-end;
}

.overlayToggle {
  justify-self: end;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: grab;
  user-select: none;
  touch-action: none;
  border: 1px solid rgba(2, 132, 199, 0.45);
  border-radius: 999px;
  padding: 7px 12px;
  background: rgba(7, 33, 53, 0.84);
  color: #e0f2fe;
  font-weight: 600;
  letter-spacing: 0.01em;
  backdrop-filter: blur(10px);
  box-shadow: 0 10px 24px rgba(2, 6, 23, 0.35);
}

.overlayToggle.dragging {
  cursor: grabbing;
}

.toggleLabel {
  font-size: 0.86rem;
}

.toggleBadge {
  min-width: 1.6rem;
  border-radius: 999px;
  padding: 1px 6px;
  text-align: center;
  font-size: 0.78rem;
  background: rgba(56, 189, 248, 0.25);
}

.togglePct {
  font-size: 0.78rem;
  color: #bae6fd;
}

.toggleState {
  font-size: 0.7rem;
  color: #cbd5e1;
  border: 1px solid rgba(148, 163, 184, 0.28);
  border-radius: 999px;
  padding: 1px 6px;
}

.toastStack {
  display: grid;
  gap: 6px;
  justify-items: end;
}

.toastCard {
  width: min(340px, 92vw);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  border-radius: 10px;
  padding: 7px 10px;
  font-size: 0.75rem;
  border: 1px solid rgba(125, 211, 252, 0.25);
  background: rgba(15, 23, 42, 0.9);
  color: #e2e8f0;
}

.toastCard.success {
  border-color: rgba(34, 197, 94, 0.35);
  background: rgba(6, 78, 59, 0.85);
}

.toastCard.warn {
  border-color: rgba(234, 179, 8, 0.4);
  background: rgba(113, 63, 18, 0.88);
}

.toastCard.error {
  border-color: rgba(244, 63, 94, 0.4);
  background: rgba(127, 29, 29, 0.88);
}

.toastBody {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.toastClose {
  border: 1px solid rgba(148, 163, 184, 0.35);
  border-radius: 6px;
  background: transparent;
  color: #e2e8f0;
  font-size: 0.7rem;
  padding: 2px 6px;
}

.taskPanel {
  width: min(460px, calc(100vw - 24px));
  border: 1px solid rgba(14, 116, 144, 0.4);
  border-radius: 16px;
  padding: 10px;
  display: grid;
  gap: 10px;
  background:
    linear-gradient(145deg, rgba(15, 23, 42, 0.94), rgba(12, 74, 110, 0.82));
  color: #e2e8f0;
  box-shadow: 0 20px 45px rgba(2, 6, 23, 0.48);
  max-height: min(72vh, 760px);
  overflow: auto;
}

.overallWrap {
  display: grid;
  gap: 6px;
}

.overallTitle {
  font-size: 0.8rem;
  color: #93c5fd;
}

.section {
  display: grid;
  gap: 6px;
}

.sectionTitle {
  font-size: 0.75rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #7dd3fc;
}

.muted {
  font-size: 0.78rem;
  color: #94a3b8;
}

.taskList {
  display: grid;
  gap: 8px;
}

.taskCard {
  border: 1px solid rgba(56, 189, 248, 0.2);
  border-radius: 12px;
  padding: 8px;
  display: grid;
  gap: 6px;
  background: rgba(15, 23, 42, 0.6);
}

.taskRow {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.taskRow.foot {
  margin-top: 2px;
}

.taskName {
  font-weight: 600;
  font-size: 0.81rem;
  color: #e2e8f0;
}

.taskStatus {
  font-size: 0.71rem;
  font-weight: 600;
  border-radius: 999px;
  padding: 2px 8px;
  border: 1px solid transparent;
  letter-spacing: 0.03em;
  text-transform: uppercase;
}

.taskStatus.running {
  color: #67e8f9;
  border-color: rgba(34, 211, 238, 0.35);
  background: rgba(34, 211, 238, 0.12);
}

.taskStatus.queued {
  color: #bfdbfe;
  border-color: rgba(59, 130, 246, 0.35);
  background: rgba(30, 64, 175, 0.22);
}

.taskStatus.cancelling {
  color: #fde68a;
  border-color: rgba(234, 179, 8, 0.35);
  background: rgba(146, 64, 14, 0.22);
}

.taskStatus.success {
  color: #86efac;
  border-color: rgba(34, 197, 94, 0.35);
  background: rgba(34, 197, 94, 0.12);
}

.taskStatus.cancelled {
  color: #fef08a;
  border-color: rgba(234, 179, 8, 0.35);
  background: rgba(234, 179, 8, 0.12);
}

.taskStatus.failed {
  color: #fda4af;
  border-color: rgba(244, 63, 94, 0.35);
  background: rgba(244, 63, 94, 0.12);
}

.taskMeta {
  font-size: 0.76rem;
  color: #cbd5e1;
  word-break: break-word;
}

.bar {
  height: 7px;
  border-radius: 999px;
  overflow: hidden;
  background: rgba(148, 163, 184, 0.18);
}

.bar.thin {
  height: 5px;
}

.fill {
  height: 100%;
  min-width: 2px;
  background: linear-gradient(90deg, #22d3ee, #38bdf8, #0ea5e9);
}

.time {
  font-size: 0.7rem;
  color: #94a3b8;
}

.cancelBtn {
  border: 1px solid rgba(244, 63, 94, 0.48);
  border-radius: 8px;
  padding: 3px 8px;
  font-size: 0.72rem;
  color: #fecdd3;
  background: rgba(136, 19, 55, 0.42);
}

.cancelBtn:disabled {
  opacity: 0.55;
}

.miniList {
  display: grid;
  gap: 4px;
}

.miniRow,
.eventRow {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px;
  font-size: 0.74rem;
  align-items: center;
}

.miniName {
  color: #cbd5e1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.miniStatus {
  justify-self: end;
}

.eventRow {
  grid-template-columns: auto 1fr;
}

.eventTime {
  color: #94a3b8;
}

.eventText {
  color: #cbd5e1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 760px) {
  .taskOverlayRoot {
    top: 8px;
    right: 8px;
  }

  .taskPanel {
    width: min(96vw, 420px);
  }
}
</style>
