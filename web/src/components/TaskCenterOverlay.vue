<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useStorage } from '@vueuse/core'
import { invokeCommand } from '../tauri/invoke'
import { useAnchoredOverlay } from '../composables/useAnchoredOverlay'
import { formatUiCompactDurationMs, formatUiPercentFromFraction } from '../utils/uiFormat'
import {
  describeTrackedTask,
  taskLifecycleLabel,
  taskLifecycleTone,
} from '../utils/statusPresentation'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'
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
const toggleRef = ref<HTMLButtonElement | null>(null)
const busyCancelIds = shallowRef<Set<string>>(new Set())
const taskCenterPanelId = 'vta-task-center-panel'
const { referenceRef, floatingRef, floatingStyles } = useAnchoredOverlay(expanded, {
  placement: 'bottom-end',
  offsetPx: 10,
  paddingPx: 10,
  insideRefs: [overlayRootRef],
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
const lastFocusedElement = shallowRef<HTMLElement | null>(null)

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
const quietMode = computed(() => !expanded.value && runningCount.value <= 0 && !overall.value && toastItems.value.length === 0)
const panelDescriptionId = 'vta-task-center-description'
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
  return taskLifecycleLabel(centerStatus.value)
})

function toggleExpanded(): void {
  if (ignoreNextToggle.value) {
    ignoreNextToggle.value = false
    return
  }
  if (!expanded.value) {
    lastFocusedElement.value = document.activeElement instanceof HTMLElement ? document.activeElement : null
  }
  expanded.value = !expanded.value
}

function closePanel(): void {
  if (!expanded.value) return
  expanded.value = false
}

function panelFocusTargets(root: HTMLElement): HTMLElement[] {
  const selectors =
    'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
  return Array.from(root.querySelectorAll<HTMLElement>(selectors)).filter(
    (el) => !el.hasAttribute('disabled') && el.getAttribute('aria-hidden') !== 'true',
  )
}

function onPanelKeydown(event: KeyboardEvent): void {
  if (!expanded.value) return
  if (event.key === 'Escape') {
    event.preventDefault()
    closePanel()
    return
  }
  if (event.key !== 'Tab') return
  const root = floatingRef.value
  if (!root) return
  const targets = panelFocusTargets(root)
  if (!targets.length) {
    event.preventDefault()
    root.focus()
    return
  }
  const first = targets[0]
  const last = targets[targets.length - 1]
  const active = document.activeElement instanceof HTMLElement ? document.activeElement : null
  if (event.shiftKey) {
    if (active === first || active === root) {
      event.preventDefault()
      last?.focus()
    }
    return
  }
  if (active === last) {
    event.preventDefault()
    first?.focus()
  }
}

function onDocumentKeydown(event: KeyboardEvent): void {
  if (!expanded.value || event.key !== 'Escape') return
  event.preventDefault()
  closePanel()
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
  document.removeEventListener('keydown', onDocumentKeydown)
})

watch(
  expanded,
  async (isOpen, wasOpen) => {
    if (isOpen) {
      document.addEventListener('keydown', onDocumentKeydown)
      await nextTick()
      const panel = floatingRef.value
      if (!panel) return
      const firstFocusable = panelFocusTargets(panel)[0]
      ;(firstFocusable ?? panel).focus()
      return
    }
    if (!wasOpen) return
    document.removeEventListener('keydown', onDocumentKeydown)
    await nextTick()
    ;(toggleRef.value ?? lastFocusedElement.value)?.focus()
  },
  { flush: 'post' },
)

function statusLabel(task: TrackedTask): string {
  return taskLifecycleLabel(task.status)
}

function statusClass(task: TrackedTask): string {
  return taskLifecycleTone(task.status)
}

function taskDetail(task: TrackedTask): string {
  return describeTrackedTask(task)
}

function absoluteTime(unixMs: number): string {
  return formatUiTimeFromUnixMs(unixMs)
}

