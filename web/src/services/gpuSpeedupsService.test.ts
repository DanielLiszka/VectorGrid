import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invokeCommand } from '../tauri/invoke'
import {
  listGpuSpeedupMap,
  listGpuSpeedups,
  normalizeGpuSpeedups,
  resetGpuSpeedupsCacheForTests,
} from './gpuSpeedupsService'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => []),
}))

const invokeCommandMock = vi.mocked(invokeCommand)

describe('gpuSpeedupsService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetGpuSpeedupsCacheForTests()
  })

  it('normalizes speedup rows and removes invalid/duplicate entries', () => {
    const normalized = normalizeGpuSpeedups([
      null,
      { indicator_id: '', speedup: 1, cpu_ms: 1, cuda_ms: 1 },
      { indicator_id: 'alma', indicator_name: 'ALMA', speedup: 42.9, cpu_ms: 158.25, cuda_ms: 3.68 },
      { indicator_id: 'alma', indicator_name: 'duplicate', speedup: 9, cpu_ms: 9, cuda_ms: 1 },
      { indicator_id: 'ema', indicator_name: 'EMA', speedup: 'bad', cpu_ms: 100, cuda_ms: 10 },
      { indicator_id: 'sma', indicator_name: '', speedup: '8.03', cpu_ms: '114.3', cuda_ms: '14.2' },
    ])

    expect(normalized).toEqual([
      {
        id: 'alma',
        name: 'ALMA',
        speedup: 42.9,
        cpuMs: 158.25,
        cudaMs: 3.68,
      },
      {
        id: 'sma',
        name: 'sma',
        speedup: 8.03,
        cpuMs: 114.3,
        cudaMs: 14.2,
      },
    ])
  })

  it('caches command responses and builds id map', async () => {
    invokeCommandMock
      .mockResolvedValueOnce([{ indicator_id: 'sma', indicator_name: 'SMA', speedup: 8, cpu_ms: 100, cuda_ms: 12 }])
      .mockResolvedValueOnce([{ indicator_id: 'ema', indicator_name: 'EMA', speedup: 10, cpu_ms: 80, cuda_ms: 8 }])

    const first = await listGpuSpeedups()
    const second = await listGpuSpeedups()
    const map = await listGpuSpeedupMap()
    const refreshed = await listGpuSpeedups(true)

    expect(invokeCommandMock).toHaveBeenCalledTimes(2)
    expect(first.map((entry) => entry.id)).toEqual(['sma'])
    expect(second.map((entry) => entry.id)).toEqual(['sma'])
    expect(map.sma?.speedup).toBe(8)
    expect(refreshed.map((entry) => entry.id)).toEqual(['ema'])
  })
})

