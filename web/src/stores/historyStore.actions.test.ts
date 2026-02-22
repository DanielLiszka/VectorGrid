import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  invokeLegacyHistoryApplySavedRunRuntime,
  invokeLegacyHistoryApplySessionRuntime,
  invokeLegacyHistoryClearSessionRuntime,
  invokeLegacyHistoryDeleteSavedRunRuntime,
  invokeLegacyHistoryExportPresetRuntime,
  invokeLegacyHistoryImportPresetRuntime,
  invokeLegacyHistoryLoadSavedRunRuntime,
  invokeLegacyHistoryOpenSavedRunReportRuntime,
  invokeLegacyHistoryRefreshSavedRunsRuntime,
  invokeLegacyHistoryRerunSavedRunRuntime,
  invokeLegacyHistoryRevealSavedRunRuntime,
  invokeLegacyHistorySaveSavedRunTagsRuntime,
} from '../legacy/actions'
import { invokeCommand } from '../tauri/invoke'
import { useHistoryStore } from './historyStore'

vi.mock('../legacy/actions', () => ({
  invokeLegacyHistoryApplySessionRuntime: vi.fn(async () => true),
  invokeLegacyHistoryExportPresetRuntime: vi.fn(async () => true),
  invokeLegacyHistoryImportPresetRuntime: vi.fn(async () => true),
  invokeLegacyHistoryClearSessionRuntime: vi.fn(async () => true),
  invokeLegacyHistorySaveSavedRunTagsRuntime: vi.fn(async () => true),
  invokeLegacyHistoryRefreshSavedRunsRuntime: vi.fn(async () => true),
  invokeLegacyHistoryLoadSavedRunRuntime: vi.fn(async () => true),
  invokeLegacyHistoryApplySavedRunRuntime: vi.fn(async () => true),
  invokeLegacyHistoryRerunSavedRunRuntime: vi.fn(async () => true),
  invokeLegacyHistoryOpenSavedRunReportRuntime: vi.fn(async () => true),
  invokeLegacyHistoryRevealSavedRunRuntime: vi.fn(async () => true),
  invokeLegacyHistoryDeleteSavedRunRuntime: vi.fn(async () => true),
}))

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeLegacyHistoryApplySessionRuntimeMock = vi.mocked(invokeLegacyHistoryApplySessionRuntime)
const invokeLegacyHistoryExportPresetRuntimeMock = vi.mocked(invokeLegacyHistoryExportPresetRuntime)
const invokeLegacyHistoryImportPresetRuntimeMock = vi.mocked(invokeLegacyHistoryImportPresetRuntime)
const invokeLegacyHistoryClearSessionRuntimeMock = vi.mocked(invokeLegacyHistoryClearSessionRuntime)
const invokeLegacyHistorySaveSavedRunTagsRuntimeMock = vi.mocked(invokeLegacyHistorySaveSavedRunTagsRuntime)
const invokeLegacyHistoryRefreshSavedRunsRuntimeMock = vi.mocked(invokeLegacyHistoryRefreshSavedRunsRuntime)
const invokeLegacyHistoryLoadSavedRunRuntimeMock = vi.mocked(invokeLegacyHistoryLoadSavedRunRuntime)
const invokeLegacyHistoryApplySavedRunRuntimeMock = vi.mocked(invokeLegacyHistoryApplySavedRunRuntime)
const invokeLegacyHistoryRerunSavedRunRuntimeMock = vi.mocked(invokeLegacyHistoryRerunSavedRunRuntime)
const invokeLegacyHistoryOpenSavedRunReportRuntimeMock = vi.mocked(invokeLegacyHistoryOpenSavedRunReportRuntime)
const invokeLegacyHistoryRevealSavedRunRuntimeMock = vi.mocked(invokeLegacyHistoryRevealSavedRunRuntime)
const invokeLegacyHistoryDeleteSavedRunRuntimeMock = vi.mocked(invokeLegacyHistoryDeleteSavedRunRuntime)
const invokeCommandMock = vi.mocked(invokeCommand)

interface LegacyField {
  value: string
  disabled?: boolean
  dispatchEvent: (event: Event) => boolean
}

interface DocumentHarness {
  fields: Record<string, LegacyField>
  eventsById: Record<string, string[]>
}

