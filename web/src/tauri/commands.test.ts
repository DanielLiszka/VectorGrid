import { describe, expect, it } from 'vitest'
import { TAURI_COMMANDS, isTauriCommand } from './commands'

describe('tauri command registry', () => {
  it('contains the full handler set and no duplicates', () => {
    expect(TAURI_COMMANDS.length).toBe(85)
    expect(new Set(TAURI_COMMANDS).size).toBe(85)
  })

  it('validates known and unknown command names', () => {
    expect(isTauriCommand('run_double_ma_optimization')).toBe(true)
    expect(isTauriCommand('list_artifact_library')).toBe(true)
    expect(isTauriCommand('load_run_artifact_hydration')).toBe(true)
    expect(isTauriCommand('load_run_artifact_detail')).toBe(true)
    expect(isTauriCommand('load_meta_artifact_replay')).toBe(true)
    expect(isTauriCommand('load_stage5_artifact_replay')).toBe(true)
    expect(isTauriCommand('load_ml_policy_eval_artifact_replay')).toBe(true)
    expect(isTauriCommand('save_report_xlsx_from_artifact')).toBe(true)
    expect(isTauriCommand('not_real_command')).toBe(false)
  })
})
