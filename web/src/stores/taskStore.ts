import { createPinia, defineStore, getActivePinia, setActivePinia, type Pinia } from 'pinia'
import { isTauriCommand, type TauriCommand } from '../tauri/commands'
import { TASK_TIMELINE_STORAGE_KEY, readStorageJson, writeStorageJson } from '../utils/storage'

export type TaskStatus = 'queued' | 'running' | 'cancelling' | 'success' | 'failed' | 'cancelled'
export type TaskCenterStatus = 'idle' | 'queued' | 'running' | 'cancelling' | 'success' | 'failed' | 'cancelled'

export interface TrackedTask {
  id: string
  command: string
  label: string
  status: TaskStatus
  startedAt: number
  endedAt?: number
  progressPct?: number
  phase?: string
  message?: string
  error?: string
}

export interface TaskEventItem {
  id: string
  atUnixMs: number
  source: string
  message: string
  level: 'info' | 'warn' | 'error'
}

export interface TaskTimelineItem {
  id: string
  atUnixMs: number
  source: string
  phase: string
  message: string
  level: 'info' | 'warn' | 'error' | 'success'
  taskId?: string
}

export interface TaskToastItem {
  id: string
  atUnixMs: number
  source: string
  message: string
  level: 'info' | 'success' | 'warn' | 'error'
}

export interface OverallProgress {
  pct: number
  label: string
  source: string
  atUnixMs: number
}

export interface DoubleMaProgressPayload {
  processed_pairs: number
  total_pairs: number
  phase: string
}

export interface DistributedProgressPayload {
  mode: string
  completed: number
  total: number
  successes: number
  failures: number
  last?: string
}

export interface LiveExecutionRunnerEventPayload {
  status: string
  message: string
  symbol: string
  at_unix_ms: number
  bar_open_unix_ms?: number
}

export interface PolygonLiveStatusEventPayload {
  status: string
  message: string
  market: string
  symbol: string
  delayed: boolean
  channel: string
  at_unix_ms: number
}

export interface PolygonLiveBarEventPayload {
  symbol: string
  at_unix_ms: number
  close: number
}

interface TaskState {
  activeById: Record<string, TrackedTask>
  history: TrackedTask[]
  events: TaskEventItem[]
  timeline: TaskTimelineItem[]
  toasts: TaskToastItem[]
  centerStatus: TaskCenterStatus
  overallProgress: OverallProgress | null
}

interface PersistedTaskCenterV1 {
  version: 1
  history: TrackedTask[]
  events: TaskEventItem[]
  timeline: TaskTimelineItem[]
}

const MAX_HISTORY = 80
const MAX_EVENTS = 160
const MAX_TIMELINE = 300
const MAX_TOASTS = 8
const PROGRESS_UPDATE_INTERVAL_MS = 120
const POLYGON_BAR_EVENT_INTERVAL_MS = 1000
const TASK_CENTER_PERSIST_VERSION = 1
const PERSIST_DEBOUNCE_MS = 220

let nextTaskSeq = 0
let nextEventSeq = 0
let nextTimelineSeq = 0
let nextToastSeq = 0
let lastDoubleMaPhase = ''
let lastDoubleMaProgressAt = 0
let lastDistributedProgressAt = 0
let lastPolygonLiveBarAt = 0
let hydratedFromStorage = false
let persistTimer: ReturnType<typeof setTimeout> | null = null

const PROGRESS_ELIGIBLE_COMMANDS = new Set<TauriCommand>([
  'run_double_ma_optimization',
  'run_composite_cpu_optimization',
  'run_composite_mo_optimization',
  'run_double_ma_validation',
  'run_composite_validation',
  'run_meta_selection',
  'run_meta_stage4_experimental',
  'distributed_run_double_ma_assets',
  'distributed_run_double_ma_tiled',
  'polygon_download_1m_and_load',
  'train_ml_confidence_model',
  'apply_ml_confidence_model',
  'compute_double_ma_drilldown',
  'compute_composite_cpu_drilldown',
  'compute_double_ma_local_stability',
  'compute_composite_cpu_local_stability',
])

