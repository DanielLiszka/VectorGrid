import { describe, expect, it, vi } from 'vitest'
import {
  SETTINGS_SHORTCUT_SUMMARY_ROWS,
  createSharedShellCommands,
  dispatchSharedShellCommandShortcut,
  type ShellCommandRuntimeContext,
} from './shellCommandRegistry'

function runtimeContext(overrides: Partial<ShellCommandRuntimeContext> = {}): ShellCommandRuntimeContext {
  return {
    canRun: true,
    canValidate: true,
    canBrowseCsv: true,
    canPolygonOpen: true,
    canExportCsv: true,
    canExportJson: true,
    canOpenLatestReport: true,
    canOpenLatestMetaReport: true,
    browseCsv: vi.fn(),
    openPolygon: vi.fn(),
    toggleControls: vi.fn(),
    runOptimization: vi.fn(),
    validateOptimization: vi.fn(),
    exportTopCsv: vi.fn(),
    exportTopJson: vi.fn(),
    openLatestReport: vi.fn(),
    openLatestMetaReport: vi.fn(),
    ...overrides,
  }
}

function keyboardEvent(init: Partial<KeyboardEvent> = {}): KeyboardEvent {
  return {
    key: '',
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    preventDefault: vi.fn(),
    ...init,
  } as unknown as KeyboardEvent
}

describe('shellCommandRegistry', () => {
  it('exposes a stable shared command inventory with unique ids', () => {
    const commands = createSharedShellCommands(runtimeContext())
    const ids = commands.map((command) => command.id)

    expect(ids).toEqual([
      'run',
      'validate',
      'browse_csv',
      'open_polygon',
      'export_csv',
      'export_json',
      'open_latest_report',
      'open_latest_meta_report',
      'toggle_controls',
    ])
    expect(new Set(ids).size).toBe(ids.length)
    expect(commands.every((command) => command.name && command.desc && command.group)).toBe(true)
  })

  it('routes matching shortcuts through runtime actions only when enabled', () => {
    const browseCsv = vi.fn()
    const event = keyboardEvent({ key: 'o', ctrlKey: true })
    const handled = dispatchSharedShellCommandShortcut(
      event,
      runtimeContext({
        canBrowseCsv: false,
        browseCsv,
      }),
    )

    expect(handled).toBe(true)
    expect(event.preventDefault).toHaveBeenCalledTimes(1)
    expect(browseCsv).not.toHaveBeenCalled()
  })

  it('keeps settings shortcut summaries derived from shared registry labels', () => {
    expect(SETTINGS_SHORTCUT_SUMMARY_ROWS).toEqual([
      { keysLabel: 'Ctrl+K', label: 'Command palette' },
      { keysLabel: 'Ctrl+Enter', label: 'Run optimization' },
      { keysLabel: 'Ctrl+Shift+V', label: 'Run validation' },
      { keysLabel: 'Ctrl+Shift+C', label: 'Export Top CSV' },
      { keysLabel: 'Ctrl+O', label: 'Browse CSV' },
    ])
  })
})

