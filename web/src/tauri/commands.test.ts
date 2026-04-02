import { describe, expect, it } from 'vitest'
import { TAURI_COMMANDS, isTauriCommand } from './commands'

describe('tauri command registry', () => {
  it('contains the full handler set and no duplicates', () => {
    expect(TAURI_COMMANDS.length).toBe(78)
    expect(new Set(TAURI_COMMANDS).size).toBe(78)
  })

  it('validates known and unknown command names', () => {
    expect(isTauriCommand('run_double_ma_optimization')).toBe(true)
    expect(isTauriCommand('not_real_command')).toBe(false)
  })
})
