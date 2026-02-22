import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { invokeCommand } from '../tauri/invoke'
import { useSettingsStore } from '../stores/settingsStore'
import { applySettingsBootstrapFromStorage, readSettingsBootstrapState } from './settingsBootstrap'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

type StorageMap = Record<string, string>

function installStorage(values: StorageMap): void {
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => (Object.prototype.hasOwnProperty.call(values, key) ? values[key] : null),
  })
}

function installDocumentDataset(): Record<string, string> {
  const dataset: Record<string, string> = {}
  vi.stubGlobal('document', { documentElement: { dataset } })
  return dataset
}

describe('settings bootstrap service', () => {
  let dataset: Record<string, string>

  beforeEach(() => {
    setActivePinia(createPinia())
    invokeCommandMock.mockClear()
    invokeCommandMock.mockResolvedValue(undefined)
    installStorage({})
    dataset = installDocumentDataset()
  })

  it('reads and normalizes persisted settings bootstrap keys', () => {
    installStorage({
      vta_theme_pref_v1: 'Dark',
      vta_window_effect_v1: 'MICA',
      vta_workflow_advanced_v1: '1',
      vta_controls_collapsed_v1: 'false',
    })

    expect(readSettingsBootstrapState()).toEqual({
      theme: 'dark',
      effect: 'mica',
      workflowAdvanced: true,
      controlsCollapsed: false,
    })
  })

  it('ignores invalid persisted values', () => {
    installStorage({
      vta_theme_pref_v1: 'midnight',
      vta_window_effect_v1: 'blur',
      vta_workflow_advanced_v1: 'maybe',
      vta_controls_collapsed_v1: 'sometimes',
    })

    expect(readSettingsBootstrapState()).toEqual({
      theme: '',
      effect: '',
      workflowAdvanced: null,
      controlsCollapsed: null,
    })
  })

  it('applies persisted values through typed settings actions', () => {
    installStorage({
      vta_theme_pref_v1: 'light',
      vta_window_effect_v1: 'acrylic',
      vta_workflow_advanced_v1: 'true',
      vta_controls_collapsed_v1: '1',
    })

    applySettingsBootstrapFromStorage()

    const store = useSettingsStore()
    expect(store.theme).toBe('light')
    expect(store.effect).toBe('acrylic')
    expect(store.workflowAdvanced).toBe(true)
    expect(store.controlsCollapsed).toBe(true)
    expect(dataset.themePref).toBe('light')
    expect(dataset.windowEffect).toBe('acrylic')
    expect(dataset.workflowAdvanced).toBe('1')
    expect(dataset.controlsCollapsed).toBe('1')
    expect(invokeCommandMock).toHaveBeenCalledWith(
      'set_window_effect',
      expect.objectContaining({ effect: 'acrylic' }),
    )
  })

  it('does not dispatch when persisted state is empty', () => {
    applySettingsBootstrapFromStorage()
    expect(invokeCommandMock).not.toHaveBeenCalled()
  })
})
