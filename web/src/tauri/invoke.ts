import { invoke as tauriApiInvoke, isTauri } from '@tauri-apps/api/core'
import { isTauriCommand, type TauriCommand } from './commands'
import { TauriRuntimeError, normalizeTauriRuntimeError } from './runtimeError'

export type InvokeArgs = Record<string, unknown> | undefined

export function tauriRuntimeAvailable(): boolean {
  try {
    return isTauri()
  } catch {
    return false
  }
}

export async function invokeCommand<T>(command: TauriCommand, args?: InvokeArgs): Promise<T> {
  try {
    return await tauriApiInvoke<T>(command, args)
  } catch (cause) {
    throw normalizeTauriRuntimeError({
      code: 'TAURI_INVOKE_FAILED',
      operation: `invoke:${command}`,
      fallbackMessage: `Failed to invoke Tauri command "${command}".`,
      causeValue: cause,
    })
  }
}

export async function invokeByName<T>(command: string, args?: InvokeArgs): Promise<T> {
  if (!isTauriCommand(command)) {
    throw new TauriRuntimeError({
      code: 'TAURI_COMMAND_UNKNOWN',
      operation: `invoke:${command}`,
      message: `Unknown Tauri command: ${command}`,
    })
  }
  return invokeCommand<T>(command, args)
}
