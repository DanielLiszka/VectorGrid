import {
  createPinia,
  defineStore,
  getActivePinia,
  setActivePinia,
  type Pinia,
} from "pinia";
import { useDebounceFn } from "@vueuse/core";
import { isTauriCommand, type TauriCommand } from "../tauri/commands";
import {
  TASK_TIMELINE_STORAGE_KEY,
  readStorageJson,
  writeStorageJson,
} from "../utils/storage";

export type TaskStatus =
  | "queued"
  | "running"
  | "cancelling"
  | "success"
  | "failed"
  | "cancelled";
export type TaskCenterStatus =
  | "idle"
  | "queued"
  | "running"
  | "cancelling"
  | "success"
  | "failed"
  | "cancelled";

export interface TrackedTask {
  id: string;
  command: string;
  label: string;
  status: TaskStatus;
  startedAt: number;
  endedAt?: number;
  progressPct?: number;
  phase?: string;
  message?: string;
  error?: string;
}

export interface TaskEventItem {
  id: string;
  atUnixMs: number;
  source: string;
  message: string;
  level: "info" | "warn" | "error";
}

export interface TaskTimelineItem {
  id: string;
  atUnixMs: number;
  source: string;
  phase: string;
  message: string;
  level: "info" | "warn" | "error" | "success";
  taskId?: string;
}

export interface TaskToastItem {
  id: string;
  atUnixMs: number;
  source: string;
  message: string;
  level: "info" | "success" | "warn" | "error";
}

export interface OverallProgress {
  pct: number;
  label: string;
  source: string;
  atUnixMs: number;
}

export interface DoubleMaProgressPayload {
  processed_pairs: number;
  total_pairs: number;
  phase: string;
}

export interface LiveExecutionRunnerEventPayload {
  status: string;
  message: string;
  symbol: string;
  at_unix_ms: number;
  bar_open_unix_ms?: number;
}

export interface PolygonLiveStatusEventPayload {
  status: string;
  message: string;
  market: string;
  symbol: string;
  delayed: boolean;
  channel: string;
  at_unix_ms: number;
}

export interface PolygonLiveBarEventPayload {
  symbol: string;
  at_unix_ms: number;
  close: number;
}

interface TaskState {
  activeById: Record<string, TrackedTask>;
  activeOrder: string[];
  history: TrackedTask[];
  events: TaskEventItem[];
  timeline: TaskTimelineItem[];
  toasts: TaskToastItem[];
  centerStatus: TaskCenterStatus;
  overallProgress: OverallProgress | null;
}

interface PersistedTaskCenterV1 {
  version: 1;
  history: TrackedTask[];
  events: TaskEventItem[];
  timeline: TaskTimelineItem[];
}

const MAX_HISTORY = 80;
const MAX_EVENTS = 160;
const MAX_TIMELINE = 300;
const MAX_TOASTS = 8;
const PROGRESS_UPDATE_INTERVAL_MS = 120;
const POLYGON_BAR_EVENT_INTERVAL_MS = 1000;
const RUNNER_EVENT_INTERVAL_MS = 200;
const POLYGON_STATUS_EVENT_INTERVAL_MS = 220;
const TASK_CENTER_PERSIST_VERSION = 1;
const PERSIST_DEBOUNCE_MS = 220;

let nextTaskSeq = 0;
let nextEventSeq = 0;
let nextTimelineSeq = 0;
let nextToastSeq = 0;
let lastDoubleMaPhase = "";
let lastDoubleMaProgressAt = 0;
let lastPolygonLiveBarAt = 0;
let lastRunnerEventAt = 0;
let lastRunnerEventSignature = "";
let lastPolygonStatusAt = 0;
let lastPolygonStatusSignature = "";
let hydratedFromStorage = false;
const commandLabelCache = new Map<string, string>();

const PROGRESS_ELIGIBLE_COMMANDS = new Set<TauriCommand>([
  "run_double_ma_optimization",
  "run_composite_cpu_optimization",
  "run_composite_mo_optimization",
  "run_double_ma_validation",
  "run_composite_validation",
  "run_meta_selection",
  "run_meta_stage4_experimental",
  "run_meta_stage5_ml_confidence",
  "export_ml_signal_dataset",
  "generate_ml_signal_score_cache",
  "run_ml_signal_validation",
  "run_ml_signal_policy_sweep",
  "polygon_download_1m_and_load",
  "train_ml_confidence_model",
  "apply_ml_confidence_model",
  "evaluate_ml_confidence_policy",
  "compute_double_ma_drilldown",
  "compute_composite_cpu_drilldown",
  "compute_double_ma_local_stability",
  "compute_composite_cpu_local_stability",
]);

