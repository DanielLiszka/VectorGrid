import { describe, expect, it } from 'vitest'
import { TAURI_COMMANDS, isTauriCommand } from './commands'

describe('tauri command registry', () => {
  it('contains a non-empty unique handler set', () => {
    expect(TAURI_COMMANDS.length).toBeGreaterThan(80)
    expect(new Set(TAURI_COMMANDS).size).toBe(TAURI_COMMANDS.length)
  })

  it('validates known and unknown command names', () => {
    expect(isTauriCommand('run_double_ma_optimization')).toBe(true)
    expect(isTauriCommand('list_artifact_library')).toBe(true)
    expect(isTauriCommand('load_run_artifact_hydration')).toBe(true)
    expect(isTauriCommand('load_run_artifact_detail')).toBe(true)
    expect(isTauriCommand('load_meta_artifact_replay')).toBe(true)
    expect(isTauriCommand('load_stage5_artifact_replay')).toBe(true)
    expect(isTauriCommand('load_ml_policy_eval_artifact_replay')).toBe(true)
    expect(isTauriCommand('save_report_xlsx')).toBe(true)
    expect(isTauriCommand('save_report_xlsx_from_artifact')).toBe(true)
    expect(isTauriCommand('open_reports_dir')).toBe(true)
    expect(isTauriCommand('open_path')).toBe(true)
    expect(isTauriCommand('reveal_path')).toBe(true)
    expect(isTauriCommand('not_real_command')).toBe(false)
  })
})
