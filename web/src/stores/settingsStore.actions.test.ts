import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  invokeLegacyDataBrowseMarketDataDirRuntime,
  invokeLegacyDataOpenMarketDataDirRuntime,
  invokeLegacyDataOpenPolygonRuntime,
  invokeLegacyDataResetMarketDataDirRuntime,
  invokeLegacyDataSetMarketDataDirRuntime,
} from '../legacy/actions'
import { invokeCommand } from '../tauri/invoke'
import { useSettingsStore } from './settingsStore'
import { useWorkflowStore } from './workflowStore'

vi.mock('../legacy/actions', () => ({
  invokeLegacyDataBrowseMarketDataDirRuntime: vi.fn(async () => true),
  invokeLegacyDataSetMarketDataDirRuntime: vi.fn(async () => true),
  invokeLegacyDataResetMarketDataDirRuntime: vi.fn(async () => true),
  invokeLegacyDataOpenMarketDataDirRuntime: vi.fn(async () => true),
  invokeLegacyDataOpenPolygonRuntime: vi.fn(async () => true),
}))

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeLegacyDataBrowseMarketDataDirRuntimeMock = vi.mocked(invokeLegacyDataBrowseMarketDataDirRuntime)
const invokeLegacyDataSetMarketDataDirRuntimeMock = vi.mocked(invokeLegacyDataSetMarketDataDirRuntime)
const invokeLegacyDataResetMarketDataDirRuntimeMock = vi.mocked(invokeLegacyDataResetMarketDataDirRuntime)
const invokeLegacyDataOpenMarketDataDirRuntimeMock = vi.mocked(invokeLegacyDataOpenMarketDataDirRuntime)
const invokeLegacyDataOpenPolygonRuntimeMock = vi.mocked(invokeLegacyDataOpenPolygonRuntime)
const invokeCommandMock = vi.mocked(invokeCommand)

interface LegacyField {
  value?: string
  checked?: boolean
  type?: string
}

interface DocumentHarness {
  dataset: Record<string, string>
  fields: Record<string, LegacyField>
}

function installDocumentHarness(): DocumentHarness {
  const dataset: Record<string, string> = {}
  const fields: Record<string, LegacyField> = {
    marketDataDir: { value: '' },
    marketDataMoveExisting: { type: 'checkbox', checked: true },
  }
  vi.stubGlobal('document', {
    documentElement: { dataset },
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { dataset, fields }
}

describe('settings store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    invokeLegacyDataBrowseMarketDataDirRuntimeMock.mockClear()
    invokeLegacyDataBrowseMarketDataDirRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataSetMarketDataDirRuntimeMock.mockClear()
    invokeLegacyDataSetMarketDataDirRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataResetMarketDataDirRuntimeMock.mockClear()
    invokeLegacyDataResetMarketDataDirRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataOpenMarketDataDirRuntimeMock.mockClear()
    invokeLegacyDataOpenMarketDataDirRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataOpenPolygonRuntimeMock.mockClear()
    invokeLegacyDataOpenPolygonRuntimeMock.mockResolvedValue(true)
    invokeCommandMock.mockClear()
    invokeCommandMock.mockResolvedValue(undefined)
    harness = installDocumentHarness()
  })

  it('applies settings field updates without legacy settings action dispatch', () => {
    const store = useSettingsStore()
    store.setInputField('settingsMarketDataDir', ' C:/cache ')
    store.setSelectField('settingsThemeSelect', 'dark')
    store.setCheckboxField('settingsWorkflowAdvanced', true)
    store.setCheckboxField('settingsControlsCollapsed', true)
    store.setCheckboxField('settingsMoveExisting', false)

    expect(store.marketDataDir).toBe('C:/cache')
    expect(harness.fields.marketDataDir?.value).toBe('C:/cache')
    expect(store.theme).toBe('dark')
    expect(harness.dataset.themePref).toBe('dark')
    expect(store.workflowAdvanced).toBe(true)
    expect(harness.dataset.workflowAdvanced).toBe('1')
    expect(store.controlsCollapsed).toBe(true)
    expect(harness.dataset.controlsCollapsed).toBe('1')
    expect(store.moveExisting).toBe(false)
    expect(harness.fields.marketDataMoveExisting?.checked).toBe(false)
    expect(invokeCommandMock).toHaveBeenCalled()
  })

  it('dispatches market-data directory actions', () => {
    const store = useSettingsStore()
    store.browseMarketDataDir()
    store.setMarketDataDir(' C:/md ', true)
    store.resetMarketDataDir()
    store.openMarketDataDir()

    expect(invokeLegacyDataBrowseMarketDataDirRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataSetMarketDataDirRuntimeMock).toHaveBeenCalledWith({
      dir: 'C:/md',
      moveExisting: true,
    })
    expect(invokeLegacyDataResetMarketDataDirRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataOpenMarketDataDirRuntimeMock).toHaveBeenCalled()
  })

  it('dispatches utility and navigation actions', () => {
    const store = useSettingsStore()
    const workflowStore = useWorkflowStore()

    store.openWorkflowData()
    store.openRunsDir()
    store.openReportsDir()
    store.openPolygonModal()
    expect(workflowStore.workspace).toBe('workflow')
    expect(workflowStore.workflow).toBe('data')

    store.gotoLibrary()
    expect(workflowStore.workspace).toBe('library')
    expect(workflowStore.workflow).toBe('history')

    store.gotoAggregate()
    expect(workflowStore.workspace).toBe('aggregate')
    expect(workflowStore.workflow).toBe('settings')

    expect(invokeLegacyDataOpenPolygonRuntimeMock).toHaveBeenCalled()
    expect(invokeCommandMock).toHaveBeenCalledWith('open_runs_dir')
    expect(invokeCommandMock).toHaveBeenCalledWith('open_reports_dir')
  })
})
