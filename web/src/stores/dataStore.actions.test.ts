import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  applyTimeframe,
  browseCsv,
  clearPolygonApiKey,
  type DataLoadResult,
  type DataSampleCsvSummary,
  getMarketDataDirInfo,
  getPolygonCacheInfo,
  type MarketDataDirInfoSummary,
  type MarketDataDirOpenResult,
  type PolygonCacheInfoSummary,
  type PolygonDatasetSummary,
  browseMarketDataDir,
  clearData,
  closePolygon,
  downloadPolygonDataset,
  listSampleCsvs,
  loadCsv,
  loadSampleCsv,
  openMarketDataDir,
  openPolygon,
  repairPolygonDataset,
  resetMarketDataDir,
  setPolygonApiKey,
  setMarketDataDir,
  validatePolygonDataset,
} from '../services/dataService'
import { useDataStore } from './dataStore'

vi.mock('../services/dataService', () => ({
  applyTimeframe: vi.fn(async () => ({
    path: '',
    dataId: 'data-1@15m',
    cacheHit: false,
    cachePath: '',
    meta: {
      len: 100,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1_700_000_000,
      maxTs: 1_700_100_000,
      timeframe: '15m',
      sourceSizeBytes: null,
      sourceMtimeUtcMs: null,
    },
  })),
  browseCsv: vi.fn(async () => null),
  browseMarketDataDir: vi.fn(async () => ''),
  clearPolygonApiKey: vi.fn(async () => ({
    cacheDir: 'C:/default/polygon-cache',
    hasApiKey: false,
    datasets: [],
  })),
  clearData: vi.fn(async () => undefined),
  closePolygon: vi.fn(async () => undefined),
  downloadPolygonDataset: vi.fn(async () => ({
    csvPath: 'C:/default/polygon-cache/AAPL_1m.csv',
    downloadedBars: 1500,
    usedCacheOnly: false,
    load: {
      path: 'C:/default/polygon-cache/AAPL_1m.csv',
      dataId: 'poly-1',
      cacheHit: false,
      cachePath: 'C:/cache/poly-1.bin',
      meta: {
        len: 1500,
        hasOpen: true,
        hasHigh: true,
        hasLow: true,
        hasClose: true,
        hasVolume: true,
        minTs: 1_700_000_000,
        maxTs: 1_700_500_000,
        timeframe: '1m',
        sourceSizeBytes: 123,
        sourceMtimeUtcMs: 456,
      },
    },
  })),
  getMarketDataDirInfo: vi.fn(async () => ({
    defaultRootDir: 'C:/default',
    configuredRootDir: '',
    effectiveRootDir: 'C:/default',
    polygonCacheDir: 'C:/default/polygon-cache',
    hint: 'Using default root: C:/default · Polygon cache: C:/default/polygon-cache',
  })),
  getPolygonCacheInfo: vi.fn(async () => ({
    cacheDir: 'C:/default/polygon-cache',
    hasApiKey: true,
    datasets: [{ market: 'stocks', displaySymbol: 'AAPL', polygonTicker: 'AAPL', timeframe: '1m', csvPath: 'C:/default/polygon-cache/AAPL_1m.csv', minTs: 1700000000, maxTs: 1700005000, lastUpdatedUtcMs: 1700005000000, rowCount: 1500, adjusted: true, integrity: null }],
  })),
  listSampleCsvs: vi.fn(async () => []),
  loadCsv: vi.fn(async () => ({
    path: 'C:/tmp/aapl.csv',
    dataId: 'data-1',
    cacheHit: false,
    cachePath: 'C:/cache/data-1.bin',
    meta: {
      len: 500,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1_700_000_000,
      maxTs: 1_700_500_000,
      timeframe: '1m',
      sourceSizeBytes: 123,
      sourceMtimeUtcMs: 456,
    },
  })),
  loadSampleCsv: vi.fn(async () => null),
  openMarketDataDir: vi.fn(async () => ({ path: 'C:/default/polygon-cache', openedWith: 'open_path' })),
  openPolygon: vi.fn(async () => undefined),
  repairPolygonDataset: vi.fn(async () => ({
    market: 'stocks',
    displaySymbol: 'AAPL',
    polygonTicker: 'AAPL',
    timeframe: '1m',
    csvPath: 'C:/default/polygon-cache/AAPL_1m.csv',
    minTs: 1700000000,
    maxTs: 1700005000,
    lastUpdatedUtcMs: 1700005000000,
    rowCount: 1500,
    adjusted: true,
    integrity: { checkedUtcMs: 1700005000000, parseErrors: 0, duplicates: 0, outOfOrder: 0, sameDayGapsGt2Min: 0 },
  })),
  resetMarketDataDir: vi.fn(async () => ({
    defaultRootDir: 'C:/default',
    configuredRootDir: '',
    effectiveRootDir: 'C:/default',
    polygonCacheDir: 'C:/default/polygon-cache',
    hint: 'Using default root: C:/default · Polygon cache: C:/default/polygon-cache',
  })),
  setMarketDataDir: vi.fn(async () => ({
    defaultRootDir: 'C:/default',
    configuredRootDir: 'C:/custom',
    effectiveRootDir: 'C:/custom',
    polygonCacheDir: 'C:/custom/polygon-cache',
    hint: 'Using custom root: C:/custom · Polygon cache: C:/custom/polygon-cache',
  })),
  setPolygonApiKey: vi.fn(async () => ({
    cacheDir: 'C:/default/polygon-cache',
    hasApiKey: true,
    datasets: [],
  })),
  validatePolygonDataset: vi.fn(async () => ({
    market: 'stocks',
    displaySymbol: 'AAPL',
    polygonTicker: 'AAPL',
    timeframe: '1m',
    csvPath: 'C:/default/polygon-cache/AAPL_1m.csv',
    minTs: 1700000000,
    maxTs: 1700005000,
    lastUpdatedUtcMs: 1700005000000,
    rowCount: 1500,
    adjusted: true,
    integrity: { checkedUtcMs: 1700005000000, parseErrors: 0, duplicates: 1, outOfOrder: 0, sameDayGapsGt2Min: 0 },
  })),
}))