function installDocumentHarness(): DocumentHarness {
  const eventsById: Record<string, string[]> = {}
  const makeField = (id: string, value = ''): LegacyField => ({
    value,
    dispatchEvent: (event: Event) => {
      if (!eventsById[id]) eventsById[id] = []
      eventsById[id].push(String(event.type || '').trim())
      return true
    },
  })
  const fields: Record<string, LegacyField> = {
    savedRunsSearch: makeField('savedRunsSearch', ''),
    historySelect: makeField('historySelect', ''),
  }
  vi.stubGlobal('document', {
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields, eventsById }
}

describe('history store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    harness = installDocumentHarness()
    invokeLegacyHistoryApplySessionRuntimeMock.mockClear()
    invokeLegacyHistoryApplySessionRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistoryExportPresetRuntimeMock.mockClear()
    invokeLegacyHistoryExportPresetRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistoryImportPresetRuntimeMock.mockClear()
    invokeLegacyHistoryImportPresetRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistoryClearSessionRuntimeMock.mockClear()
    invokeLegacyHistoryClearSessionRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistorySaveSavedRunTagsRuntimeMock.mockClear()
    invokeLegacyHistorySaveSavedRunTagsRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistoryRefreshSavedRunsRuntimeMock.mockClear()
    invokeLegacyHistoryRefreshSavedRunsRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistoryLoadSavedRunRuntimeMock.mockClear()
    invokeLegacyHistoryLoadSavedRunRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistoryApplySavedRunRuntimeMock.mockClear()
    invokeLegacyHistoryApplySavedRunRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistoryRerunSavedRunRuntimeMock.mockClear()
    invokeLegacyHistoryRerunSavedRunRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistoryOpenSavedRunReportRuntimeMock.mockClear()
    invokeLegacyHistoryOpenSavedRunReportRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistoryRevealSavedRunRuntimeMock.mockClear()
    invokeLegacyHistoryRevealSavedRunRuntimeMock.mockResolvedValue(true)
    invokeLegacyHistoryDeleteSavedRunRuntimeMock.mockClear()
    invokeLegacyHistoryDeleteSavedRunRuntimeMock.mockResolvedValue(true)
    invokeCommandMock.mockClear()
    invokeCommandMock.mockResolvedValue(undefined)
  })

  it('dispatches history field updates via legacy DOM events', () => {
    const store = useHistoryStore()
    store.setInputField('savedRunsSearch', 'aapl')
    store.setSelectField('historySelect', 'run-1')

    expect(harness.fields.savedRunsSearch?.value).toBe('aapl')
    expect(harness.fields.historySelect?.value).toBe('run-1')
    expect(harness.eventsById.savedRunsSearch).toEqual(['input'])
    expect(harness.eventsById.historySelect).toEqual(['change'])
  })

  it('dispatches direct history session/runtime actions and runs-directory action', () => {
    const store = useHistoryStore()
    store.applySession()
    store.exportPreset()
    store.importPreset()
    store.clearSession()
    store.refreshSavedRuns()
    store.openRunsDir()

    expect(invokeLegacyHistoryApplySessionRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyHistoryExportPresetRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyHistoryImportPresetRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyHistoryClearSessionRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyHistoryRefreshSavedRunsRuntimeMock).toHaveBeenCalled()
    expect(invokeCommandMock).toHaveBeenCalledWith('open_runs_dir')
  })

  it('dispatches saved-run actions with normalized path payload', () => {
    const store = useHistoryStore()
    store.saveSavedRunTags(' C:/runs/run1.json ', 'holdout,1m')
    store.loadSavedRun(' C:/runs/run1.json ')
    store.applySavedRun(' C:/runs/run1.json ')
    store.rerunSavedRun(' C:/runs/run1.json ')
    store.openSavedRunReport(' C:/runs/run1.json ')
    store.revealSavedRun(' C:/runs/run1.json ')
    store.deleteSavedRun(' C:/runs/run1.json ')

    expect(invokeLegacyHistorySaveSavedRunTagsRuntimeMock).toHaveBeenCalledWith({
      path: 'C:/runs/run1.json',
      tags: 'holdout,1m',
    })
    expect(invokeLegacyHistoryLoadSavedRunRuntimeMock).toHaveBeenCalledWith({ path: 'C:/runs/run1.json' })
    expect(invokeLegacyHistoryApplySavedRunRuntimeMock).toHaveBeenCalledWith({ path: 'C:/runs/run1.json' })
    expect(invokeLegacyHistoryRerunSavedRunRuntimeMock).toHaveBeenCalledWith({ path: 'C:/runs/run1.json' })
    expect(invokeLegacyHistoryOpenSavedRunReportRuntimeMock).toHaveBeenCalledWith({ path: 'C:/runs/run1.json' })
    expect(invokeLegacyHistoryRevealSavedRunRuntimeMock).toHaveBeenCalledWith({ path: 'C:/runs/run1.json' })
    expect(invokeLegacyHistoryDeleteSavedRunRuntimeMock).toHaveBeenCalledWith({ path: 'C:/runs/run1.json' })
  })
})
