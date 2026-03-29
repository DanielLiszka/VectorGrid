import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { TAURI_COMMANDS } from './commands'
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
})