const CANCEL_COMMAND_BY_COMMAND: Partial<Record<TauriCommand, TauriCommand>> = {
  run_double_ma_optimization: "cancel_current_run",
  run_composite_cpu_optimization: "cancel_current_run",
  run_composite_mo_optimization: "cancel_current_run",
  run_double_ma_validation: "cancel_current_run",
  run_composite_validation: "cancel_current_run",
  run_meta_selection: "cancel_current_run",
  run_meta_stage4_experimental: "cancel_current_run",
  run_meta_stage5_ml_confidence: "cancel_current_run",
  export_ml_signal_dataset: "cancel_current_run",
  generate_ml_signal_score_cache: "cancel_current_run",
  run_ml_signal_validation: "cancel_current_run",
  run_ml_signal_policy_sweep: "cancel_current_run",
  compute_double_ma_drilldown: "cancel_current_run",
  compute_composite_cpu_drilldown: "cancel_current_run",
  compute_double_ma_local_stability: "cancel_current_run",
  compute_composite_cpu_local_stability: "cancel_current_run",
  train_ml_confidence_model: "cancel_current_run",
  apply_ml_confidence_model: "cancel_current_run",
  evaluate_ml_confidence_policy: "cancel_current_run",
  polygon_download_1m_and_load: "cancel_background_task",
};

export const useTaskStore = defineStore("task", {
  state: (): TaskState => ({
    activeById: {},
    activeOrder: [],
    history: [],
    events: [],
    timeline: [],
    toasts: [],
    centerStatus: "idle",
    overallProgress: null,
  }),
});

function ensurePiniaActive(): Pinia {
  const active = getActivePinia();
  if (active) return active;
  const created = createPinia();
  setActivePinia(created);
  return created;
}

function clampHistory(items: TrackedTask[]): TrackedTask[] {
  return items.slice(0, MAX_HISTORY);
}

function clampEvents(items: TaskEventItem[]): TaskEventItem[] {
  return items.slice(0, MAX_EVENTS);
}

function clampTimeline(items: TaskTimelineItem[]): TaskTimelineItem[] {
  return items.slice(0, MAX_TIMELINE);
}

function hydrateStoreFromDisk(st: TaskState): void {
  if (hydratedFromStorage) return;
  hydratedFromStorage = true;
  const payload = readStorageJson<PersistedTaskCenterV1>(
    TASK_TIMELINE_STORAGE_KEY,
  );
  if (!payload || payload.version !== 1) return;

  st.history = clampHistory(
    Array.isArray(payload.history) ? payload.history : [],
  );
  st.events = clampEvents(Array.isArray(payload.events) ? payload.events : []);
  st.timeline = clampTimeline(
    Array.isArray(payload.timeline) ? payload.timeline : [],
  );
}

function taskStoreSingleton() {
  const pinia = ensurePiniaActive();
  const st = useTaskStore(pinia);
  hydrateStoreFromDisk(st);
  return st;
}

function removeActiveTaskId(st: TaskState, id: string): void {
  const index = st.activeOrder.indexOf(id);
  if (index >= 0) st.activeOrder.splice(index, 1);
}

function persistTaskCenterNow(): void {
  const st = taskStoreSingleton();
  const payload: PersistedTaskCenterV1 = {
    version: TASK_CENTER_PERSIST_VERSION,
    history: clampHistory(st.history),
    events: clampEvents(st.events),
    timeline: clampTimeline(st.timeline),
  };
  writeStorageJson(TASK_TIMELINE_STORAGE_KEY, payload);
}

const persistTaskCenterDebounced = useDebounceFn(
  () => {
    persistTaskCenterNow();
  },
  PERSIST_DEBOUNCE_MS,
  { maxWait: PERSIST_DEBOUNCE_MS * 3 },
);

function schedulePersistTaskCenter(): void {
  persistTaskCenterDebounced();
}

