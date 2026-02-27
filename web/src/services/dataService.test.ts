import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invokeCommand } from '../tauri/invoke'
import {
  applyTimeframe,
  browseCsv,
  browseMarketDataDir,
  closePolygon,
  getMarketDataDirInfo,
  listSampleCsvs,
  loadCsv,
  loadSampleCsv,
  openMarketDataDir,
  openPolygon,
  resetMarketDataDir,
  setMarketDataDir,
} from './dataService'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => undefined),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

describe('dataService market-data directory actions', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('normalizes market-data directory info and hint text', async () => {
    invokeCommandMock.mockResolvedValueOnce({
      default_root_dir: 'C:/default',
      configured_root_dir: 'C:/custom',
      effective_root_dir: 'C:/effective',
      polygon_cache_dir: 'C:/effective/polygon-cache',
    })

    const info = await getMarketDataDirInfo()

    expect(invokeCommandMock).toHaveBeenCalledWith('get_market_data_dir_info')
    expect(info.defaultRootDir).toBe('C:/default')
    expect(info.configuredRootDir).toBe('C:/custom')
    expect(info.effectiveRootDir).toBe('C:/effective')
    expect(info.polygonCacheDir).toBe('C:/effective/polygon-cache')
    expect(info.hint).toContain('Using custom root: C:/custom')
  })

  it('returns trimmed picked folder path', async () => {
    invokeCommandMock.mockResolvedValueOnce(' C:/cache ')

    const path = await browseMarketDataDir()

    expect(invokeCommandMock).toHaveBeenCalledWith('pick_folder')
    expect(path).toBe('C:/cache')
  })

  it('picks CSV and loads via load_price_data command', async () => {
    invokeCommandMock
      .mockResolvedValueOnce(' C:/sample.csv ')
      .mockResolvedValueOnce({
        id: 'data-1',
        meta: {
          len: 123,
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
          min_ts: 1_700_000_000,
          max_ts: 1_700_000_999,
          timeframe: '1m',
        },
        cache_hit: false,
        cache_path: 'C:/cache/a.bin',
      })

    const result = await browseCsv()

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'pick_csv_file')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'load_price_data', { path: 'C:/sample.csv' })
    expect(result?.path).toBe('C:/sample.csv')
    expect(result?.dataId).toBe('data-1')
    expect(result?.meta.timeframe).toBe('1m')
  })

  it('loads CSV directly and normalizes response shape', async () => {
    invokeCommandMock.mockResolvedValueOnce({
      id: 'data-2',
      meta: {
        len: '250',
        has_open: 1,
        has_high: true,
        has_low: true,
        has_close: true,
        has_volume: 0,
        min_ts: '1700000010',
        max_ts: '1700000510',
        timeframe: '5m',
      },
      cache_hit: true,
      cache_path: 'C:/cache/data-2.bin',
    })

    const result = await loadCsv(' C:/raw.csv ')

    expect(invokeCommandMock).toHaveBeenCalledWith('load_price_data', { path: 'C:/raw.csv' })
    expect(result.path).toBe('C:/raw.csv')
    expect(result.dataId).toBe('data-2')
    expect(result.cacheHit).toBe(true)
    expect(result.meta.len).toBe(250)
    expect(result.meta.hasVolume).toBe(false)
    expect(result.meta.timeframe).toBe('5m')
  })

  it('applies timeframe using base data id', async () => {
    invokeCommandMock.mockResolvedValueOnce({
      id: 'data-2@15m',
      meta: { len: 100, timeframe: '15m', has_close: true },
      cache_hit: false,
      cache_path: '',
    })

    const result = await applyTimeframe('15m', 'data-2')

    expect(invokeCommandMock).toHaveBeenCalledWith('resample_price_data', {
      baseDataId: 'data-2',
      timeframe: '15m',
    })
    expect(result.dataId).toBe('data-2@15m')
    expect(result.meta.timeframe).toBe('15m')
  })

  it('loads sample CSV by index using list_sample_csvs command', async () => {
    invokeCommandMock
      .mockResolvedValueOnce([
        { path: 'C:/a.csv' },
        { path: ' C:/b.csv ' },
      ])
      .mockResolvedValueOnce({
        id: 'data-9',
        meta: { len: 42, timeframe: '1m', has_close: true },
        cache_hit: false,
        cache_path: '',
      })

    const result = await loadSampleCsv(1)

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'list_sample_csvs')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'load_price_data', { path: 'C:/b.csv' })
    expect(result?.dataId).toBe('data-9')
  })

  it('lists sample CSV entries with normalized labels and paths', async () => {
    invokeCommandMock.mockResolvedValueOnce([
      { path: ' C:/samples/aapl.csv ' },
      { label: 'MSFT sample', path: 'C:/samples/msft.csv' },
      { label: 'bad', path: '   ' },
    ])

    const rows = await listSampleCsvs()

    expect(invokeCommandMock).toHaveBeenCalledWith('list_sample_csvs')
    expect(rows).toEqual([
      { label: 'aapl.csv', path: 'C:/samples/aapl.csv', disabled: false },
      { label: 'MSFT sample', path: 'C:/samples/msft.csv', disabled: false },
    ])
  })

  it('sets and resets market-data folder via direct commands', async () => {
    invokeCommandMock
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({
        default_root_dir: 'C:/default',
        configured_root_dir: 'C:/custom',
        effective_root_dir: 'C:/custom',
        polygon_cache_dir: 'C:/custom/polygon-cache',
      })
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({
        default_root_dir: 'C:/default',
        configured_root_dir: '',
        effective_root_dir: 'C:/default',
        polygon_cache_dir: 'C:/default/polygon-cache',
      })

    const setInfo = await setMarketDataDir(' C:/custom ', true)
    const resetInfo = await resetMarketDataDir()

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'set_market_data_dir', {
      dir: 'C:/custom',
      moveExisting: true,
    })
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'get_market_data_dir_info')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(3, 'clear_market_data_dir')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(4, 'get_market_data_dir_info')
    expect(setInfo.configuredRootDir).toBe('C:/custom')
    expect(resetInfo.configuredRootDir).toBe('')
  })

  it('opens market-data directory and falls back to reveal_path', async () => {
    invokeCommandMock
      .mockResolvedValueOnce({
        default_root_dir: 'C:/default',
        configured_root_dir: '',
        effective_root_dir: 'C:/default',
        polygon_cache_dir: 'C:/default/polygon-cache',
      })
      .mockRejectedValueOnce(new Error('open failed'))
      .mockResolvedValueOnce(undefined)

    const result = await openMarketDataDir()

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, 'get_market_data_dir_info')
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, 'open_path', { path: 'C:/default/polygon-cache' })
    expect(invokeCommandMock).toHaveBeenNthCalledWith(3, 'reveal_path', { path: 'C:/default/polygon-cache' })
    expect(result).toEqual({ path: 'C:/default/polygon-cache', openedWith: 'reveal_path' })
  })

  it('rejects when market-data directory path is empty', async () => {
    await expect(setMarketDataDir('   ', true)).rejects.toThrow('Enter a market data folder path first.')
  })

  it('toggles polygon modal via direct DOM control', async () => {
    class FakeElement {
      classList = {
        _set: new Set<string>(),
        add: (value: string) => {
          this.classList._set.add(value)
        },
        remove: (value: string) => {
          this.classList._set.delete(value)
        },
        contains: (value: string) => this.classList._set.has(value),
      }
      focus(): void {}
    }
    vi.stubGlobal('HTMLElement', FakeElement as unknown as typeof HTMLElement)
    const modal = new FakeElement()
    const symbol = new FakeElement()
    vi.stubGlobal('document', {
      body: { style: { overflow: '' } },
      getElementById: (id: string) => {
        if (id === 'polygonModal') return modal
        if (id === 'polygonSymbol') return symbol
        if (id === 'polygonApiKey') return null
        return null
      },
    })

    await openPolygon()
    expect(modal.classList.contains('open')).toBe(true)

    await closePolygon()
    expect(modal.classList.contains('open')).toBe(false)
    vi.unstubAllGlobals()
  })
})