const CANCEL_COMMAND_BY_COMMAND: Partial<Record<TauriCommand, TauriCommand>> = {
  run_double_ma_optimization: 'cancel_current_run',
  run_composite_cpu_optimization: 'cancel_current_run',
  run_composite_mo_optimization: 'cancel_current_run',
  run_double_ma_validation: 'cancel_current_run',
  run_composite_validation: 'cancel_current_run',
  run_meta_selection: 'cancel_current_run',
  run_meta_stage4_experimental: 'cancel_current_run',
  distributed_run_double_ma_assets: 'cancel_current_run',
  distributed_run_double_ma_tiled: 'cancel_current_run',
  compute_double_ma_drilldown: 'cancel_current_run',
  compute_composite_cpu_drilldown: 'cancel_current_run',
  compute_double_ma_local_stability: 'cancel_current_run',
  compute_composite_cpu_local_stability: 'cancel_current_run',
  train_ml_confidence_model: 'cancel_current_run',
  apply_ml_confidence_model: 'cancel_current_run',
  polygon_download_1m_and_load: 'cancel_background_task',
}

export const useTaskStore = defineStore('task', {
  state: (): TaskState => ({
    activeById: {},
    history: [],
    events: [],
    timeline: [],
    toasts: [],
    centerStatus: 'idle',
    overallProgress: null,
  }),
})

function ensurePiniaActive(): Pinia {
  const active = getActivePinia()
  if (active) return active
  const created = createPinia()
  setActivePinia(created)
  return created
}

function clampHistory(items: TrackedTask[]): TrackedTask[] {
  return items.slice(0, MAX_HISTORY)
}

function clampEvents(items: TaskEventItem[]): TaskEventItem[] {
  return items.slice(0, MAX_EVENTS)
}

function clampTimeline(items: TaskTimelineItem[]): TaskTimelineItem[] {
  return items.slice(0, MAX_TIMELINE)
}

function hydrateStoreFromDisk(st: TaskState): void {
  if (hydratedFromStorage) return
  hydratedFromStorage = true
  const payload = readStorageJson<PersistedTaskCenterV1>(TASK_TIMELINE_STORAGE_KEY)
  if (!payload || payload.version !== 1) return

  st.history = clampHistory(Array.isArray(payload.history) ? payload.history : [])
  st.events = clampEvents(Array.isArray(payload.events) ? payload.events : [])
  st.timeline = clampTimeline(Array.isArray(payload.timeline) ? payload.timeline : [])
}

function taskStoreSingleton() {
  const pinia = ensurePiniaActive()
  const st = useTaskStore(pinia)
  hydrateStoreFromDisk(st)
  return st
}

function persistTaskCenterNow(): void {
  const st = taskStoreSingleton()
  const payload: PersistedTaskCenterV1 = {
    version: TASK_CENTER_PERSIST_VERSION,
    history: clampHistory(st.history),
    events: clampEvents(st.events),
    timeline: clampTimeline(st.timeline),
  }
  writeStorageJson(TASK_TIMELINE_STORAGE_KEY, payload)
}

function schedulePersistTaskCenter(): void {
  if (persistTimer) return
  persistTimer = setTimeout(() => {
    persistTimer = null
    persistTaskCenterNow()
  }, PERSIST_DEBOUNCE_MS)
}

