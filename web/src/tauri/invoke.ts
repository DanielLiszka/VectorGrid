import { invoke as tauriApiInvoke, isTauri } from '@tauri-apps/api/core'
import { isTauriCommand, type TauriCommand } from './commands'
import { TauriRuntimeError, normalizeTauriRuntimeError } from './runtimeError'

export type InvokeArgs = Record<string, unknown> | undefined

export interface InvokeLifecycleHooks {
  beforeInvoke?: (command: TauriCommand, args?: InvokeArgs) => unknown
  afterSuccess?: (
    token: unknown,
    command: TauriCommand,
    args: InvokeArgs,
    result: unknown,
  ) => void
  afterError?: (
    token: unknown,
    command: TauriCommand,
    args: InvokeArgs,
    error: unknown,
  ) => void
}

let invokeLifecycleHooks: InvokeLifecycleHooks | null = null

export function tauriRuntimeAvailable(): boolean {
  try {
    return isTauri()
  } catch {
    return false
  }
}

export function setInvokeLifecycleHooks(
  hooks: InvokeLifecycleHooks | null,
): void {
  invokeLifecycleHooks = hooks
}

export function resetInvokeLifecycleHooksForTests(): void {
  invokeLifecycleHooks = null
}

function safeBeforeInvoke(
  command: TauriCommand,
  args: InvokeArgs,
): unknown {
  try {
    return invokeLifecycleHooks?.beforeInvoke?.(command, args)
  } catch {
    return null
  }
}

function safeAfterSuccess(
  token: unknown,
  command: TauriCommand,
  args: InvokeArgs,
  result: unknown,
): void {
  try {
    invokeLifecycleHooks?.afterSuccess?.(token, command, args, result)
  } catch {
      }
}

function safeAfterError(
  token: unknown,
  command: TauriCommand,
  args: InvokeArgs,
  error: unknown,
): void {
  try {
    invokeLifecycleHooks?.afterError?.(token, command, args, error)
  } catch {
  }
}

function humanizeCommand(command: TauriCommand): string {
  return String(command || '')
    .split('_')
    .filter((part) => part.length > 0)
    .join(' ')
}

export async function invokeCommand<T>(command: TauriCommand, args?: InvokeArgs): Promise<T> {
  const token = safeBeforeInvoke(command, args)
  if (!tauriRuntimeAvailable()) {
    const error = new TauriRuntimeError({
      code: 'TAURI_INVOKE_FAILED',
      operation: `invoke:${command}`,
      message: `Desktop runtime unavailable for ${humanizeCommand(command)}. Open this action in the Tauri desktop app.`,
    })
    safeAfterError(token, command, args, error)
    throw error
  }
  try {
    const result = await tauriApiInvoke<T>(command, args)
    safeAfterSuccess(token, command, args, result)
    return result
  } catch (cause) {
    const error = normalizeTauriRuntimeError({
      code: 'TAURI_INVOKE_FAILED',
      operation: `invoke:${command}`,
      fallbackMessage: `Failed to invoke Tauri command "${command}".`,
      causeValue: cause,
    })
    safeAfterError(token, command, args, error)
    throw error
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
