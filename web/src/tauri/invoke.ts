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
    // keep invoke lifecycle instrumentation best-effort
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
    // keep invoke lifecycle instrumentation best-effort
  }
}

export async function invokeCommand<T>(command: TauriCommand, args?: InvokeArgs): Promise<T> {
  const token = safeBeforeInvoke(command, args)
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
