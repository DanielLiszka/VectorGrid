import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  invokeLegacyDataApplyTimeframeRuntime,
  invokeLegacyDataBrowseCsvRuntime,
  invokeLegacyDataClosePolygonRuntime,
  invokeLegacyDataBrowseMarketDataDirRuntime,
  invokeLegacyDataClearRuntime,
  invokeLegacyDataLoadRuntime,
  invokeLegacyDataLoadSampleCsvRuntime,
  invokeLegacyDataOpenMarketDataDirRuntime,
  invokeLegacyDataOpenPolygonRuntime,
  invokeLegacyDataResetMarketDataDirRuntime,
  invokeLegacyDataSetMarketDataDirRuntime,
} from '../legacy/actions'
import { useDataStore } from './dataStore'

vi.mock('../legacy/actions', () => ({
  invokeLegacyDataApplyTimeframeRuntime: vi.fn(async () => true),
  invokeLegacyDataClearRuntime: vi.fn(async () => true),
  invokeLegacyDataLoadRuntime: vi.fn(async () => true),
  invokeLegacyDataLoadSampleCsvRuntime: vi.fn(async () => true),
  invokeLegacyDataBrowseCsvRuntime: vi.fn(async () => true),
  invokeLegacyDataOpenPolygonRuntime: vi.fn(async () => true),
  invokeLegacyDataClosePolygonRuntime: vi.fn(async () => true),
  invokeLegacyDataBrowseMarketDataDirRuntime: vi.fn(async () => true),
  invokeLegacyDataSetMarketDataDirRuntime: vi.fn(async () => true),
  invokeLegacyDataResetMarketDataDirRuntime: vi.fn(async () => true),
  invokeLegacyDataOpenMarketDataDirRuntime: vi.fn(async () => true),
}))

const invokeLegacyDataApplyTimeframeRuntimeMock = vi.mocked(invokeLegacyDataApplyTimeframeRuntime)
const invokeLegacyDataClearRuntimeMock = vi.mocked(invokeLegacyDataClearRuntime)
const invokeLegacyDataLoadRuntimeMock = vi.mocked(invokeLegacyDataLoadRuntime)
const invokeLegacyDataLoadSampleCsvRuntimeMock = vi.mocked(invokeLegacyDataLoadSampleCsvRuntime)
const invokeLegacyDataBrowseCsvRuntimeMock = vi.mocked(invokeLegacyDataBrowseCsvRuntime)
const invokeLegacyDataOpenPolygonRuntimeMock = vi.mocked(invokeLegacyDataOpenPolygonRuntime)
const invokeLegacyDataClosePolygonRuntimeMock = vi.mocked(invokeLegacyDataClosePolygonRuntime)
const invokeLegacyDataBrowseMarketDataDirRuntimeMock = vi.mocked(invokeLegacyDataBrowseMarketDataDirRuntime)
const invokeLegacyDataSetMarketDataDirRuntimeMock = vi.mocked(invokeLegacyDataSetMarketDataDirRuntime)
const invokeLegacyDataResetMarketDataDirRuntimeMock = vi.mocked(invokeLegacyDataResetMarketDataDirRuntime)
const invokeLegacyDataOpenMarketDataDirRuntimeMock = vi.mocked(invokeLegacyDataOpenMarketDataDirRuntime)

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
    csvPath: {
      value: '',
      dispatchEvent: (event: Event) => trackEvent('csvPath', event),
    },
    dataTimeframe: {
      value: '',
      dispatchEvent: (event: Event) => trackEvent('dataTimeframe', event),
    },
    autoRestoreDataset: {
      checked: false,
      type: 'checkbox',
      dispatchEvent: (event: Event) => trackEvent('autoRestoreDataset', event),
    },
  }
  vi.stubGlobal('document', {
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields, eventsById }
}

