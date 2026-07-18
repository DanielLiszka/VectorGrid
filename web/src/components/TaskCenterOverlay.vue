<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useStorage } from '@vueuse/core'
import AppVisualIcon from './AppVisualIcon.vue'
import { useAnchoredOverlay } from '../composables/useAnchoredOverlay'
import { vMotionList } from '../directives/motionList'
import {
  TASK_CENTER_PANEL_ID,
  closeTaskCenter,
  consumeTaskCenterReturnFocus,
  toggleTaskCenter,
  useTaskCenterVisibility,
} from '../composables/useTaskCenterVisibility'
import { requestCancellationForMappedCommand } from '../services/runControlService'
import { formatUiCompactDurationMs, formatUiPercentFromFraction } from '../utils/uiFormat'
import {
  describeTaskEvent,
  describeTaskToast,
  describeTimelineEntry,
  describeTrackedTask,
  formatTaskPhaseLabel,
  formatTaskSourceLabel,
  taskLevelLabel,
  taskLevelTone,
  taskLifecycleLabel,
  taskLifecycleTone,
} from '../utils/statusPresentation'
import { taskLevelIcon, taskStatusIcon, type AppIcon } from '../utils/iconography'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'
import {
  activeTasksCount,
  cancelCommandFor,
  dismissToast,
  formatCommandLabel,
  taskPostureSnapshot,
  useTaskStore,
  type TaskEventItem,
  type TaskTimelineItem,
  type TaskToastItem,
  type TrackedTask,
} from '../stores/taskStore'
import { useWorkflowStore } from '../stores/workflowStore'

const taskStore = useTaskStore()
const workflowStore = useWorkflowStore()
const { centerStatus, events, overallProgress, timeline, toasts } = storeToRefs(taskStore)

const expanded = useTaskCenterVisibility()
const launcherPos = useStorage<{ x: number; y: number }>('VTA_TASK_CENTER_POS_V1', { x: 12, y: 12 })
const launcherPosInitialized = useStorage<boolean>('VTA_TASK_CENTER_POS_INIT_V1', false)
const overlayRootRef = ref<HTMLElement | null>(null)
const toggleRef = ref<HTMLButtonElement | null>(null)
const busyCancelIds = shallowRef<Set<string>>(new Set())
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
const posture = computed(() => taskPostureSnapshot(taskStore.$state, 10))
const queuedTasks = computed(() => (expanded.value ? posture.value.queued : []))
const runningTasks = computed(() => (expanded.value ? posture.value.running : []))
const cancellingTasks = computed(() => (expanded.value ? posture.value.cancelling : []))
const completedTasks = computed(() => posture.value.completed)
const failedTasks = computed(() => posture.value.failed)
const cancelledTasks = computed(() => posture.value.cancelled)
type RelatedTaskAction = { label: string; run: () => void }
type TaskWithRelatedAction = { task: TrackedTask; action: RelatedTaskAction | null }

function taskRowsWithRelatedAction(tasks: TrackedTask[]): TaskWithRelatedAction[] {
  return tasks.map((task) => ({ task, action: relatedActionFor(task) }))
}

const attentionTasks = computed(() => failedTasks.value.concat(cancelledTasks.value))
const attentionTaskRows = computed(() => taskRowsWithRelatedAction(attentionTasks.value))
const completedTaskRows = computed(() => taskRowsWithRelatedAction(completedTasks.value))
const postureSummary = computed(() => {
  const parts = [
    posture.value.counts.running ? `${posture.value.counts.running} running` : '',
    posture.value.counts.queued ? `${posture.value.counts.queued} queued` : '',
    posture.value.counts.cancelling ? `${posture.value.counts.cancelling} cancelling` : '',
    posture.value.counts.failed ? `${posture.value.counts.failed} failed` : '',
    posture.value.counts.cancelled ? `${posture.value.counts.cancelled} cancelled` : '',
    posture.value.counts.completed ? `${posture.value.counts.completed} completed` : '',
  ].filter((part) => part.length > 0)
  return parts.join(' · ') || 'No active or recent commands.'
})

function desktopTitlebarHeightPx(): number {
  const raw = getComputedStyle(document.documentElement)
    .getPropertyValue('--desktop-titlebar-h')
    .trim()
  const parsed = Number.parseFloat(raw)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 0
}

function footerTopPx(): number {
  const footer = document.querySelector<HTMLElement>('#runFooterShell')
  const rect = footer?.getBoundingClientRect()
  if (!rect || rect.top <= 0 || rect.top >= window.innerHeight) {
    return window.innerHeight
  }
  return rect.top
}