function refreshCenterStatus(): void {
  const st = taskStoreSingleton();
  let hasCancelling = false;
  let hasRunning = false;
  let hasQueued = false;
  for (const id of st.activeOrder) {
    const task = st.activeById[id];
    if (!task) continue;
    if (task.status === "cancelling") hasCancelling = true;
    else if (task.status === "running") hasRunning = true;
    else if (task.status === "queued") hasQueued = true;
    if (hasCancelling) break;
  }
  if (hasCancelling) {
    st.centerStatus = "cancelling";
    return;
  }
  if (hasRunning) {
    st.centerStatus = "running";
    return;
  }
  if (hasQueued) {
    st.centerStatus = "queued";
    return;
  }

  const last = st.history[0];
  if (!last) {
    st.centerStatus = "idle";
    return;
  }

  if (last.status === "success") st.centerStatus = "success";
  else if (last.status === "cancelled") st.centerStatus = "cancelled";
  else if (last.status === "failed") st.centerStatus = "failed";
  else st.centerStatus = "idle";
}

function pushTimeline(
  source: string,
  phase: string,
  message: string,
  level: TaskTimelineItem["level"] = "info",
  taskId?: string,
  atUnixMs = Date.now(),
): void {
  const st = taskStoreSingleton();
  nextTimelineSeq += 1;
  const item: TaskTimelineItem = {
    id: `timeline_${atUnixMs}_${nextTimelineSeq}`,
    atUnixMs,
    source,
    phase,
    message,
    level,
    ...(taskId ? { taskId } : {}),
  };
  st.timeline.unshift(item);
  if (st.timeline.length > MAX_TIMELINE) {
    st.timeline.splice(MAX_TIMELINE);
  }
  schedulePersistTaskCenter();
}

function pushToast(
  source: string,
  message: string,
  level: TaskToastItem["level"],
): void {
  const st = taskStoreSingleton();
  const atUnixMs = Date.now();
  nextToastSeq += 1;
  st.toasts.unshift({
    id: `toast_${atUnixMs}_${nextToastSeq}`,
    atUnixMs,
    source,
    message,
    level,
  });
  if (st.toasts.length > MAX_TOASTS) {
    st.toasts.splice(MAX_TOASTS);
  }
}

function pushHistory(task: TrackedTask): void {
  const st = taskStoreSingleton();
  st.history.unshift(task);
  if (st.history.length > MAX_HISTORY) {
    st.history.splice(MAX_HISTORY);
  }
  schedulePersistTaskCenter();
}

function pushEvent(
  source: string,
  message: string,
  level: TaskEventItem["level"] = "info",
  atUnixMs = Date.now(),
): void {
  const st = taskStoreSingleton();
  nextEventSeq += 1;
  st.events.unshift({
    id: `event_${atUnixMs}_${nextEventSeq}`,
    atUnixMs,
    source,
    message,
    level,
  });
  if (st.events.length > MAX_EVENTS) {
    st.events.splice(MAX_EVENTS);
  }
  schedulePersistTaskCenter();
}

function isLikelyCancelledMessage(message: string): boolean {
  const lower = message.toLowerCase();
  return lower.includes("cancel") || lower.includes("aborted");
}

function newestProgressTask(): TrackedTask | null {
  const st = taskStoreSingleton();
  let fallback: TrackedTask | null = null;
  for (const id of st.activeOrder) {
    const task = st.activeById[id];
    if (!task) continue;
    if (
      task.status !== "running" &&
      task.status !== "cancelling" &&
      task.status !== "queued"
    )
      continue;
    if (!fallback) fallback = task;
    if (
      isTauriCommand(task.command) &&
      PROGRESS_ELIGIBLE_COMMANDS.has(task.command)
    ) {
      return task;
    }
  }
  return fallback;
}

function setTaskRunning(id: string): void {
  const st = taskStoreSingleton();
  const task = st.activeById[id];
  if (!task || task.status !== "queued") return;
  task.status = "running";
  refreshCenterStatus();
}

export function dismissToast(id: string): void {
  const st = taskStoreSingleton();
  const targetId = String(id || "").trim();
  if (!targetId) return;
  const index = st.toasts.findIndex((toast) => toast.id === targetId);
  if (index >= 0) st.toasts.splice(index, 1);
}

export function markTaskCancelling(id: string): void {
  const st = taskStoreSingleton();
  const task = st.activeById[id];
  if (!task) return;
  if (task.status === "cancelling" || task.status === "cancelled") return;
  task.status = "cancelling";
  task.message = "Cancellation requested.";
  pushTimeline(
    task.command,
    "cancelling",
    `${task.label}: cancellation requested`,
    "warn",
    task.id,
  );
  refreshCenterStatus();
}

export function restoreTaskRunning(id: string): void {
  const st = taskStoreSingleton();
  const task = st.activeById[id];
  if (!task || task.status !== "cancelling") return;
  task.status = "running";
  task.message = "Cancellation failed. Task still running.";
  pushTimeline(
    task.command,
    "running",
    `${task.label}: cancellation request failed`,
    "warn",
    task.id,
  );
  refreshCenterStatus();
}

