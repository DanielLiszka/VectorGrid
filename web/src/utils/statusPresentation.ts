import type {
  OverallProgress,
  TaskCenterStatus,
  TaskStatus,
  TaskToastItem,
  TrackedTask,
} from '../stores/taskStore'
import type { LegacyBootPhase } from '../stores/runtimeStore'
import { formatUiPercentFromFraction } from './uiFormat'

type TaskLifecycleValue = TaskStatus | TaskCenterStatus | string | null | undefined

export function taskLifecycleTone(status: TaskLifecycleValue): TaskCenterStatus | 'idle' {
  const value = String(status || 'idle').trim().toLowerCase()
  if (value === 'running') return 'running'
  if (value === 'queued') return 'queued'
  if (value === 'cancelling') return 'cancelling'
  if (value === 'success') return 'success'
  if (value === 'failed') return 'failed'
  if (value === 'cancelled') return 'cancelled'
  return 'idle'
}

export function taskLifecycleLabel(status: TaskLifecycleValue): string {
  const tone = taskLifecycleTone(status)
  if (tone === 'running') return 'Running'
  if (tone === 'queued') return 'Queued'
  if (tone === 'cancelling') return 'Cancelling'
  if (tone === 'success') return 'Completed'
  if (tone === 'failed') return 'Failed'
  if (tone === 'cancelled') return 'Cancelled'
  return 'Idle'
}

export function describeTrackedTask(task: TrackedTask | null | undefined): string {
  if (!task) return 'No active commands.'
  if (String(task.message || '').trim()) return String(task.message || '').trim()
  if (String(task.phase || '').trim() && Number.isFinite(task.progressPct)) {
    return `${String(task.phase || '').trim()} (${Number(task.progressPct || 0).toFixed(1)}%)`
  }
  if (String(task.phase || '').trim()) return String(task.phase || '').trim()
  if (String(task.error || '').trim()) return String(task.error || '').trim()
  return `${task.label} ${taskLifecycleLabel(task.status).toLowerCase()}.`
}

export function describeTaskActivity(options: {
  activeTasks?: TrackedTask[] | null
  latestToast?: TaskToastItem | null
  latestTask?: TrackedTask | null
}): string {
  const activeTask = options.activeTasks?.[0] ?? null
  if (activeTask) return describeTrackedTask(activeTask)

  const toast = options.latestToast
  if (toast && String(toast.message || '').trim()) {
    return String(toast.message || '').trim()
  }

  const latestTask = options.latestTask
  if (latestTask) {
    return `${latestTask.label} ${taskLifecycleLabel(latestTask.status).toLowerCase()}.`
  }

  return 'No active commands.'
}

export function formatOverallProgressLabel(overall: OverallProgress | null | undefined): string {
  if (!overall) return 'No active progress'
  const label = String(overall.label || '').trim()
  if (!label) return `${formatUiPercentFromFraction(overall.pct)} complete`
  return `${formatUiPercentFromFraction(overall.pct)} · ${label}`
}

export function runtimePhaseTone(phase: LegacyBootPhase | string | null | undefined): LegacyBootPhase {
  const value = String(phase || 'idle').trim().toLowerCase()
  if (value === 'booting') return 'booting'
  if (value === 'ready') return 'ready'
  if (value === 'error') return 'error'
  return 'idle'
}

export function runtimePhaseLabel(phase: LegacyBootPhase | string | null | undefined): string {
  const tone = runtimePhaseTone(phase)
  if (tone === 'booting') return 'Booting'
  if (tone === 'ready') return 'Ready'
  if (tone === 'error') return 'Error'
  return 'Idle'
}

export function describeRuntimePhase(options: {
  phase: LegacyBootPhase | string | null | undefined
  stage?: string | null | undefined
  elapsedMs?: number | null | undefined
  error?: string | null | undefined
}): string {
  const tone = runtimePhaseTone(options.phase)
  if (tone === 'error') {
    return String(options.error || '').trim() || 'Unknown runtime error'
  }

  if (tone === 'booting') {
    const elapsed = Math.max(0, Math.floor(Number(options.elapsedMs || 0)))
    const stage = String(options.stage || '').trim() || 'Starting frontend runtime'
    return elapsed > 0 ? `${stage} (${elapsed}ms)` : stage
  }

  if (tone === 'ready') {
    const elapsed = Math.max(0, Math.floor(Number(options.elapsedMs || 0)))
    return elapsed > 0 ? `Frontend runtime ready (${elapsed}ms)` : 'Frontend runtime ready.'
  }

  return 'Runtime has not reported startup activity yet.'
}
