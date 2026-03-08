import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invokeCommand } from '../tauri/invoke'
import {
  listDynamicIndicatorCatalog,
  normalizeDynamicIndicatorCatalog,
  resetDynamicIndicatorCatalogCacheForTests,
} from './dynamicIndicatorCatalogService'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => []),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

describe('dynamicIndicatorCatalogService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetDynamicIndicatorCatalogCacheForTests()
  })

  it('normalizes indicator descriptors with params and outputs', () => {
    const normalized = normalizeDynamicIndicatorCatalog([
      null,
      {
        id: '  SMA ',
        label: 'Simple MA',
        category: 'trend',
        dynamic_strategy_eligible: true,
        input_kind: 'slice',
        requires_candles: false,
        supports_cpu_single: true,
        supports_cpu_batch: true,
        supports_cuda_single: true,
        supports_cuda_sweep: true,
        supports_cuda_vram: false,
        supports_gpu_kernel: true,
        prefers_gpu_by_speedup: true,
        output_count: 1,
        outputs: [
          { id: 'value', label: 'Value', value_type: 'f64' },
          { id: 'value', label: 'duplicate', value_type: 'f64' },
        ],
        params: [
          {
            key: 'period',
            label: 'Period',
            kind: 'int',
            required: true,
            default: { kind: 'int', value: 20 },
            min: 1,
            max: 500,
            step: 1,
            enum_values: [],
            notes: '',
          },
          {
            key: 'period',
            label: 'duplicate',
            kind: 'int',
          },
        ],
        notes: 'fast path',
      },
      {
        id: 'macd',
        outputs: [
          { id: 'line', value_type: 'f64' },
          { id: 'signal', value_type: 'f64' },
          { id: 'hist', value_type: 'f64' },
        ],
        params: [
          {
            key: 'normalize',
            kind: 'bool',
            default: { kind: 'bool', value: true },
            required: false,
          },
          {
            key: 'mode',
            kind: 'enum_string',
            default: { kind: 'enum_string', value: 'classic' },
            enum_values: ['classic', 'zero_lag'],
            required: false,
          },
        ],
      },
    ])

    expect(normalized).toHaveLength(2)
    const first = normalized[0]
    const second = normalized[1]
    expect(first).toBeDefined()
    expect(second).toBeDefined()
    expect(first).toEqual({
      id: 'sma',
      label: 'Simple MA',
      category: 'trend',
      dynamicStrategyEligible: true,
      inputKind: 'slice',
      requiresCandles: false,
      supportsCpuSingle: true,
      supportsCpuBatch: true,
      supportsCudaSingle: true,
      supportsCudaSweep: true,
      supportsCudaVram: false,
      supportsGpuKernel: true,
      prefersGpuBySpeedup: true,
      outputCount: 1,
      outputs: [{ id: 'value', label: 'Value', valueType: 'f64' }],
      params: [
        {
          key: 'period',
          label: 'Period',
          kind: 'int',
          required: true,
          defaultValue: { kind: 'int', value: 20 },
          min: 1,
          max: 500,
          step: 1,
          enumValues: [],
          notes: '',
        },
      ],
      notes: 'fast path',
    })

    expect(second?.id).toBe('macd')
    expect(second?.outputs.map((row) => row.id)).toEqual(['line', 'signal', 'hist'])
    expect(second?.params[0]?.defaultValue).toEqual({ kind: 'bool', value: true })
    expect(second?.params[1]?.defaultValue).toEqual({ kind: 'enum_string', value: 'classic' })
  })

  it('caches list_dynamic_indicators responses unless force refreshed', async () => {
    invokeCommandMock
      .mockResolvedValueOnce([{ id: 'sma', outputs: [] }])
      .mockResolvedValueOnce([{ id: 'ema', outputs: [] }])

    const first = await listDynamicIndicatorCatalog()
    const second = await listDynamicIndicatorCatalog()
    const third = await listDynamicIndicatorCatalog(true)

    expect(invokeCommandMock).toHaveBeenCalledTimes(2)
    expect(first.map((row) => row.id)).toEqual(['sma'])
    expect(second.map((row) => row.id)).toEqual(['sma'])
    expect(third.map((row) => row.id)).toEqual(['ema'])
  })
})
