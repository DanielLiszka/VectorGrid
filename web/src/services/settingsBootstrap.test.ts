import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useSettingsStore } from '../stores/settingsStore'
import { applySettingsBootstrapFromStorage, readSettingsBootstrapState } from './settingsBootstrap'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeCommandMock = vi.mocked(invokeCommand)
let pinia: ReturnType<typeof createPinia>

type StorageMap = Record<string, string>

function installStorage(values: StorageMap): {
  setItemMock: ReturnType<typeof vi.fn>
  removeItemMock: ReturnType<typeof vi.fn>
} {
  const setItemMock = vi.fn((key: string, value: string) => {
    values[key] = value
  })
  const removeItemMock = vi.fn((key: string) => {
    delete values[key]
  })
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => (Object.prototype.hasOwnProperty.call(values, key) ? values[key] : null),
    setItem: setItemMock,
    removeItem: removeItemMock,
  })
  return { setItemMock, removeItemMock }
}

function installDocumentDataset(): Record<string, string> {
  const dataset: Record<string, string> = {}
  vi.stubGlobal('document', { documentElement: { dataset } })
  return dataset
}

function installNavigator(userAgent: string): void {
  vi.stubGlobal('navigator', { userAgent })
}

describe('settings bootstrap service', () => {
  let dataset: Record<string, string>

  beforeEach(() => {
    pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    invokeCommandMock.mockClear()
    invokeCommandMock.mockResolvedValue(undefined)
    installStorage({})
    dataset = installDocumentDataset()
    installNavigator('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')
  })

  it('reads and normalizes persisted settings bootstrap keys', () => {
    installStorage({
      vta_theme_pref_v1: 'Dark',
      vta_window_effect_v1: 'blur',
      vta_help_mode_v1: 'concise',
      vta_density_mode_v1: 'dense',
      vta_ui_mode_v1: 'guided',
      vta_guided_step_v1: 'validate',
      vta_workflow_advanced_v1: '1',
      vta_controls_collapsed_v1: 'false',
    })

    expect(readSettingsBootstrapState()).toEqual({
      theme: 'dark',
      effect: 'blur',
      helpMode: 'concise',
      densityMode: 'dense',
      uiMode: 'guided',
      guidedStep: 'validate',
      desktopNotificationMode: '',
      workflowAdvanced: true,
      controlsCollapsed: false,
    })
  })

  it('ignores invalid or unsupported persisted values on the current platform', () => {
    installStorage({
      vta_theme_pref_v1: 'midnight',
      vta_window_effect_v1: 'vibrancy',
      vta_help_mode_v1: 'verbose',
      vta_density_mode_v1: 'spacious',
      vta_ui_mode_v1: 'pro',
      vta_guided_step_v1: 'unknown',
      vta_workflow_advanced_v1: 'maybe',
      vta_controls_collapsed_v1: 'sometimes',
    })

    expect(readSettingsBootstrapState()).toEqual({
      theme: '',
      effect: '',
      helpMode: '',
      densityMode: '',
      uiMode: '',
      guidedStep: '',
      desktopNotificationMode: '',
      workflowAdvanced: null,
      controlsCollapsed: null,
    })
  })

  it('accepts macOS vibrancy when the current platform supports it', () => {
    installNavigator('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5)')
    installStorage({
      vta_window_effect_v1: 'vibrancy',
    })

    expect(readSettingsBootstrapState().effect).toBe('vibrancy')
  })

  it('applies persisted values through typed settings actions', () => {
    const values = {
      vta_theme_pref_v1: 'light',
      vta_window_effect_v1: 'acrylic',
      vta_help_mode_v1: 'off',
      vta_density_mode_v1: 'comfortable',
      vta_ui_mode_v1: 'guided',
      vta_guided_step_v1: 'trade',
      vta_desktop_notification_mode_v1: 'failures',
      vta_workflow_advanced_v1: 'true',
      vta_controls_collapsed_v1: '1',
    }
    const { setItemMock, removeItemMock } = installStorage(values)

    applySettingsBootstrapFromStorage()

    const store = useSettingsStore(pinia)
    expect(store.theme).toBe('light')
    expect(store.effect).toBe('acrylic')
    expect(store.helpMode).toBe('off')
    expect(store.densityMode).toBe('comfortable')
    expect(store.uiMode).toBe('guided')
    expect(store.guidedStep).toBe('trade')
    expect(store.desktopNotificationMode).toBe('failures')
    expect(store.workflowAdvanced).toBe(true)
    expect(store.controlsCollapsed).toBe(true)
    expect(dataset.themePref).toBe('light')
    expect(dataset.windowEffect).toBe('acrylic')
    expect(dataset.helpMode).toBe('off')
    expect(dataset.density).toBe('comfortable')
    expect(dataset.uiMode).toBe('guided')
    expect(dataset.guidedStep).toBe('trade')
    expect(dataset.workflowAdvanced).toBe('1')
    expect(dataset.controlsCollapsed).toBe('1')
    expect(invokeCommandMock).toHaveBeenCalledWith(
      'set_window_effect',
      expect.objectContaining({ effect: 'acrylic' }),
    )
    expect(setItemMock).not.toHaveBeenCalled()
    expect(removeItemMock).not.toHaveBeenCalled()
  })

  it('hydrates through the bootstrap path without rewriting unchanged persisted storage', () => {
    const values = {
      vta_theme_pref_v1: 'dark',
      vta_window_effect_v1: 'mica',
      vta_help_mode_v1: 'concise',
      vta_density_mode_v1: 'dense',
      vta_ui_mode_v1: 'guided',
      vta_guided_step_v1: 'optimize',
      vta_desktop_notification_mode_v1: 'important',
      vta_workflow_advanced_v1: '1',
      vta_controls_collapsed_v1: '1',
    }
    const { setItemMock, removeItemMock } = installStorage(values)

    applySettingsBootstrapFromStorage()

    const store = useSettingsStore(pinia)
    expect(store.theme).toBe('dark')
    expect(store.effect).toBe('mica')
    expect(store.helpMode).toBe('concise')
    expect(store.densityMode).toBe('dense')
    expect(store.uiMode).toBe('guided')
    expect(store.guidedStep).toBe('optimize')
    expect(store.desktopNotificationMode).toBe('important')
    expect(store.workflowAdvanced).toBe(true)
    expect(store.controlsCollapsed).toBe(true)
    expect(dataset.themePref).toBe('dark')
    expect(dataset.windowEffect).toBe('mica')
    expect(dataset.helpMode).toBe('concise')
    expect(dataset.density).toBe('dense')
    expect(dataset.uiMode).toBe('guided')
    expect(dataset.guidedStep).toBe('optimize')
    expect(dataset.workflowAdvanced).toBe('1')
    expect(dataset.controlsCollapsed).toBe('1')
    expect(invokeCommandMock).toHaveBeenCalledWith(
      'set_window_effect',
      expect.objectContaining({ effect: 'mica' }),
    )
    expect(setItemMock).not.toHaveBeenCalled()
    expect(removeItemMock).not.toHaveBeenCalled()
  })

  it('preserves an explicit Advanced Mode preference across bootstrap', () => {
    const values = {
      vta_ui_mode_v1: 'advanced',
    }
    const { setItemMock, removeItemMock } = installStorage(values)

    applySettingsBootstrapFromStorage()

    const store = useSettingsStore(pinia)
    expect(store.uiMode).toBe('advanced')
    expect(dataset.uiMode).toBe('advanced')
    expect(setItemMock).not.toHaveBeenCalled()
    expect(removeItemMock).not.toHaveBeenCalled()
  })

  it('does not dispatch when persisted state is empty', () => {
    applySettingsBootstrapFromStorage()
    expect(useSettingsStore(pinia).uiMode).toBe('guided')
    expect(invokeCommandMock).not.toHaveBeenCalled()
  })

  it('treats storage access failures as an empty bootstrap state', () => {
    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('storage unavailable')
      },
      setItem: () => undefined,
      removeItem: () => undefined,
    })

    expect(readSettingsBootstrapState()).toEqual({
      theme: '',
      effect: '',
      helpMode: '',
      densityMode: '',
      uiMode: '',
      guidedStep: '',
      desktopNotificationMode: '',
      workflowAdvanced: null,
      controlsCollapsed: null,
    })

    applySettingsBootstrapFromStorage()
    expect(invokeCommandMock).not.toHaveBeenCalled()
  })
})
