import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  applyTimeframe,
  browseCsv,
  type DataLoadResult,
  type DataSampleCsvSummary,
  getMarketDataDirInfo,
  type MarketDataDirInfoSummary,
  type MarketDataDirOpenResult,
  browseMarketDataDir,
  clearData,
  closePolygon,
  listSampleCsvs,
  loadCsv,
  loadSampleCsv,
  openMarketDataDir,
  openPolygon,
  resetMarketDataDir,
  setMarketDataDir,
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
  clearData: vi.fn(async () => undefined),
  closePolygon: vi.fn(async () => undefined),
  getMarketDataDirInfo: vi.fn(async () => ({
    defaultRootDir: 'C:/default',
    configuredRootDir: '',
    effectiveRootDir: 'C:/default',
    polygonCacheDir: 'C:/default/polygon-cache',
    hint: 'Using default root: C:/default · Polygon cache: C:/default/polygon-cache',
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
const setMarketDataDirMock = vi.mocked(setMarketDataDir)
const resetMarketDataDirMock = vi.mocked(resetMarketDataDir)
const openMarketDataDirMock = vi.mocked(openMarketDataDir)

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
    setMarketDataDirMock.mockClear()
    resetMarketDataDirMock.mockClear()
    openMarketDataDirMock.mockClear()

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
    setMarketDataDirMock.mockResolvedValue(defaultMarketDataDirInfo)
    resetMarketDataDirMock.mockResolvedValue(defaultMarketDataDirInfo)
    openMarketDataDirMock.mockResolvedValue(defaultMarketDataOpenResult)
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

  it('hydrates startup data using Vue-owned data bootstrap services', async () => {
    const store = useDataStore()

    store.hydrateStartupState()
    await flushAsync()

    expect(getMarketDataDirInfoMock).toHaveBeenCalled()
    expect(listSampleCsvsMock).toHaveBeenCalled()
    expect(store.marketDataDir).toBe('')
    expect(store.marketDataDirHint).toContain('Using default root')
    expect(store.sampleCsvVisible).toBe(true)
    expect(store.sampleCsvOptions).toEqual(defaultSampleRows)
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
