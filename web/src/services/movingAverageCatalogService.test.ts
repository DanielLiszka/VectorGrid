import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invokeCommand } from '../tauri/invoke'
import {
  listMovingAverageCatalog,
  normalizeMovingAverageCatalog,
  resetMovingAverageCatalogCacheForTests,
} from './movingAverageCatalogService'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => []),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

describe('movingAverageCatalogService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetMovingAverageCatalogCacheForTests()
  })

  it('normalizes moving average catalog entries and parameter schema', () => {
    const catalog = normalizeMovingAverageCatalog([
      null,
      { id: '   ', label: 'invalid', params: [] },
      {
        id: 'alma',
        label: 'ALMA',
        params: [
          {
            key: 'offset',
            label: 'Offset',
            kind: 'float',
            default: 0.85,
            min: 0,
            max: 1,
            step: 0.01,
            notes: 'offset factor',
          },
          {
            key: 'sigma',
            label: 'Sigma',
            kind: 'int',
            default: 6,
            min: 1,
            max: 50,
            step: 1,
            notes: '',
          },
          {
            key: 'sigma',
            label: 'duplicate should be ignored',
            kind: 'float',
            default: 0.5,
          },
        ],
      },
      {
        id: 'sma',
        params: [{ key: 'period', default: 10, step: 0 }],
      },
    ])

    expect(catalog).toHaveLength(2)
    expect(catalog[0]).toEqual({
      id: 'alma',
      label: 'ALMA',
      supportsCpu: false,
      supportsCuda: false,
      supportsCudaMaSweep: false,
      supportsCudaKernel: false,
      prefersCudaKernel: false,
      requiresCandles: false,
      periodBased: false,
      singleOutput: false,
      notes: '',
      params: [
        {
          key: 'offset',
          label: 'Offset',
          kind: 'float',
          defaultValue: 0.85,
          min: 0,
          max: 1,
          step: 0.01,
          notes: 'offset factor',
        },
        {
          key: 'sigma',
          label: 'Sigma',
          kind: 'int',
          defaultValue: 6,
          min: 1,
          max: 50,
          step: 1,
          notes: '',
        },
      ],
    })
    expect(catalog[1]).toEqual({
      id: 'sma',
      label: 'sma',
      supportsCpu: false,
      supportsCuda: false,
      supportsCudaMaSweep: false,
      supportsCudaKernel: false,
      prefersCudaKernel: false,
      requiresCandles: false,
      periodBased: false,
      singleOutput: false,
      notes: '',
      params: [
        {
          key: 'period',
          label: 'period',
          kind: 'float',
          defaultValue: 10,
          min: null,
          max: null,
          step: null,
          notes: '',
        },
      ],
    })
  })

  it('normalizes capability flags and notes', () => {
    const catalog = normalizeMovingAverageCatalog([
      {
        id: 'ema',
        label: 'EMA',
        supports_cpu: true,
        supports_cuda: 1,
        supports_cuda_ma_sweep: 'yes',
        supports_cuda_kernel: 'false',
        prefers_cuda_kernel: '0',
        requires_candles: 'on',
        period_based: true,
        single_output: 'true',
        notes: '  test note  ',
        params: [],
      },
    ])

    expect(catalog).toEqual([
      {
        id: 'ema',
        label: 'EMA',
        supportsCpu: true,
        supportsCuda: true,
        supportsCudaMaSweep: true,
        supportsCudaKernel: false,
        prefersCudaKernel: false,
        requiresCandles: true,
        periodBased: true,
        singleOutput: true,
        notes: 'test note',
        params: [],
      },
    ])
  })

  it('caches list_moving_averages responses unless force refresh is requested', async () => {
    invokeCommandMock
      .mockResolvedValueOnce([{ id: 'sma', label: 'SMA', params: [] }])
      .mockResolvedValueOnce([{ id: 'ema', label: 'EMA', params: [] }])

    const first = await listMovingAverageCatalog()
    const second = await listMovingAverageCatalog()
    const third = await listMovingAverageCatalog(true)

    expect(invokeCommandMock).toHaveBeenCalledTimes(2)
    expect(first.map((entry) => entry.id)).toEqual(['sma'])
    expect(second.map((entry) => entry.id)).toEqual(['sma'])
    expect(third.map((entry) => entry.id)).toEqual(['ema'])
  })
})