function refreshCenterStatus(): void {
  const st = taskStoreSingleton()
  const active = Object.values(st.activeById)
  if (active.some((task) => task.status === 'cancelling')) {
    st.centerStatus = 'cancelling'
    return
  }
  if (active.some((task) => task.status === 'running')) {
    st.centerStatus = 'running'
    return
  }
  if (active.some((task) => task.status === 'queued')) {
    st.centerStatus = 'queued'
    return
  }

  const last = st.history[0]
  if (!last) {
    st.centerStatus = 'idle'
    return
  }

  if (last.status === 'success') st.centerStatus = 'success'
  else if (last.status === 'cancelled') st.centerStatus = 'cancelled'
  else if (last.status === 'failed') st.centerStatus = 'failed'
  else st.centerStatus = 'idle'
}

function pushTimeline(
  source: string,
  phase: string,
  message: string,
  level: TaskTimelineItem['level'] = 'info',
  taskId?: string,
  atUnixMs = Date.now(),
): void {
  const st = taskStoreSingleton()
  nextTimelineSeq += 1
  const item: TaskTimelineItem = {
    id: `timeline_${atUnixMs}_${nextTimelineSeq}`,
    atUnixMs,
    source,
    phase,
    message,
    level,
    ...(taskId ? { taskId } : {}),
  }
  st.timeline.unshift(item)
  if (st.timeline.length > MAX_TIMELINE) {
    st.timeline.splice(MAX_TIMELINE)
  }
  schedulePersistTaskCenter()
}

function pushToast(source: string, message: string, level: TaskToastItem['level']): void {
  const st = taskStoreSingleton()
  const atUnixMs = Date.now()
  nextToastSeq += 1
  st.toasts.unshift({
    id: `toast_${atUnixMs}_${nextToastSeq}`,
    atUnixMs,
    source,
    message,
    level,
  })
  if (st.toasts.length > MAX_TOASTS) {
    st.toasts.splice(MAX_TOASTS)
  }
}

function pushHistory(task: TrackedTask): void {
  const st = taskStoreSingleton()
  st.history.unshift(task)
  if (st.history.length > MAX_HISTORY) {
    st.history.splice(MAX_HISTORY)
  }
  schedulePersistTaskCenter()
}

function pushEvent(source: string, message: string, level: TaskEventItem['level'] = 'info', atUnixMs = Date.now()): void {
  const st = taskStoreSingleton()
  nextEventSeq += 1
  st.events.unshift({
    id: `event_${atUnixMs}_${nextEventSeq}`,
    atUnixMs,
    source,
    message,
    level,
  })
  if (st.events.length > MAX_EVENTS) {
    st.events.splice(MAX_EVENTS)
  }
  schedulePersistTaskCenter()
}

function isLikelyCancelledMessage(message: string): boolean {
  const lower = message.toLowerCase()
  return lower.includes('cancel') || lower.includes('aborted')
}

function newestProgressTask(): TrackedTask | null {
  const st = taskStoreSingleton()
  const all = Object.values(st.activeById)
  const ranked = all
    .filter((task) => task.status === 'running' || task.status === 'cancelling' || task.status === 'queued')
    .sort((a, b) => b.startedAt - a.startedAt)
  for (const task of ranked) {
    if (isTauriCommand(task.command) && PROGRESS_ELIGIBLE_COMMANDS.has(task.command)) {
      return task
    }
  }
  return ranked[0] ?? null
}

function setTaskRunning(id: string): void {
  const st = taskStoreSingleton()
  const task = st.activeById[id]
  if (!task || task.status !== 'queued') return
  task.status = 'running'
  refreshCenterStatus()
}

export function dismissToast(id: string): void {
  const st = taskStoreSingleton()
  st.toasts = st.toasts.filter((toast) => toast.id !== id)
}

export function markTaskCancelling(id: string): void {
  const st = taskStoreSingleton()
  const task = st.activeById[id]
  if (!task) return
  if (task.status === 'cancelling' || task.status === 'cancelled') return
  task.status = 'cancelling'
  task.message = 'Cancellation requested.'
  pushTimeline(task.command, 'cancelling', `${task.label}: cancellation requested`, 'warn', task.id)
  refreshCenterStatus()
}