export function formatCommandLabel(command: string): string {
  const key = String(command || "");
  const cached = commandLabelCache.get(key);
  if (cached) return cached;
  const label =
    key === "preview_dynamic_double_ma_strategy_ir_json"
      ? "Preview Dynamic Double MA Strategy IR"
      : key === "encode_dynamic_strategy_constant_bits"
        ? "Encode Dynamic Strategy Constant"
        : key === "format_dynamic_strategy_ir_json"
          ? "Format Dynamic Strategy IR JSON"
          : key
              .replace(/^run_/, "")
              .replace(/^compute_/, "")
              .replace(/^list_/, "")
              .replace(/^get_/, "")
              .replace(/_/g, " ")
              .replace(/\b[a-z]/g, (s) => s.toUpperCase());
  if (commandLabelCache.size > 256) commandLabelCache.clear();
  commandLabelCache.set(key, label);
  return label;
}

export function activeTasksSnapshot(): TrackedTask[] {
  const st = taskStoreSingleton();
  const ordered: TrackedTask[] = [];
  for (const id of st.activeOrder) {
    const task = st.activeById[id];
    if (!task) continue;
    ordered.push(task);
  }
  if (ordered.length === st.activeOrder.length) return ordered;

  const seen = new Set<string>();
  for (const task of ordered) seen.add(task.id);
  for (const [id, task] of Object.entries(st.activeById)) {
    if (seen.has(id)) continue;
    ordered.push(task);
  }
  ordered.sort((a, b) => b.startedAt - a.startedAt);
  st.activeOrder = ordered.map((task) => task.id);
  return ordered;
}

export function activeTasksCount(): number {
  const st = taskStoreSingleton();
  let count = 0;
  for (const id of st.activeOrder) {
    if (st.activeById[id]) count += 1;
  }
  return count;
}

export function cancelCommandFor(command: string): TauriCommand | null {
  if (!isTauriCommand(command)) return null;
  return CANCEL_COMMAND_BY_COMMAND[command] ?? null;
}

export function beginTrackedCommand(
  command: string,
  opts?: { queued?: boolean },
): string {
  const st = taskStoreSingleton();
  const now = Date.now();
  nextTaskSeq += 1;
  const id = `task_${now}_${nextTaskSeq}`;
  const label = formatCommandLabel(command);
  const status: TaskStatus = opts?.queued ? "queued" : "running";
  const task: TrackedTask = {
    id,
    command,
    label,
    status,
    startedAt: now,
    ...(status === "queued" ? { message: "Queued." } : {}),
  };
  st.activeById[id] = task;
  st.activeOrder.unshift(id);

  pushTimeline(
    command,
    status,
    `${label} ${status === "queued" ? "queued" : "started"}.`,
    "info",
    id,
    now,
  );
  refreshCenterStatus();

  if (status === "queued") {
    Promise.resolve().then(() => setTaskRunning(id));
  }
  return id;
}

export function completeTrackedCommand(id: string): void {
  const st = taskStoreSingleton();
  const task = st.activeById[id];
  if (!task) return;
  const done: TrackedTask = {
    ...task,
    status: "success",
    endedAt: Date.now(),
    message: task.message ?? "Completed.",
  };
  delete st.activeById[id];
  removeActiveTaskId(st, id);
  pushHistory(done);
  pushEvent(task.command, `${task.label} completed.`, "info");
  pushTimeline(
    task.command,
    "success",
    `${task.label} completed.`,
    "success",
    task.id,
  );
  pushToast(task.command, `${task.label} completed.`, "success");
  refreshCenterStatus();
}

export function failTrackedCommand(id: string, err: unknown): void {
  const st = taskStoreSingleton();
  const task = st.activeById[id];
  if (!task) return;
  const errorText = String(
    err instanceof Error ? err.message : (err ?? "Unknown error"),
  ).trim();
  const cancelled = isLikelyCancelledMessage(errorText);
  const done: TrackedTask = {
    ...task,
    status: cancelled ? "cancelled" : "failed",
    endedAt: Date.now(),
    error: errorText || "Unknown error",
    ...(cancelled ? { message: "Cancelled." } : {}),
  };
  delete st.activeById[id];
  removeActiveTaskId(st, id);
  pushHistory(done);
  pushEvent(
    task.command,
    `${task.label} ${cancelled ? "cancelled" : "failed"}: ${done.error}`,
    cancelled ? "warn" : "error",
  );
  pushTimeline(
    task.command,
    cancelled ? "cancelled" : "failed",
    `${task.label} ${cancelled ? "cancelled" : "failed"}${done.error ? `: ${done.error}` : ""}`,
    cancelled ? "warn" : "error",
    task.id,
  );
  pushToast(
    task.command,
    `${task.label} ${cancelled ? "cancelled" : "failed"}.`,
    cancelled ? "warn" : "error",
  );
  refreshCenterStatus();
}

