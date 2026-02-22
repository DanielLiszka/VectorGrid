import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { TAURI_COMMANDS } from './commands'
import { invokeByName, invokeCommand } from './invoke'

interface InvokeCall {
  command: string
  args: Record<string, unknown> | undefined
}

describe('tauri invoke contract smoke', () => {
  const calls: InvokeCall[] = []

  beforeEach(() => {
    calls.splice(0)
    ;(globalThis as Record<string, unknown>).window = {
      __TAURI__: {
        core: {
          invoke: vi.fn(async (command: string, args?: Record<string, unknown>) => {
            calls.push({ command, args })
            return { ok: true, command }
          }),
        },
      },
    }
  })

  afterEach(() => {
    delete (globalThis as Record<string, unknown>).window
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
})
