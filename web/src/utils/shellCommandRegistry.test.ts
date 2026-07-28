import { describe, expect, it, vi } from 'vitest'
import {
  SETTINGS_SHORTCUT_SUMMARY_ROWS,
  createSharedShellCommandEnabledStates,
  createSharedShellCommands,
  dispatchSharedShellCommandShortcut,
  type ShellCommandRuntimeContext,
} from './shellCommandRegistry'

function runtimeContext(overrides: Partial<ShellCommandRuntimeContext> = {}): ShellCommandRuntimeContext {
  return {
    canRun: true,
    canCancel: false,
    canValidate: true,
    canBrowseCsv: true,
    canPolygonOpen: true,
    canExportCsv: true,
    canExportJson: true,
    canOpenLatestReport: true,
    canOpenLatestMetaReport: true,
    controlsPanelVisible: true,
    browseCsv: vi.fn(),
    openPolygon: vi.fn(),
    toggleControls: vi.fn(),
    runOptimization: vi.fn(),
    cancelOptimization: vi.fn(),
    validateOptimization: vi.fn(),
    exportTopCsv: vi.fn(),
    exportTopJson: vi.fn(),
    openLatestReport: vi.fn(),
    openLatestMetaReport: vi.fn(),
    openTaskCenter: vi.fn(),
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
      'cancel_run',
      'validate',
      'browse_csv',
      'open_polygon',
      'export_csv',
      'export_json',
      'open_latest_report',
      'open_latest_meta_report',
      'open_task_center',
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

  it('replaces disabled command descriptions with the centralized reason', () => {
    const commands = createSharedShellCommands(runtimeContext({
      canValidate: false,
      disabledReasons: {
        validate: 'Load the matching dataset before validation.',
      },
    }))

    expect(commands.find((command) => command.id === 'validate')).toMatchObject({
      enabled: false,
      desc: 'Load the matching dataset before validation.',
    })
  })

  it('exposes enabled states for native menu synchronization', () => {
    const states = createSharedShellCommandEnabledStates(runtimeContext({
      canRun: false,
      canCancel: true,
      canExportJson: false,
      controlsPanelVisible: false,
    }))

    expect(states).toContainEqual({ commandId: 'run', enabled: false })
    expect(states).toContainEqual({ commandId: 'cancel_run', enabled: true })
    expect(states).toContainEqual({ commandId: 'export_json', enabled: false })
    expect(states).toContainEqual({ commandId: 'open_task_center', enabled: true })
    expect(states).toContainEqual({ commandId: 'toggle_controls', enabled: true, checked: false })
    expect(states.map((state) => state.commandId)).toEqual(
      createSharedShellCommands(runtimeContext()).map((command) => command.id),
    )
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
