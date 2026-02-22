import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  invokeLegacyOptimizeValidateRuntime,
  invokeLegacyValidationCrossRefreshRuntime,
  invokeLegacyValidationCrossSelectAllRuntime,
  invokeLegacyValidationCrossSelectMatchRuntime,
  invokeLegacyValidationCrossSelectNoneRuntime,
  invokeLegacyValidationExportReportJsonRuntime,
  invokeLegacyValidationExportReportXlsxRuntime,
  invokeLegacyValidationExportShortlistCsvRuntime,
  invokeLegacyValidationExportValidationJsonRuntime,
  invokeLegacyValidationToggleCrossDatasetRuntime,
} from '../legacy/actions'
import { invokeCommand } from '../tauri/invoke'
import { useValidationStore } from './validationStore'

vi.mock('../legacy/actions', () => ({
  invokeLegacyOptimizeValidateRuntime: vi.fn(async () => true),
  invokeLegacyValidationExportShortlistCsvRuntime: vi.fn(async () => true),
  invokeLegacyValidationExportValidationJsonRuntime: vi.fn(async () => true),
  invokeLegacyValidationExportReportJsonRuntime: vi.fn(async () => true),
  invokeLegacyValidationExportReportXlsxRuntime: vi.fn(async () => true),
  invokeLegacyValidationCrossRefreshRuntime: vi.fn(async () => true),
  invokeLegacyValidationCrossSelectAllRuntime: vi.fn(async () => true),
  invokeLegacyValidationCrossSelectNoneRuntime: vi.fn(async () => true),
  invokeLegacyValidationCrossSelectMatchRuntime: vi.fn(async () => true),
  invokeLegacyValidationToggleCrossDatasetRuntime: vi.fn(async () => true),
}))

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeLegacyOptimizeValidateRuntimeMock = vi.mocked(invokeLegacyOptimizeValidateRuntime)
const invokeLegacyValidationExportShortlistCsvRuntimeMock = vi.mocked(invokeLegacyValidationExportShortlistCsvRuntime)
const invokeLegacyValidationExportValidationJsonRuntimeMock = vi.mocked(invokeLegacyValidationExportValidationJsonRuntime)
const invokeLegacyValidationExportReportJsonRuntimeMock = vi.mocked(invokeLegacyValidationExportReportJsonRuntime)
const invokeLegacyValidationExportReportXlsxRuntimeMock = vi.mocked(invokeLegacyValidationExportReportXlsxRuntime)
const invokeLegacyValidationCrossRefreshRuntimeMock = vi.mocked(invokeLegacyValidationCrossRefreshRuntime)
const invokeLegacyValidationCrossSelectAllRuntimeMock = vi.mocked(invokeLegacyValidationCrossSelectAllRuntime)
const invokeLegacyValidationCrossSelectNoneRuntimeMock = vi.mocked(invokeLegacyValidationCrossSelectNoneRuntime)
const invokeLegacyValidationCrossSelectMatchRuntimeMock = vi.mocked(invokeLegacyValidationCrossSelectMatchRuntime)
const invokeLegacyValidationToggleCrossDatasetRuntimeMock = vi.mocked(invokeLegacyValidationToggleCrossDatasetRuntime)
const invokeCommandMock = vi.mocked(invokeCommand)

interface LegacyTextField {
  value: string
  disabled?: boolean
  type?: string
  dispatchEvent: (event: Event) => boolean
}

interface LegacyCheckboxField {
  checked: boolean
  disabled?: boolean
  type: 'checkbox'
  dispatchEvent: (event: Event) => boolean
}

interface DocumentHarness {
  fields: Record<string, LegacyTextField | LegacyCheckboxField>
  eventsById: Record<string, string[]>
}

