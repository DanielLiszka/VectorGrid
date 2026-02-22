import { invoke as tauriApiInvoke } from '@tauri-apps/api/core'
import { isTauriCommand, type TauriCommand } from './commands'
import { TauriRuntimeError, normalizeTauriRuntimeError } from './runtimeError'

export type InvokeArgs = Record<string, unknown> | undefined

type GlobalInvokeFn = <T>(command: string, args?: InvokeArgs) => Promise<T>

function globalInvoke(): GlobalInvokeFn | null {
  const tauri = window.__TAURI__
  if (!tauri) return null
  if (tauri.core && typeof tauri.core.invoke === 'function') {
    return tauri.core.invoke as GlobalInvokeFn
  }
  if (typeof tauri.invoke === 'function') {
    return tauri.invoke as GlobalInvokeFn
  }
  return null
}

export function tauriRuntimeAvailable(): boolean {
  return globalInvoke() !== null
}

export async function invokeCommand<T>(command: TauriCommand, args?: InvokeArgs): Promise<T> {
  try {
    const invoke = globalInvoke()
    if (invoke) {
      return invoke<T>(command, args)
    }
    return tauriApiInvoke<T>(command, args)
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