describe('data store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    harness = installDocumentHarness()
    invokeLegacyDataApplyTimeframeRuntimeMock.mockClear()
    invokeLegacyDataApplyTimeframeRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataClearRuntimeMock.mockClear()
    invokeLegacyDataClearRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataLoadRuntimeMock.mockClear()
    invokeLegacyDataLoadRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataLoadSampleCsvRuntimeMock.mockClear()
    invokeLegacyDataLoadSampleCsvRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataBrowseCsvRuntimeMock.mockClear()
    invokeLegacyDataBrowseCsvRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataOpenPolygonRuntimeMock.mockClear()
    invokeLegacyDataOpenPolygonRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataClosePolygonRuntimeMock.mockClear()
    invokeLegacyDataClosePolygonRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataBrowseMarketDataDirRuntimeMock.mockClear()
    invokeLegacyDataBrowseMarketDataDirRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataSetMarketDataDirRuntimeMock.mockClear()
    invokeLegacyDataSetMarketDataDirRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataResetMarketDataDirRuntimeMock.mockClear()
    invokeLegacyDataResetMarketDataDirRuntimeMock.mockResolvedValue(true)
    invokeLegacyDataOpenMarketDataDirRuntimeMock.mockClear()
    invokeLegacyDataOpenMarketDataDirRuntimeMock.mockResolvedValue(true)
  })

  it('dispatches typed field mutations via legacy DOM events', () => {
    const store = useDataStore()
    store.setInputField('csvPath', 'C:/tmp/aapl.csv')
    store.setSelectField('dataTimeframe', '5m')
    store.setCheckboxField('autoRestoreDataset', true)

    const autoRestore = harness.fields.autoRestoreDataset as LegacyCheckboxField
    expect((harness.fields.csvPath as LegacyTextField).value).toBe('C:/tmp/aapl.csv')
    expect((harness.fields.dataTimeframe as LegacyTextField).value).toBe('5m')
    expect(autoRestore.checked).toBe(true)
    expect(harness.eventsById.csvPath).toEqual(['input'])
    expect(harness.eventsById.dataTimeframe).toEqual(['change'])
    expect(harness.eventsById.autoRestoreDataset).toEqual(['change'])
  })

  it('dispatches direct data-load runtime wrapper with optional path payload', () => {
    const store = useDataStore()
    store.loadCsv(' C:/tmp/msft.csv ')
    store.loadCsv('')

    expect(invokeLegacyDataLoadRuntimeMock).toHaveBeenNthCalledWith(1, { path: 'C:/tmp/msft.csv' })
    expect(invokeLegacyDataLoadRuntimeMock).toHaveBeenNthCalledWith(2)
  })

  it('dispatches direct market-data directory/sample runtime wrappers', () => {
    const store = useDataStore()
    store.browseCsv()
    store.openPolygon()
    store.closePolygon()
    store.setMarketDataDir(' C:/cache ', true)
    store.browseMarketDataDir()
    store.resetMarketDataDir()
    store.openMarketDataDir()
    store.loadSampleCsv(2, '')
    store.loadSampleCsv(1, ' C:/samples/aapl.csv ')

    expect(invokeLegacyDataBrowseCsvRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataOpenPolygonRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataClosePolygonRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataSetMarketDataDirRuntimeMock).toHaveBeenNthCalledWith(1, {
      dir: 'C:/cache',
      moveExisting: true,
    })
    expect(invokeLegacyDataBrowseMarketDataDirRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataResetMarketDataDirRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataOpenMarketDataDirRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDataLoadSampleCsvRuntimeMock).toHaveBeenCalledWith({ index: 2 })
    expect(invokeLegacyDataLoadRuntimeMock).toHaveBeenCalledWith({ path: 'C:/samples/aapl.csv' })
  })

  it('dispatches direct data runtime wrappers for clear/apply-timeframe actions', () => {
    const store = useDataStore()
    store.applyTimeframe(' 15m ')
    store.clearData()

    expect(invokeLegacyDataApplyTimeframeRuntimeMock).toHaveBeenCalledWith({ timeframe: '15m' })
    expect(invokeLegacyDataClearRuntimeMock).toHaveBeenCalled()
  })
})
