import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import { expect } from 'vitest'

export interface MockIpcCall {
  command: string
  args: Record<string, unknown> | undefined
}

export type MockIpcResolver =
  | unknown
  | Error
  | ((args: Record<string, unknown> | undefined, call: MockIpcCall) => unknown | Promise<unknown>)

export interface MockIpcHarness {
  calls: MockIpcCall[]
  clear(): void
  expectCalled(command: string, partialArgs?: Record<string, unknown>): void
  expectCallCount(command: string, count: number): void
}

export interface InstallMockIpcOptions {
  allowUnknown?: boolean
  shouldMockEvents?: boolean
}

function toError(value: unknown): Error {
  if (value instanceof Error) return value
  return new Error(String(value))
}

function matchesPartialArgs(
  actual: Record<string, unknown> | undefined,
  partialArgs?: Record<string, unknown>,
): boolean {
  if (!partialArgs) return true
  if (!actual) return false
  return Object.entries(partialArgs).every(([key, value]) => {
    return JSON.stringify(actual[key]) === JSON.stringify(value)
  })
}

export function resetTauriApiMocks(): void {
  clearMocks()
  delete (globalThis as { isTauri?: boolean }).isTauri
  if (typeof window === 'undefined') return
  delete window.__TAURI__
  window.__VTA_INVOKE_PATCHED__ = false
  window.__VTA_INVOKE_PATCH_BLOCKED__ = false
  window.__VTA_EVENT_UNSUBSCRIBERS__ = []
}

export function installLegacyTauriWindowBridge(): void {
  ;(globalThis as { isTauri?: boolean }).isTauri = true
  window.__TAURI__ = {
    core: {
      invoke: (command: string, args?: Record<string, unknown>) => invoke(command, args),
    },
    event: {
      listen: (event, handler) => listen(event, handler),
    },
  }
  window.__VTA_INVOKE_PATCHED__ = false
  window.__VTA_INVOKE_PATCH_BLOCKED__ = false
  window.__VTA_EVENT_UNSUBSCRIBERS__ = []
}

export function installMockIpc(
  resolvers: Record<string, MockIpcResolver>,
  options?: InstallMockIpcOptions,
): MockIpcHarness {
  const calls: MockIpcCall[] = []
  ;(globalThis as { isTauri?: boolean }).isTauri = true
  mockIPC(
    async (command, args) => {
      const call: MockIpcCall = {
        command,
        args: args as Record<string, unknown> | undefined,
      }
      calls.push(call)
      if (!(command in resolvers)) {
        if (options?.allowUnknown) return undefined
        throw new Error(`Unexpected mocked Tauri command: ${command}`)
      }
      const resolver = resolvers[command]
      if (resolver instanceof Error) throw resolver
      if (typeof resolver === 'function') {
        return await resolver(call.args, call)
      }
      return resolver
    },
    { shouldMockEvents: options?.shouldMockEvents ?? false },
  )

  return {
    calls,
    clear() {
      calls.splice(0)
    },
    expectCalled(command, partialArgs) {
      const match = calls.find((call) => call.command === command && matchesPartialArgs(call.args, partialArgs))
      expect(match).toBeTruthy()
    },
    expectCallCount(command, count) {
      const hits = calls.filter((call) => call.command === command)
      expect(hits.length).toBe(count)
    },
  }
}

export function mockCommandSuccess(command: string, value: unknown): MockIpcHarness {
  return installMockIpc({ [command]: value })
}

export function mockCommandFailure(command: string, error: string | Error): MockIpcHarness {
  return installMockIpc({ [command]: toError(error) })
}