function durationText(task: TrackedTask): string {
  const ended = task.endedAt ?? Date.now()
  return formatUiCompactDurationMs(Math.max(0, ended - task.startedAt))
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
        ref="toggleRef"
        class="overlayToggle"
        :class="{ dragging: toggleDragging, quiet: quietMode }"
        type="button"
        :aria-expanded="expanded ? 'true' : 'false'"
        :aria-controls="taskCenterPanelId"
        aria-haspopup="dialog"
        @click="toggleExpanded"
        @pointerdown="onTogglePointerDown"
        @pointermove="onTogglePointerMove"
        @pointerup="onTogglePointerUp"
        @pointercancel="onTogglePointerCancel"
      >
        <span class="toggleLabel">{{ quietMode ? 'Tasks' : 'Task Center' }}</span>
        <span v-if="!quietMode || runningCount > 0" class="toggleBadge">{{ runningCount }}</span>
        <span v-if="!quietMode || centerStatusLabel !== 'Idle'" class="toggleState">{{ centerStatusLabel }}</span>
        <span v-if="overall" class="togglePct">{{ formatUiPercentFromFraction(overall.pct) }}</span>
      </button>
    </div>

    <section v-if="toastItems.length > 0" class="toastStack">
      <article v-for="toast in toastItems" :key="toast.id" class="toastCard" :class="toastClass(toast)">
        <div class="toastBody">{{ toast.message }}</div>
        <button class="toastClose" type="button" @click="dismissToast(toast.id)">Close</button>
      </article>
    </section>

    <section
      v-if="expanded"
      :id="taskCenterPanelId"
      ref="floatingRef"
      class="taskPanel"
      :style="floatingStyles"
      role="dialog"
      aria-modal="false"
      aria-label="Task center"
      :aria-describedby="panelDescriptionId"
      tabindex="-1"
      @keydown.capture="onPanelKeydown"
    >
      <div class="taskPanelHeader">
        <div class="taskPanelHeaderCopy">
          <div class="sectionTitle">Task center</div>
          <div :id="panelDescriptionId" class="muted">
            Review active commands, recent events, and the command timeline. Press Escape to close this panel.
          </div>
        </div>
        <button type="button" class="panelCloseBtn" @click="closePanel">Close</button>
      </div>

      <div v-if="overall" class="overallWrap">
        <div class="overallTitle">{{ overall.label }}</div>
        <div class="bar">
          <div class="fill" :style="{ width: formatUiPercentFromFraction(overall.pct, 1) }"></div>
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
                {{ isCancelBusy(task.id) ? 'Cancelling…' : 'Cancel' }}
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
  --task-border: color-mix(in oklab, var(--panel-focus-edge) 76%, var(--border));
  --task-surface: color-mix(in oklab, var(--surface-focus) 92%, var(--panel));
  --task-surface-muted: color-mix(in oklab, var(--surface-sunken) 88%, var(--panel));
  --task-surface-sticky: color-mix(in oklab, var(--surface-sticky) 92%, var(--panel-footer));
  --task-text: var(--text);
  --task-muted: var(--muted-strong);
  --task-muted-soft: var(--muted);
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
  border: 1px solid var(--task-border);
  border-radius: 999px;
  padding: 7px 12px;
  background: color-mix(in oklab, var(--task-surface) 76%, var(--bg));
  color: var(--task-text);
  font-weight: 600;
  letter-spacing: 0.01em;
  backdrop-filter: blur(10px);
  box-shadow: var(--shadow-focus);
  transition:
    opacity var(--motion-fast) var(--ease-standard),
    transform var(--motion-fast) var(--ease-standard),
    box-shadow var(--motion-fast) var(--ease-standard),
    background-color var(--motion-fast) var(--ease-standard),
    border-color var(--motion-fast) var(--ease-standard);
}

.overlayToggle.dragging {
  cursor: grabbing;
}

.overlayToggle.quiet {
  gap: 6px;
  padding: 5px 10px;
  background: color-mix(in oklab, var(--task-surface-muted) 78%, var(--panel));
  border-color: color-mix(in oklab, var(--panel-focus-edge) 56%, var(--border));
  color: var(--task-muted);
  opacity: 0.84;
  box-shadow: var(--shadow-inline);
}

.overlayToggle.quiet:hover {
  opacity: 1;
  transform: translateY(-1px);
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
  background: color-mix(in oklab, var(--selection-fill-strong) 84%, var(--surface-working));
  color: var(--selection-text);
}

.togglePct {
  font-size: 0.78rem;
  color: var(--selection-text);
}

.toggleState {
  font-size: 0.7rem;
  color: var(--task-muted);
  border: 1px solid color-mix(in oklab, var(--border-strong) 74%, transparent);
  border-radius: 999px;
  padding: 1px 6px;
  background: color-mix(in oklab, var(--surface-inline) 88%, transparent);
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
  border: 1px solid color-mix(in oklab, var(--border-strong) 78%, transparent);
  background: var(--task-surface-sticky);
  color: var(--task-text);
  box-shadow: var(--shadow-inline);
}

.toastCard.success {
  border-color: color-mix(in oklab, var(--ok) 42%, var(--border));
  background: color-mix(in oklab, var(--surface-success) 92%, var(--task-surface-sticky));
  color: var(--ok-ink);
}