function defaultLauncherPos(): { x: number; y: number } {
  const width = Math.max(1, overlayRootRef.value?.offsetWidth ?? 220)
  const height = Math.max(1, overlayRootRef.value?.offsetHeight ?? 48)
  const margin = 12
  return {
    x: window.innerWidth - width - margin,
    y: footerTopPx() - height - margin,
  }
}

function toggleExpanded(): void {
  if (ignoreNextToggle.value) {
    ignoreNextToggle.value = false
    return
  }
  const activeElement = document.activeElement instanceof HTMLElement ? document.activeElement : null
  if (!expanded.value) lastFocusedElement.value = activeElement
  toggleTaskCenter(expanded, toggleRef.value ?? activeElement)
}

function closePanel(): void {
  if (!expanded.value) return
  closeTaskCenter(expanded)
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
  const minY = desktopTitlebarHeightPx() + margin
  const width = Math.max(1, overlayRootRef.value?.offsetWidth ?? 220)
  const height = Math.max(1, overlayRootRef.value?.offsetHeight ?? 48)
  const maxX = Math.max(margin, window.innerWidth - width - margin)
  const maxY = Math.max(minY, window.innerHeight - height - margin)
  return {
    x: Math.min(maxX, Math.max(margin, Math.round(next.x))),
    y: Math.min(maxY, Math.max(minY, Math.round(next.y))),
  }
}

function setLauncherPos(next: { x: number; y: number }): void {
  launcherPos.value = clampLauncherPos(next)
}

function syncInitialLauncherPos(): void {
  const stored = normalizeLauncherPos(launcherPos.value)
  if (!launcherPosInitialized.value || !stored) {
    setLauncherPos(defaultLauncherPos())
    launcherPosInitialized.value = true
    return
  }
  if (stored.y < desktopTitlebarHeightPx() + 24) {
    setLauncherPos(defaultLauncherPos())
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
    ;(consumeTaskCenterReturnFocus() ?? toggleRef.value ?? lastFocusedElement.value)?.focus()
  },
  { flush: 'post' },
)

function statusLabel(task: TrackedTask): string {
  return taskLifecycleLabel(task.status)
}

function statusClass(task: TrackedTask): string {
  return taskLifecycleTone(task.status)
}

function statusIcon(task: TrackedTask): AppIcon {
  return taskStatusIcon(task.status)
}

