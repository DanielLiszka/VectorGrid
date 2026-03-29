import type { TauriCommand } from '../tauri/commands'
import type { TrackedTask } from '../stores/taskStore'
import type { DesktopNotificationMode } from '../stores/settingsStore'
import { formatCommandLabel } from '../stores/taskStore'

export interface DesktopNotificationIntent {
  title: string
  body: string
}

export interface DesktopNotificationPolicyContext {
  documentVisibility?: string | null | undefined
}

const IMPORTANT_SUCCESS_COMMANDS = new Set<TauriCommand>([
  'run_double_ma_optimization',
  'run_composite_cpu_optimization',
  'run_composite_mo_optimization',
  'run_double_ma_validation',
  'run_composite_validation',
  'run_meta_selection',
  'run_meta_stage4_experimental',
  'run_meta_stage5_ml_confidence',
  'export_ml_signal_dataset',
  'generate_ml_signal_score_cache',
  'run_ml_signal_validation',
  'run_ml_signal_policy_sweep',
  'train_ml_confidence_model',
  'apply_ml_confidence_model',
  'evaluate_ml_confidence_policy',
  'build_report_xlsx',
  'build_validation_shortlist_csv',
])

function classifyTaskTitle(task: TrackedTask): string {
  const label = formatCommandLabel(task.command)
  if (task.status === 'success') return `${label} complete`
  if (task.status === 'failed') return `${label} failed`
  if (task.status === 'cancelled') return `${label} cancelled`
  return label
}

function classifyTaskBody(task: TrackedTask): string {
  const detail = String(task.message || task.error || '').trim()
  if (detail) return detail
  if (task.status === 'success') return `${formatCommandLabel(task.command)} finished successfully.`
  if (task.status === 'failed') return 'Review the task center for the failure detail.'
  if (task.status === 'cancelled') return 'The task ended before completion.'
  return ''
}

export function desktopNotificationIntentForTask(
  task: TrackedTask | null | undefined,
  mode: DesktopNotificationMode,
  context: DesktopNotificationPolicyContext = {},
): DesktopNotificationIntent | null {
  if (!task || mode === 'off') return null
  const visibility = String(context.documentVisibility || '').trim().toLowerCase()
  if (visibility === 'visible') return null
  const command = task.command as TauriCommand
  const terminal = task.status === 'success' || task.status === 'failed' || task.status === 'cancelled'
  if (!terminal) return null

  if (mode === 'failures' && task.status === 'success') return null
  if (task.status === 'success' && !IMPORTANT_SUCCESS_COMMANDS.has(command)) return null

  return {
    title: classifyTaskTitle(task),
    body: classifyTaskBody(task),
  }
}
