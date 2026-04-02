import { invokeCommand } from '../tauri/invoke'
import {
  markTasksCancellingForCommand,
  restoreTasksRunning,
} from '../stores/taskStore'
import type { TauriCommand } from '../tauri/commands'

export interface CancelCurrentRunResult {
  requested: boolean
  usedLegacyFallback: boolean
  backgroundTaskRequested: boolean
}

async function requestCancellationByCommand(
  command: TauriCommand,
): Promise<boolean> {
  const touchedTaskIds = markTasksCancellingForCommand(command)
  try {
    await invokeCommand(command)
    return true
  } catch {
    restoreTasksRunning(touchedTaskIds)
    return false
  }
}

async function requestForegroundRunCancellation(): Promise<{
  requested: boolean
  usedLegacyFallback: boolean
}> {
  const touchedTaskIds = markTasksCancellingForCommand('cancel_current_run')
  try {
    await invokeCommand('cancel_current_run')
    return {
      requested: true,
      usedLegacyFallback: false,
    }
  } catch {
    try {
      await invokeCommand('cancel_double_ma_optimization')
      return {
        requested: true,
        usedLegacyFallback: true,
      }
    } catch {
      restoreTasksRunning(touchedTaskIds)
      return {
        requested: false,
        usedLegacyFallback: false,
      }
    }
  }
}

export async function requestCancellationForMappedCommand(
  command: TauriCommand,
): Promise<void> {
  if (command === 'cancel_current_run') {
    await requestCancelCurrentRun()
    return
  }
  if (command === 'cancel_background_task') {
    const requested = await requestCancellationByCommand(command)
    if (!requested) {
      throw new Error('Cancellation command is unavailable.')
    }
    return
  }
  throw new Error(`Unsupported cancellation command: ${command}`)
}

export async function requestCancelCurrentRun(): Promise<CancelCurrentRunResult> {
  const backgroundTaskPromise = requestCancellationByCommand('cancel_background_task')
  const foreground = await requestForegroundRunCancellation()
  const backgroundTaskRequested = await backgroundTaskPromise

  const requested = foreground.requested || backgroundTaskRequested
  const usedLegacyFallback = foreground.usedLegacyFallback

  if (!requested) {
    throw new Error('Cancellation command is unavailable.')
  }

  return {
    requested,
    usedLegacyFallback,
    backgroundTaskRequested,
  }
}
