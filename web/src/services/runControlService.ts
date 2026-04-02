import { invokeCommand } from '../tauri/invoke'

export interface CancelCurrentRunResult {
  requested: boolean
  usedLegacyFallback: boolean
  backgroundTaskRequested: boolean
}

export async function requestCancelCurrentRun(): Promise<CancelCurrentRunResult> {
  let requested = false
  let usedLegacyFallback = false
  const backgroundTaskPromise = invokeCommand('cancel_background_task')
    .then(() => true)
    .catch(() => false)

  try {
    await invokeCommand('cancel_current_run')
    requested = true
  } catch {
    try {
      await invokeCommand('cancel_double_ma_optimization')
      requested = true
      usedLegacyFallback = true
    } catch {
      // keep probing background cancellation path below
    }
  }

  const backgroundTaskRequested = await backgroundTaskPromise
  if (backgroundTaskRequested) {
    requested = true
  }

  if (!requested) {
    throw new Error('Cancellation command is unavailable.')
  }

  return {
    requested,
    usedLegacyFallback,
    backgroundTaskRequested,
  }
}
