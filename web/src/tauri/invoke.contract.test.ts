import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { TAURI_COMMANDS } from './commands'
import { TauriRuntimeError } from './runtimeError'
import { invokeByName, invokeCommand, tauriRuntimeAvailable } from './invoke'

interface InvokeCall {
  command: string
  args: Record<string, unknown> | undefined
}

const calls: InvokeCall[] = []
const invokeMock = vi.fn(async (command: string, args?: Record<string, unknown>) => {
  calls.push({ command, args })
  return { ok: true, command }
})
const isTauriMock = vi.fn(() => true)

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (command: string, args?: Record<string, unknown>) => invokeMock(command, args),
  isTauri: () => isTauriMock(),
}))

describe('tauri invoke contract smoke', () => {
  beforeEach(() => {
    calls.splice(0)
    invokeMock.mockClear()
    isTauriMock.mockReset()
    isTauriMock.mockReturnValue(true)
  })

  afterEach(() => {
    calls.splice(0)
  })

  it('invokes every registered command with typed wrapper', async () => {
    for (const command of TAURI_COMMANDS) {
      const args = { smoke: true, nested: { command } }
      await invokeCommand(command, args)
    }

    expect(calls.length).toBe(TAURI_COMMANDS.length)
    for (const command of TAURI_COMMANDS) {
      const hit = calls.find((call) => call.command === command)
      expect(hit).toBeTruthy()
      expect(hit?.args).toMatchObject({ smoke: true, nested: { command } })
    }
  })

  it('resolves every registered command through invokeByName', async () => {
    for (const command of TAURI_COMMANDS) {
      await invokeByName(command, { smokeByName: command })
    }
    expect(calls.length).toBe(TAURI_COMMANDS.length)
  })

  it('detects runtime presence through the module API instead of globals', () => {
    expect(tauriRuntimeAvailable()).toBe(true)
    isTauriMock.mockReturnValue(false)
    expect(tauriRuntimeAvailable()).toBe(false)
  })

  it('returns a user-facing desktop-runtime message when commands run outside Tauri', async () => {
    isTauriMock.mockReturnValue(false)

    await expect(invokeCommand('polygon_get_cache_info')).rejects.toMatchObject({
      message: 'Desktop runtime unavailable for polygon get cache info. Open this action in the Tauri desktop app.',
      operation: 'invoke:polygon_get_cache_info',
    })
    expect(calls).toEqual([])
  })

  it('rejects unknown command names before they reach the raw invoke path', async () => {
    await expect(invokeByName('unknown_command')).rejects.toBeInstanceOf(TauriRuntimeError)
    expect(calls).toEqual([])
  })
})