export function restoreTaskRunning(id: string): void {
  const st = taskStoreSingleton()
  const task = st.activeById[id]
  if (!task || task.status !== 'cancelling') return
  task.status = 'running'
  task.message = 'Cancellation failed. Task still running.'
  pushTimeline(task.command, 'running', `${task.label}: cancellation request failed`, 'warn', task.id)
  refreshCenterStatus()
}

export function formatCommandLabel(command: string): string {
  return command
    .replace(/^run_/, '')
    .replace(/^compute_/, '')
    .replace(/^list_/, '')
    .replace(/^get_/, '')
    .replace(/_/g, ' ')
    .replace(/\b[a-z]/g, (s) => s.toUpperCase())
}

export function activeTasksSnapshot(): TrackedTask[] {
  const st = taskStoreSingleton()
  return Object.values(st.activeById).sort((a, b) => b.startedAt - a.startedAt)
}

export function cancelCommandFor(command: string): TauriCommand | null {
  if (!isTauriCommand(command)) return null
  return CANCEL_COMMAND_BY_COMMAND[command] ?? null
}

export function beginTrackedCommand(command: string, opts?: { queued?: boolean }): string {
  const st = taskStoreSingleton()
  const now = Date.now()
  nextTaskSeq += 1
  const id = `task_${now}_${nextTaskSeq}`
  const label = formatCommandLabel(command)
  const status: TaskStatus = opts?.queued ? 'queued' : 'running'
  const task: TrackedTask = {
    id,
    command,
    label,
    status,
    startedAt: now,
    ...(status === 'queued' ? { message: 'Queued.' } : {}),
  }
  st.activeById[id] = task

  pushTimeline(command, status, `${label} ${status === 'queued' ? 'queued' : 'started'}.`, 'info', id, now)
  refreshCenterStatus()

  if (status === 'queued') {
    Promise.resolve().then(() => setTaskRunning(id))
  }
  return id
}

export function completeTrackedCommand(id: string): void {
  const st = taskStoreSingleton()
  const task = st.activeById[id]
  if (!task) return
  const done: TrackedTask = {
    ...task,
    status: 'success',
    endedAt: Date.now(),
    message: task.message ?? 'Completed.',
  }
  delete st.activeById[id]
  pushHistory(done)
  pushEvent(task.command, `${task.label} completed.`, 'info')
  pushTimeline(task.command, 'success', `${task.label} completed.`, 'success', task.id)
  pushToast(task.command, `${task.label} completed.`, 'success')
  refreshCenterStatus()
}

export function failTrackedCommand(id: string, err: unknown): void {
  const st = taskStoreSingleton()
  const task = st.activeById[id]
  if (!task) return
  const errorText = String(err instanceof Error ? err.message : err ?? 'Unknown error').trim()
  const cancelled = isLikelyCancelledMessage(errorText)
  const done: TrackedTask = {
    ...task,
    status: cancelled ? 'cancelled' : 'failed',
    endedAt: Date.now(),
    error: errorText || 'Unknown error',
    ...(cancelled ? { message: 'Cancelled.' } : {}),
  }
  delete st.activeById[id]
  pushHistory(done)
  pushEvent(
    task.command,
    `${task.label} ${cancelled ? 'cancelled' : 'failed'}: ${done.error}`,
    cancelled ? 'warn' : 'error',
  )
  pushTimeline(
    task.command,
    cancelled ? 'cancelled' : 'failed',
    `${task.label} ${cancelled ? 'cancelled' : 'failed'}${done.error ? `: ${done.error}` : ''}`,
    cancelled ? 'warn' : 'error',
    task.id,
  )
  pushToast(task.command, `${task.label} ${cancelled ? 'cancelled' : 'failed'}.`, cancelled ? 'warn' : 'error')
  refreshCenterStatus()
}