export function applyDoubleMaProgress(payload: DoubleMaProgressPayload): void {
  const st = taskStoreSingleton();
  const processed = Number(payload.processed_pairs || 0);
  const total = Number(payload.total_pairs || 0);
  const phase = String(payload.phase || "").trim();
  const pct =
    total > 0 ? Math.max(0, Math.min(100, (processed / total) * 100)) : 0;
  const now = Date.now();
  const phaseChanged = !!phase && phase !== lastDoubleMaPhase;
  if (phaseChanged) {
    lastDoubleMaPhase = phase;
    pushEvent("double_ma_progress", `Phase: ${phase}`);
    pushTimeline(
      "double_ma_progress",
      "phase",
      `Phase changed: ${phase}`,
      "info",
    );
  }

  const forceRefresh = phaseChanged || pct <= 0 || pct >= 100;
  if (
    !forceRefresh &&
    now - lastDoubleMaProgressAt < PROGRESS_UPDATE_INTERVAL_MS
  )
    return;
  lastDoubleMaProgressAt = now;

  const target = newestProgressTask();
  if (target) {
    target.progressPct = pct;
    if (phase) target.phase = phase;
    target.message = `${phase || "Progress"} ${processed}/${total}`;
  }
  st.overallProgress = {
    pct,
    label: `${phase || "Progress"} ${processed}/${total}`,
    source: "double_ma_progress",
    atUnixMs: now,
  };
}

export function recordLiveExecutionRunnerEvent(
  payload: LiveExecutionRunnerEventPayload,
): void {
  const now = Date.now();
  const status = String(payload.status || "").trim();
  const symbol = String(payload.symbol || "").trim();
  const message = String(payload.message || "").trim();
  const signature = `${symbol}|${status}|${message}`;
  if (
    signature === lastRunnerEventSignature &&
    now - lastRunnerEventAt < RUNNER_EVENT_INTERVAL_MS
  )
    return;
  lastRunnerEventAt = now;
  lastRunnerEventSignature = signature;

  const msg = [symbol || "symbol", status || "status", message]
    .join(" ")
    .trim();
  pushEvent(
    "live_execution_runner_event",
    msg,
    "info",
    Number(payload.at_unix_ms || now),
  );
}

export function recordPolygonLiveStatusEvent(
  payload: PolygonLiveStatusEventPayload,
): void {
  const now = Date.now();
  const status = String(payload.status || "").trim();
  const market = String(payload.market || "").trim();
  const symbol = String(payload.symbol || "").trim();
  const message = String(payload.message || "").trim();
  const signature = `${market}|${symbol}|${status}|${message}`;
  if (
    signature === lastPolygonStatusSignature &&
    now - lastPolygonStatusAt < POLYGON_STATUS_EVENT_INTERVAL_MS
  )
    return;
  lastPolygonStatusAt = now;
  lastPolygonStatusSignature = signature;

  const msg = [
    market || "market",
    symbol || "symbol",
    status || "status",
    message,
  ]
    .join(" ")
    .trim();
  const level: TaskEventItem["level"] = status === "error" ? "error" : "info";
  pushEvent(
    "polygon_live_status",
    msg,
    level,
    Number(payload.at_unix_ms || now),
  );
}

export function recordPolygonLiveBarEvent(
  payload: PolygonLiveBarEventPayload,
): void {
  const now = Date.now();
  if (now - lastPolygonLiveBarAt < POLYGON_BAR_EVENT_INTERVAL_MS) return;
  lastPolygonLiveBarAt = now;

  const closeValue = Number(payload.close);
  const closeText = Number.isFinite(closeValue) ? closeValue.toFixed(4) : "-";
  const symbol = String(payload.symbol || "").trim() || "symbol";
  const atUnixMs = Number(payload.at_unix_ms || now);
  pushEvent(
    "polygon_live_bar",
    `${symbol} close=${closeText}`,
    "info",
    atUnixMs,
  );
}