const applyTimeframeMock = vi.mocked(applyTimeframe)
const clearDataMock = vi.mocked(clearData)
const loadCsvMock = vi.mocked(loadCsv)
const loadSampleCsvMock = vi.mocked(loadSampleCsv)
const browseCsvMock = vi.mocked(browseCsv)
const openPolygonMock = vi.mocked(openPolygon)
const closePolygonMock = vi.mocked(closePolygon)
const getMarketDataDirInfoMock = vi.mocked(getMarketDataDirInfo)
const listSampleCsvsMock = vi.mocked(listSampleCsvs)
const browseMarketDataDirMock = vi.mocked(browseMarketDataDir)
const clearPolygonApiKeyMock = vi.mocked(clearPolygonApiKey)
const setMarketDataDirMock = vi.mocked(setMarketDataDir)
const resetMarketDataDirMock = vi.mocked(resetMarketDataDir)
const openMarketDataDirMock = vi.mocked(openMarketDataDir)
const getPolygonCacheInfoMock = vi.mocked(getPolygonCacheInfo)
const setPolygonApiKeyMock = vi.mocked(setPolygonApiKey)
const downloadPolygonDatasetMock = vi.mocked(downloadPolygonDataset)
const validatePolygonDatasetMock = vi.mocked(validatePolygonDataset)
const repairPolygonDatasetMock = vi.mocked(repairPolygonDataset)

const defaultMarketDataDirInfo: MarketDataDirInfoSummary = {
  defaultRootDir: 'C:/default',
  configuredRootDir: '',
  effectiveRootDir: 'C:/default',
  polygonCacheDir: 'C:/default/polygon-cache',
  hint: 'Using default root: C:/default · Polygon cache: C:/default/polygon-cache',
}