export function applyDoubleMaProgress(payload: DoubleMaProgressPayload): void {
  const st = taskStoreSingleton()
  const processed = Number(payload.processed_pairs || 0)
  const total = Number(payload.total_pairs || 0)
  const phase = String(payload.phase || '').trim()
  const pct = total > 0 ? Math.max(0, Math.min(100, (processed / total) * 100)) : 0
  const now = Date.now()
  const phaseChanged = !!phase && phase !== lastDoubleMaPhase
  if (phaseChanged) {
    lastDoubleMaPhase = phase
    pushEvent('double_ma_progress', `Phase: ${phase}`)
    pushTimeline('double_ma_progress', 'phase', `Phase changed: ${phase}`, 'info')
  }

  const forceRefresh = phaseChanged || pct <= 0 || pct >= 100
  if (!forceRefresh && now - lastDoubleMaProgressAt < PROGRESS_UPDATE_INTERVAL_MS) return
  lastDoubleMaProgressAt = now

  const target = newestProgressTask()
  if (target) {
    target.progressPct = pct
    if (phase) target.phase = phase
    target.message = `${phase || 'Progress'} ${processed}/${total}`
  }
  st.overallProgress = {
    pct,
    label: `${phase || 'Progress'} ${processed}/${total}`,
    source: 'double_ma_progress',
    atUnixMs: now,
  }
}

export function applyDistributedProgress(payload: DistributedProgressPayload): void {
  const st = taskStoreSingleton()
  const completed = Number(payload.completed || 0)
  const total = Number(payload.total || 0)
  const pct = total > 0 ? Math.max(0, Math.min(100, (completed / total) * 100)) : 0
  const now = Date.now()
  const forceRefresh = completed <= 0 || (total > 0 && completed >= total)
  if (!forceRefresh && now - lastDistributedProgressAt < PROGRESS_UPDATE_INTERVAL_MS) return
  lastDistributedProgressAt = now

  const detail = [
    `Mode=${payload.mode || 'distributed'}`,
    `${completed}/${total}`,
    `ok=${Number(payload.successes || 0)}`,
    `fail=${Number(payload.failures || 0)}`,
  ].join(' ')
  const target = newestProgressTask()
  if (target) {
    target.progressPct = pct
    target.phase = `distributed:${payload.mode || 'run'}`
    target.message = detail
  }
  st.overallProgress = {
    pct,
    label: detail,
    source: 'distributed_progress',
    atUnixMs: now,
  }
}

export function recordLiveExecutionRunnerEvent(payload: LiveExecutionRunnerEventPayload): void {
  const msg = [
    payload.symbol || 'symbol',
    payload.status || 'status',
    payload.message || '',
  ]
    .join(' ')
    .trim()
  pushEvent('live_execution_runner_event', msg, 'info', Number(payload.at_unix_ms || Date.now()))
}

export function recordPolygonLiveStatusEvent(payload: PolygonLiveStatusEventPayload): void {
  const msg = [
    payload.market || 'market',
    payload.symbol || 'symbol',
    payload.status || 'status',
    payload.message || '',
  ]
    .join(' ')
    .trim()
  const level: TaskEventItem['level'] = payload.status === 'error' ? 'error' : 'info'
  pushEvent('polygon_live_status', msg, level, Number(payload.at_unix_ms || Date.now()))
}

export function recordPolygonLiveBarEvent(payload: PolygonLiveBarEventPayload): void {
  const now = Date.now()
  if (now - lastPolygonLiveBarAt < POLYGON_BAR_EVENT_INTERVAL_MS) return
  lastPolygonLiveBarAt = now

  const closeValue = Number(payload.close)
  const closeText = Number.isFinite(closeValue) ? closeValue.toFixed(4) : '-'
  const symbol = String(payload.symbol || '').trim() || 'symbol'
  const atUnixMs = Number(payload.at_unix_ms || now)
  pushEvent('polygon_live_bar', `${symbol} close=${closeText}`, 'info', atUnixMs)
}