function statusIconClass(task: TrackedTask): string {
  return task.status === 'running' ? 'running' : ''
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
  setCancelBusy(task.id, true)
  try {
    await requestCancellationForMappedCommand(cmd)
  } catch {
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

function toastClass(item: Pick<TaskToastItem | TaskEventItem | TaskTimelineItem, 'level'>): string {
  return taskLevelTone(item.level)
}

function levelLabel(level: string | null | undefined): string {
  return taskLevelLabel(level)
}

function levelIcon(level: string | null | undefined): AppIcon {
  return taskLevelIcon(level)
}

function sourceLabel(source: string | null | undefined): string {
  return formatTaskSourceLabel(source)
}

function phaseLabel(phase: string | null | undefined): string {
  return formatTaskPhaseLabel(phase)
}

function timelineEntrySummary(index: number): string {
  return describeTimelineEntry(timelineItems.value[index] ?? null)
}

function toastSummary(index: number): string {
  return describeTaskToast(toastItems.value[index] ?? null)
}

function eventSummary(index: number): string {
  return describeTaskEvent(eventItems.value[index] ?? null)
}

function sectionEmptyLabel(kind: 'queued' | 'running' | 'cancelling' | 'completed' | 'attention'): string {
  if (kind === 'queued') return 'No queued commands.'
  if (kind === 'running') return 'No running commands.'
  if (kind === 'cancelling') return 'No cancellation requests in flight.'
  if (kind === 'completed') return 'No recently completed commands.'
  return 'No failed or cancelled commands.'
}

function taskSecondaryMeta(task: TrackedTask): string {
  const phase = phaseLabel(task.phase)
  const detail = taskDetail(task)
  if (phase !== 'Unknown' && detail && detail !== phase) return `${phase} · ${detail}`
  return detail
}

function relatedActionFor(task: TrackedTask): RelatedTaskAction | null {
  const command = String(task.command || '').trim()
  if (!command) return null
  if (command.includes('validation')) {
    return {
      label: 'Open validation',
      run: () => workflowStore.selectWorkflowStep('validate'),
    }
  }
  if (command.includes('optimization')) {
    return {
      label: 'Open top results',
      run: () => {
        workflowStore.selectWorkflowStep('analyze')
        workflowStore.selectResultsTab('top')
      },
    }
  }
  if (command.includes('meta') || command.includes('policy') || command.includes('stage4')) {
    return {
      label: 'Open aggregate analysis',
      run: () => workflowStore.selectWorkspace('aggregate'),
    }
  }
  return null
}

function openRelatedAction(action: RelatedTaskAction | null): void {
  if (!action) return
  action.run()
  closePanel()
}
</script>

<template>
  <div
    v-if="!quietMode"
    ref="overlayRootRef"
    class="taskOverlayRoot"
    :style="rootStyle"
  >
    <div ref="referenceRef" class="overlayAnchor">
      <button
        ref="toggleRef"
        class="overlayToggle"
        :class="{ dragging: toggleDragging, quiet: quietMode }"
        type="button"
        :aria-expanded="expanded ? 'true' : 'false'"
        :aria-controls="TASK_CENTER_PANEL_ID"
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

    <section v-if="toastItems.length > 0" v-motion-list class="toastStack">
      <article
        v-for="(toast, index) in toastItems"
        :key="toast.id"
        class="toastCard"
        :class="toastClass(toast)"
        :aria-label="toastSummary(index)"
      >
        <div class="toastMeta">
          <span class="toastSource">{{ sourceLabel(toast.source) }}</span>
          <span class="toastLevel">
            <AppVisualIcon :icon="levelIcon(toast.level)" class="levelIcon" />
            <span>{{ levelLabel(toast.level) }}</span>
          </span>
        </div>
        <div class="toastBody">{{ toast.message }}</div>
        <button class="toastClose" type="button" @click="dismissToast(toast.id)">Close</button>
      </article>
    </section>

    <section
      v-if="expanded"
      :id="TASK_CENTER_PANEL_ID"
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
          <div class="postureSummary mono">{{ postureSummary }}</div>
        </div>
        <button type="button" class="panelCloseBtn" @click="closePanel">Close</button>
      </div>

      <div v-if="overall" class="overallWrap">
        <div class="overallTitle">{{ overall.label }}</div>
        <div class="bar">
          <div class="fill" :style="{ width: formatUiPercentFromFraction(overall.pct, 1) }"></div>
        </div>
      </div>

      <div class="section" aria-label="Task center active posture">
        <div class="sectionTitle">Active posture</div>
        <div class="postureColumns">
          <div class="postureColumn">
            <div class="postureHead">
              <span>Running</span>
              <span class="eventBadge running">{{ runningTasks.length }}</span>
            </div>
            <div v-if="runningTasks.length === 0" class="muted">{{ sectionEmptyLabel('running') }}</div>
            <div v-else v-motion-list class="taskList">
              <article v-for="task in runningTasks" :key="task.id" class="taskCard">
                <div class="taskRow">
                  <div class="taskName">{{ formatCommandLabel(task.command) }}</div>
                  <div class="taskStatus" :class="statusClass(task)">
                    <AppVisualIcon :icon="statusIcon(task)" class="statusIcon" :class="statusIconClass(task)" />
                    <span>{{ statusLabel(task) }}</span>
                  </div>
                </div>
                <div class="taskMeta">{{ taskSecondaryMeta(task) }}</div>
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

          <div class="postureColumn">
            <div class="postureHead">
              <span>Queued</span>
              <span class="eventBadge queued">{{ queuedTasks.length }}</span>
            </div>
            <div v-if="queuedTasks.length === 0" class="muted">{{ sectionEmptyLabel('queued') }}</div>
            <div v-else v-motion-list class="taskList">
              <article v-for="task in queuedTasks" :key="task.id" class="taskCard taskCardQueued">
                <div class="taskRow">
                  <div class="taskName">{{ formatCommandLabel(task.command) }}</div>
                  <div class="taskStatus" :class="statusClass(task)">
                    <AppVisualIcon :icon="statusIcon(task)" class="statusIcon" :class="statusIconClass(task)" />
                    <span>{{ statusLabel(task) }}</span>
                  </div>
                </div>
                <div class="taskMeta">{{ taskSecondaryMeta(task) }}</div>
                <div class="taskRow foot">
                  <div class="time">{{ absoluteTime(task.startedAt) }}</div>
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

            <div class="postureHead postureHeadSecondary">
              <span>Cancelling</span>
              <span class="eventBadge cancelling">{{ cancellingTasks.length }}</span>
            </div>
            <div v-if="cancellingTasks.length === 0" class="muted">{{ sectionEmptyLabel('cancelling') }}</div>
            <div v-else v-motion-list class="taskList">
              <article v-for="task in cancellingTasks" :key="task.id" class="taskCard taskCardCancelling">
                <div class="taskRow">
                  <div class="taskName">{{ formatCommandLabel(task.command) }}</div>
                  <div class="taskStatus" :class="statusClass(task)">
                    <AppVisualIcon :icon="statusIcon(task)" class="statusIcon" :class="statusIconClass(task)" />
                    <span>{{ statusLabel(task) }}</span>
                  </div>
                </div>
                <div class="taskMeta">{{ taskSecondaryMeta(task) }}</div>
                <div class="time">{{ absoluteTime(task.startedAt) }} · {{ durationText(task) }}</div>
              </article>
            </div>
          </div>
        </div>
      </div>

      <div class="section" aria-label="Task center recent outcomes">
        <div class="sectionTitle">Recent outcomes</div>
        <div class="postureColumns">
          <div class="postureColumn">
            <div class="postureHead">
              <span>Attention</span>
              <span class="eventBadge warn">{{ attentionTasks.length }}</span>
            </div>
            <div v-if="attentionTasks.length === 0" class="muted">{{ sectionEmptyLabel('attention') }}</div>
            <div v-else v-motion-list class="taskList">
              <article
                v-for="row in attentionTaskRows"
                :key="row.task.id"
                class="taskCard"
              >
                <div class="taskRow">
                  <div class="taskName">{{ formatCommandLabel(row.task.command) }}</div>
                  <div class="taskStatus" :class="statusClass(row.task)">
                    <AppVisualIcon :icon="statusIcon(row.task)" class="statusIcon" :class="statusIconClass(row.task)" />
                    <span>{{ statusLabel(row.task) }}</span>
                  </div>
                </div>
                <div class="taskMeta">{{ taskSecondaryMeta(row.task) }}</div>
                <div class="taskRow foot">
                  <div class="time">{{ absoluteTime(row.task.endedAt || row.task.startedAt) }}</div>
                  <button
                    v-if="row.action"
                    type="button"
                    class="relatedBtn"
                    @click="openRelatedAction(row.action)"
                  >
                    {{ row.action.label }}
                  </button>
                </div>
              </article>
            </div>
          </div>

          <div class="postureColumn">
            <div class="postureHead">
              <span>Completed</span>
              <span class="eventBadge success">{{ completedTasks.length }}</span>
            </div>
            <div v-if="completedTasks.length === 0" class="muted">{{ sectionEmptyLabel('completed') }}</div>
            <div v-else v-motion-list class="taskList">
              <article v-for="row in completedTaskRows" :key="row.task.id" class="taskCard taskCardSuccess">
                <div class="taskRow">
                  <div class="taskName">{{ formatCommandLabel(row.task.command) }}</div>
                  <div class="taskStatus" :class="statusClass(row.task)">
                    <AppVisualIcon :icon="statusIcon(row.task)" class="statusIcon" :class="statusIconClass(row.task)" />
                    <span>{{ statusLabel(row.task) }}</span>
                  </div>
                </div>
                <div class="taskMeta">{{ taskSecondaryMeta(row.task) }}</div>
                <div class="taskRow foot">
                  <div class="time">{{ absoluteTime(row.task.endedAt || row.task.startedAt) }}</div>
                  <button
                    v-if="row.action"
                    type="button"
                    class="relatedBtn"
                    @click="openRelatedAction(row.action)"
                  >
                    {{ row.action.label }}
                  </button>
                </div>
              </article>
            </div>
          </div>
        </div>
      </div>

      <div class="section" aria-label="Task center events">
        <div class="sectionTitle">Events</div>
        <div v-if="eventItems.length === 0" class="muted">No events yet.</div>
        <div v-else v-motion-list class="miniList">
          <div
            v-for="(evt, index) in eventItems"
            :key="evt.id"
            class="eventRow"
            :aria-label="eventSummary(index)"
          >
            <div class="eventMeta">
              <span class="eventTime">{{ absoluteTime(evt.atUnixMs) }}</span>
              <span class="eventBadge" :class="toastClass(evt)">
                <AppVisualIcon :icon="levelIcon(evt.level)" class="levelIcon" />
                <span>{{ levelLabel(evt.level) }}</span>
              </span>
              <span class="eventSource">{{ sourceLabel(evt.source) }}</span>
            </div>
            <span class="eventText">{{ evt.message }}</span>
          </div>
        </div>
      </div>

      <div class="section" aria-label="Task center timeline">
        <div class="sectionTitle">Timeline</div>
        <div v-if="timelineItems.length === 0" class="muted">No timeline entries yet.</div>
        <div v-else v-motion-list class="miniList">
          <div
            v-for="(entry, index) in timelineItems"
            :key="entry.id"
            class="eventRow"
            :aria-label="timelineEntrySummary(index)"
          >
            <div class="eventMeta">
              <span class="eventTime">{{ absoluteTime(entry.atUnixMs) }}</span>
              <span class="eventBadge" :class="toastClass(entry)">
                <AppVisualIcon :icon="levelIcon(entry.level)" class="levelIcon" />
                <span>{{ levelLabel(entry.level) }}</span>
              </span>
              <span class="eventBadge">{{ phaseLabel(entry.phase) }}</span>
              <span class="eventSource">{{ sourceLabel(entry.source) }}</span>
            </div>
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
  top: calc(var(--desktop-titlebar-h, 0px) + 12px);
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
}

.overlayToggle.quiet:hover {
  opacity: 1;
  transform: translateY(-1px);
  box-shadow: var(--shadow-inline);
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

.toastMeta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.toastSource {
  font-size: 0.69rem;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.toastLevel {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: 1px solid currentColor;
  border-radius: 999px;
  padding: 1px 6px;
  font-size: 0.64rem;
  line-height: 1.1;
  text-transform: uppercase;
  letter-spacing: 0.04em;
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

.postureSummary {
  font-size: 0.72rem;
  line-height: 1.35;
  color: var(--task-muted);
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

.postureColumns {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.postureColumn {
  display: grid;
  gap: 8px;
  min-width: 0;
}

.postureHead {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-size: 0.74rem;
  font-weight: 700;
  letter-spacing: 0.03em;
  color: var(--task-muted);
}

.postureHeadSecondary {
  margin-top: 4px;
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

.taskCardQueued {
  background: color-mix(in oklab, var(--surface-inline) 92%, var(--panel));
}

.taskCardCancelling {
  background: color-mix(in oklab, var(--surface-warning) 78%, var(--panel));
}

.taskCardSuccess {
  background: color-mix(in oklab, var(--surface-success) 84%, var(--panel));
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
  display: inline-flex;
  align-items: center;
  gap: 5px;
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

.relatedBtn {
  border: 1px solid color-mix(in oklab, var(--accent) 38%, var(--border));
  border-radius: 8px;
  padding: 3px 8px;
  font-size: 0.72rem;
  color: var(--selection-text);
  background: color-mix(in oklab, var(--selection-fill) 86%, var(--surface-inline));
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
  gap: 4px;
  align-items: start;
}

.miniRow {
  grid-template-columns: 1fr auto;
}

.eventRow {
  grid-template-columns: 1fr;
}

.eventMeta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
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

.eventTime {
  color: var(--task-muted-soft);
}

.eventBadge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  justify-self: start;
  border: 1px solid color-mix(in oklab, var(--border-strong) 76%, transparent);
  border-radius: 999px;
  padding: 1px 6px;
  font-size: 0.65rem;
  line-height: 1.15;
  background: color-mix(in oklab, var(--surface-inline) 88%, transparent);
  color: var(--task-muted);
}

.statusIcon,
.levelIcon {
  width: 0.9rem;
  height: 0.9rem;
  flex: 0 0 auto;
}

.statusIcon.running {
  animation: taskStatusSpin 1.1s linear infinite;
}

@keyframes taskStatusSpin {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .statusIcon.running {
    animation: none;
  }
}

.eventBadge.info,
.eventBadge.running {
  border-color: color-mix(in oklab, var(--accent) 34%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 84%, var(--surface-inline));
  color: var(--selection-text);
}

.eventBadge.warn,
.eventBadge.cancelling,
.eventBadge.cancelled {
  border-color: color-mix(in oklab, var(--warning) 36%, var(--border));
  background: color-mix(in oklab, var(--warning-soft) 88%, var(--surface-inline));
  color: var(--warning-ink);
}

.eventBadge.error,
.eventBadge.failed {
  border-color: color-mix(in oklab, var(--danger) 36%, var(--border));
  background: var(--surface-danger);
  color: var(--danger-ink);
}

.eventBadge.success {
  border-color: color-mix(in oklab, var(--ok) 36%, var(--border));
  background: var(--surface-success);
  color: var(--ok-ink);
}

.eventSource {
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--task-text);
}

.eventText {
  color: var(--task-muted);
  overflow: hidden;
  text-overflow: ellipsis;
}

@media (max-width: 760px) {
  .taskOverlayRoot {
    top: calc(var(--desktop-titlebar-h, 0px) + 8px);
    right: 8px;
  }

  .taskPanel {
    width: min(96vw, 420px);
  }

  .postureColumns {
    grid-template-columns: 1fr;
  }
}
</style>