function installDocumentHarness(): DocumentHarness {
  const eventsById: Record<string, string[]> = {}
  const trackEvent = (id: string, event: Event): boolean => {
    if (!eventsById[id]) eventsById[id] = []
    eventsById[id].push(String(event.type || '').trim())
    return true
  }
  const fields: Record<string, LegacyTextField | LegacyCheckboxField> = {
    valCandidates: {
      value: '32',
      dispatchEvent: (event: Event) => trackEvent('valCandidates', event),
    },
    valAutoRun: {
      checked: false,
      type: 'checkbox',
      dispatchEvent: (event: Event) => trackEvent('valAutoRun', event),
    },
  }
  vi.stubGlobal('document', {
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields, eventsById }
}

describe('validation store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    harness = installDocumentHarness()
    invokeLegacyOptimizeValidateRuntimeMock.mockClear()
    invokeLegacyOptimizeValidateRuntimeMock.mockResolvedValue(true)
    invokeLegacyValidationExportShortlistCsvRuntimeMock.mockClear()
    invokeLegacyValidationExportShortlistCsvRuntimeMock.mockResolvedValue(true)
    invokeLegacyValidationExportValidationJsonRuntimeMock.mockClear()
    invokeLegacyValidationExportValidationJsonRuntimeMock.mockResolvedValue(true)
    invokeLegacyValidationExportReportJsonRuntimeMock.mockClear()
    invokeLegacyValidationExportReportJsonRuntimeMock.mockResolvedValue(true)
    invokeLegacyValidationExportReportXlsxRuntimeMock.mockClear()
    invokeLegacyValidationExportReportXlsxRuntimeMock.mockResolvedValue(true)
    invokeLegacyValidationCrossRefreshRuntimeMock.mockClear()
    invokeLegacyValidationCrossRefreshRuntimeMock.mockResolvedValue(true)
    invokeLegacyValidationCrossSelectAllRuntimeMock.mockClear()
    invokeLegacyValidationCrossSelectAllRuntimeMock.mockResolvedValue(true)
    invokeLegacyValidationCrossSelectNoneRuntimeMock.mockClear()
    invokeLegacyValidationCrossSelectNoneRuntimeMock.mockResolvedValue(true)
    invokeLegacyValidationCrossSelectMatchRuntimeMock.mockClear()
    invokeLegacyValidationCrossSelectMatchRuntimeMock.mockResolvedValue(true)
    invokeLegacyValidationToggleCrossDatasetRuntimeMock.mockClear()
    invokeLegacyValidationToggleCrossDatasetRuntimeMock.mockResolvedValue(true)
    invokeCommandMock.mockClear()
    invokeCommandMock.mockResolvedValue(undefined)
  })

  it('dispatches validation field updates via legacy DOM events', () => {
    const store = useValidationStore()
    store.setInputField('valCandidates', '64')
    store.setCheckboxField('valAutoRun', true)

    const autoRun = harness.fields.valAutoRun as LegacyCheckboxField
    expect((harness.fields.valCandidates as LegacyTextField).value).toBe('64')
    expect(autoRun.checked).toBe(true)
    expect(harness.eventsById.valCandidates).toEqual(['input'])
    expect(harness.eventsById.valAutoRun).toEqual(['change'])
  })

  it('dispatches validate and export actions', () => {
    const store = useValidationStore()
    store.runValidation()
    store.exportShortlistCsv()
    store.exportValidationJson()
    store.exportReportJson()
    store.exportReportXlsx()
    store.openReportsDir()

    expect(invokeLegacyOptimizeValidateRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyValidationExportShortlistCsvRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyValidationExportValidationJsonRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyValidationExportReportJsonRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyValidationExportReportXlsxRuntimeMock).toHaveBeenCalled()
    expect(invokeCommandMock).toHaveBeenCalledWith('open_reports_dir')
  })

  it('dispatches cross-dataset actions', () => {
    const store = useValidationStore()
    store.crossRefresh()
    store.crossSelectAll()
    store.crossSelectNone()
    store.crossSelectMatch()
    store.toggleCrossDataset(' C:/data/aapl.csv ', true)
    store.toggleCrossDataset('', false)

    expect(invokeLegacyValidationCrossRefreshRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyValidationCrossSelectAllRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyValidationCrossSelectNoneRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyValidationCrossSelectMatchRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyValidationToggleCrossDatasetRuntimeMock).toHaveBeenCalledWith({
      path: 'C:/data/aapl.csv',
      checked: true,
    })
    expect(invokeLegacyValidationToggleCrossDatasetRuntimeMock).not.toHaveBeenCalledWith({
      path: '',
      checked: false,
    })
  })
})