.toastCard.warn {
  border-color: color-mix(in oklab, var(--warning) 42%, var(--border));
  background: color-mix(in oklab, var(--surface-warning) 92%, var(--task-surface-sticky));
  color: var(--warning-ink);
}

.toastCard.error {
  border-color: color-mix(in oklab, var(--danger) 42%, var(--border));
  background: color-mix(in oklab, var(--surface-danger) 92%, var(--task-surface-sticky));
  color: var(--danger-ink);
}

.toastBody {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.toastClose {
  border: 1px solid color-mix(in oklab, var(--border-strong) 72%, transparent);
  border-radius: 6px;
  background: transparent;
  color: inherit;
  font-size: 0.7rem;
  padding: 2px 6px;
}

.taskPanel {
  width: min(460px, calc(100vw - 24px));
  border: 1px solid var(--task-border);
  border-radius: 16px;
  padding: 10px;
  display: grid;
  gap: 10px;
  background:
    linear-gradient(
      145deg,
      color-mix(in oklab, var(--task-surface) 94%, var(--panel-footer)),
      color-mix(in oklab, var(--surface-working) 88%, var(--task-surface))
    );
  color: var(--task-text);
  box-shadow: var(--shadow-lg);
  max-height: min(72vh, 760px);
  overflow: auto;
}

.taskPanelHeader {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.taskPanelHeaderCopy {
  display: grid;
  gap: 4px;
}

.overallWrap {
  display: grid;
  gap: 6px;
}

.overallTitle {
  font-size: 0.8rem;
  color: var(--selection-text);
}

.section {
  display: grid;
  gap: 6px;
}

.sectionTitle {
  font-size: 0.75rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: color-mix(in oklab, var(--accent) 70%, var(--muted-strong));
}

.panelCloseBtn {
  padding: 4px 10px;
  font-size: 0.72rem;
  border-radius: 999px;
}

.muted {
  font-size: 0.78rem;
  color: var(--task-muted-soft);
}

.taskList {
  display: grid;
  gap: 8px;
}

.taskCard {
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 64%, var(--border));
  border-radius: 12px;
  padding: 8px;
  display: grid;
  gap: 6px;
  background: color-mix(in oklab, var(--surface-working) 92%, var(--panel));
  box-shadow: var(--shadow-inline);
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
  color: var(--task-text);
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
  color: var(--selection-text);
  border-color: color-mix(in oklab, var(--accent) 34%, var(--border));
  background: color-mix(in oklab, var(--selection-fill-strong) 82%, var(--surface-working));
}

.taskStatus.queued {
  color: var(--task-text);
  border-color: color-mix(in oklab, var(--accent-hover) 32%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 84%, var(--surface-inline));
}

.taskStatus.cancelling {
  color: var(--warning-ink);
  border-color: color-mix(in oklab, var(--warning) 36%, var(--border));
  background: var(--surface-warning);
}

.taskStatus.success {
  color: var(--ok-ink);
  border-color: color-mix(in oklab, var(--ok) 36%, var(--border));
  background: var(--surface-success);
}

.taskStatus.cancelled {
  color: var(--warning-ink);
  border-color: color-mix(in oklab, var(--warning) 36%, var(--border));
  background: color-mix(in oklab, var(--warning-soft) 88%, var(--surface-inline));
}

.taskStatus.failed {
  color: var(--danger-ink);
  border-color: color-mix(in oklab, var(--danger) 36%, var(--border));
  background: var(--surface-danger);
}

.taskMeta {
  font-size: 0.76rem;
  color: var(--task-muted);
  word-break: break-word;
}

.bar {
  height: 7px;
  border-radius: 999px;
  overflow: hidden;
  background: color-mix(in oklab, var(--surface-sunken) 82%, var(--panel));
}

.bar.thin {
  height: 5px;
}

.fill {
  height: 100%;
  min-width: 2px;
  background: linear-gradient(90deg, var(--accent), var(--accent-hover), var(--ok));
}

.time {
  font-size: 0.7rem;
  color: var(--task-muted-soft);
}

.cancelBtn {
  border: 1px solid color-mix(in oklab, var(--danger) 44%, var(--border));
  border-radius: 8px;
  padding: 3px 8px;
  font-size: 0.72rem;
  color: var(--danger-ink);
  background: color-mix(in oklab, var(--surface-danger) 88%, var(--surface-inline));
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
  color: var(--task-muted);
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
  color: var(--task-muted-soft);
}

.eventText {
  color: var(--task-muted);
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