const defaultMarketDataOpenResult: MarketDataDirOpenResult = {
  path: 'C:/default/polygon-cache',
  openedWith: 'open_path',
}

const defaultSampleRows: DataSampleCsvSummary[] = [
  { label: 'aapl.csv', path: 'C:/samples/aapl.csv', disabled: false },
  { label: 'msft.csv', path: 'C:/samples/msft.csv', disabled: false },
]

const defaultLoadResult: DataLoadResult = {
  path: 'C:/tmp/aapl.csv',
  dataId: 'data-1',
  cacheHit: false,
  cachePath: 'C:/cache/data-1.bin',
  meta: {
    len: 500,
    hasOpen: true,
    hasHigh: true,
    hasLow: true,
    hasClose: true,
    hasVolume: true,
    minTs: 1_700_000_000,
    maxTs: 1_700_500_000,
    timeframe: '1m',
    sourceSizeBytes: 123,
    sourceMtimeUtcMs: 456,
  },
}

const defaultPolygonDataset: PolygonDatasetSummary = {
  market: 'stocks',
  displaySymbol: 'AAPL',
  polygonTicker: 'AAPL',
  timeframe: '1m',
  csvPath: 'C:/default/polygon-cache/AAPL_1m.csv',
  minTs: 1_700_000_000,
  maxTs: 1_700_500_000,
  lastUpdatedUtcMs: 1_700_005_000_000,
  rowCount: 1500,
  adjusted: true,
  integrity: null,
}

const defaultPolygonCacheInfo: PolygonCacheInfoSummary = {
  cacheDir: 'C:/default/polygon-cache',
  hasApiKey: true,
  datasets: [{ ...defaultPolygonDataset }],
}

async function flushAsync(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}

describe('data store actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    applyTimeframeMock.mockClear()
    clearDataMock.mockClear()
    loadCsvMock.mockClear()
    loadSampleCsvMock.mockClear()
    browseCsvMock.mockClear()
    openPolygonMock.mockClear()
    closePolygonMock.mockClear()
    getMarketDataDirInfoMock.mockClear()
    listSampleCsvsMock.mockClear()
    browseMarketDataDirMock.mockClear()
    clearPolygonApiKeyMock.mockClear()
    setMarketDataDirMock.mockClear()
    resetMarketDataDirMock.mockClear()
    openMarketDataDirMock.mockClear()
    getPolygonCacheInfoMock.mockClear()
    setPolygonApiKeyMock.mockClear()
    downloadPolygonDatasetMock.mockClear()
    validatePolygonDatasetMock.mockClear()
    repairPolygonDatasetMock.mockClear()

    applyTimeframeMock.mockResolvedValue({
      ...defaultLoadResult,
      path: '',
      dataId: 'data-1@15m',
      cacheHit: false,
      cachePath: '',
      meta: { ...defaultLoadResult.meta, len: 90, timeframe: '15m' },
    })
    clearDataMock.mockResolvedValue(undefined)
    loadCsvMock.mockResolvedValue({ ...defaultLoadResult })
    loadSampleCsvMock.mockResolvedValue({ ...defaultLoadResult, path: 'C:/samples/10k.csv' })
    browseCsvMock.mockResolvedValue({ ...defaultLoadResult, path: 'C:/picked.csv' })
    openPolygonMock.mockResolvedValue(undefined)
    closePolygonMock.mockResolvedValue(undefined)
    getMarketDataDirInfoMock.mockResolvedValue(defaultMarketDataDirInfo)
    listSampleCsvsMock.mockResolvedValue(defaultSampleRows)
    browseMarketDataDirMock.mockResolvedValue('')
    clearPolygonApiKeyMock.mockResolvedValue({ ...defaultPolygonCacheInfo, hasApiKey: false, datasets: [] })
    setMarketDataDirMock.mockResolvedValue(defaultMarketDataDirInfo)
    resetMarketDataDirMock.mockResolvedValue(defaultMarketDataDirInfo)
    openMarketDataDirMock.mockResolvedValue(defaultMarketDataOpenResult)
    getPolygonCacheInfoMock.mockResolvedValue({ ...defaultPolygonCacheInfo })
    setPolygonApiKeyMock.mockResolvedValue({ ...defaultPolygonCacheInfo, datasets: [] })
    downloadPolygonDatasetMock.mockResolvedValue({
      csvPath: defaultPolygonDataset.csvPath,
      downloadedBars: 1500,
      usedCacheOnly: false,
      baseLoad: { ...defaultLoadResult, path: defaultPolygonDataset.csvPath, dataId: 'poly-1-base' },
      load: {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: 'poly-1',
        meta: { ...defaultLoadResult.meta, timeframe: '15m', len: 100 },
      },
    })
    validatePolygonDatasetMock.mockResolvedValue({
      ...defaultPolygonDataset,
      integrity: { checkedUtcMs: 1_700_005_000_000, parseErrors: 0, duplicates: 1, outOfOrder: 0, sameDayGapsGt2Min: 0 },
    })
    repairPolygonDatasetMock.mockResolvedValue({
      ...defaultPolygonDataset,
      integrity: { checkedUtcMs: 1_700_005_000_000, parseErrors: 0, duplicates: 0, outOfOrder: 0, sameDayGapsGt2Min: 0 },
    })
  })

  it('updates Vue data state without legacy DOM mirroring', () => {
    const store = useDataStore()
    store.setInputField('csvPath', 'C:/tmp/aapl.csv')
    store.setSelectField('dataTimeframe', '5m')
    store.setCheckboxField('autoRestoreDataset', true)

    expect(store.csvPath).toBe('C:/tmp/aapl.csv')
    expect(store.dataTimeframe).toBe('5m')
    expect(store.autoRestoreDataset).toBe(true)
    expect(store.source).toBe('vue')
  })

  it('loads CSV and applies Vue-owned dataset state', async () => {
    const store = useDataStore()

    store.loadCsv(' C:/tmp/msft.csv ')
    await flushAsync()

    expect(loadCsvMock).toHaveBeenCalledWith('C:/tmp/msft.csv')
    expect(store.baseDataId).toBe('data-1')
    expect(store.dataId).toBe('data-1')
    expect(store.datasetLabel).toBe('aapl.csv')
    expect(store.datasetContext).toContain('source=csv')
    expect(store.dataLen).toBe('500')
    expect(store.canApplyTimeframe).toBe(true)
  })

  it('applies timeframe using base data id and supports reset to base timeframe', async () => {
    const store = useDataStore()
    store.loadCsv('C:/tmp/aapl.csv')
    await flushAsync()

    store.applyTimeframe('15m')
    await flushAsync()

    expect(applyTimeframeMock).toHaveBeenCalledWith('15m', 'data-1')
    expect(store.dataId).toBe('data-1@15m')
    expect(store.dataTimeframe).toBe('15m')
    expect(store.datasetRange).toContain('15m')

    store.applyTimeframe('')
    await flushAsync()

    expect(store.dataId).toBe('data-1')
    expect(store.dataTimeframe).toBe('')
    expect(store.datasetRange).toContain('1m')
  })

  it('dispatches market-data directory and sample actions through data service', async () => {
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
    await flushAsync()

    expect(browseCsvMock).toHaveBeenCalled()
    expect(openPolygonMock).toHaveBeenCalled()
    expect(closePolygonMock).toHaveBeenCalled()
    expect(setMarketDataDirMock).toHaveBeenNthCalledWith(1, 'C:/cache', true)
    expect(browseMarketDataDirMock).toHaveBeenCalled()
    expect(resetMarketDataDirMock).toHaveBeenCalled()
    expect(openMarketDataDirMock).toHaveBeenCalled()
    expect(loadSampleCsvMock).toHaveBeenNthCalledWith(1, 2, '')
    expect(loadSampleCsvMock).toHaveBeenNthCalledWith(2, 1, ' C:/samples/aapl.csv ')
  })

  it('opens, refreshes, and closes the Vue-native Polygon workflow', async () => {
    const store = useDataStore()

    store.openPolygon()
    await flushAsync()
    expect(store.polygonOpen).toBe(true)
    expect(openPolygonMock).toHaveBeenCalled()
    expect(getPolygonCacheInfoMock).toHaveBeenCalled()
    expect(store.polygonHasApiKey).toBe(true)
    expect(store.polygonDatasets.length).toBe(1)

    store.closePolygon()
    await flushAsync()
    expect(store.polygonOpen).toBe(false)
    expect(closePolygonMock).toHaveBeenCalled()
  })

  it('saves API keys and downloads/loads Polygon data through the store', async () => {
    const store = useDataStore()
    store.setInputField('polygonApiKey', 'secret')
    store.setInputField('polygonSymbol', 'aapl')
    store.setInputField('polygonFrom', '2024-01-01')
    store.setInputField('polygonTo', '2024-01-31')
    store.setSelectField('polygonTimeframe', '15m')

    store.savePolygonApiKey()
    await flushAsync()
    expect(setPolygonApiKeyMock).toHaveBeenCalledWith('secret')
    expect(store.polygonHasApiKey).toBe(true)

    store.downloadPolygonDataset()
    await flushAsync()
    expect(downloadPolygonDatasetMock).toHaveBeenCalledWith({
      market: 'stocks',
      symbol: 'AAPL',
      from: '2024-01-01',
      to: '2024-01-31',
      adjusted: true,
      timeframe: '15m',
    })
    expect(store.dataId).toBe('poly-1')
    expect(store.baseDataId).toBe('poly-1-base')
    expect(store.dataTimeframe).toBe('15m')
    expect(store.datasetContext).toContain('source=polygon')
  })

  it('validates and repairs the selected Polygon dataset', async () => {
    const store = useDataStore()
    store.openPolygon()
    await flushAsync()

    store.validatePolygonDataset()
    await flushAsync()
    expect(validatePolygonDatasetMock).toHaveBeenCalledWith(defaultPolygonDataset.csvPath)
    expect(store.polygonSelectedMeta).toContain('dup=1')

    store.repairPolygonDataset()
    await flushAsync()
    expect(repairPolygonDatasetMock).toHaveBeenCalledWith(defaultPolygonDataset.csvPath)
    expect(store.polygonSelectedMeta).toContain('dup=0')

    store.clearPolygonApiKey()
    await flushAsync()
    expect(clearPolygonApiKeyMock).toHaveBeenCalled()
    expect(store.polygonHasApiKey).toBe(false)
  })

  it('hydrates startup data using Vue-owned data bootstrap services', async () => {
    const store = useDataStore()

    store.hydrateStartupState()
    await flushAsync()

    expect(getMarketDataDirInfoMock).toHaveBeenCalled()
    expect(listSampleCsvsMock).toHaveBeenCalled()
    expect(getPolygonCacheInfoMock).toHaveBeenCalled()
    expect(store.marketDataDir).toBe('')
    expect(store.marketDataDirHint).toContain('Using default root')
    expect(store.sampleCsvVisible).toBe(true)
    expect(store.sampleCsvOptions).toEqual(defaultSampleRows)
    expect(store.polygonCacheDir).toBe(defaultPolygonCacheInfo.cacheDir)
  })

  it('clears data state while still calling data service clear hook', async () => {
    const store = useDataStore()
    store.loadCsv('C:/tmp/aapl.csv')
    await flushAsync()

    store.clearData()
    await flushAsync()

    expect(clearDataMock).toHaveBeenCalled()
    expect(store.dataId).toBe('—')
    expect(store.baseDataId).toBe('')
    expect(store.datasetLabel).toBe('—')
    expect(store.dataFields).toEqual([])
    expect(store.canApplyTimeframe).toBe(false)
  })
})
