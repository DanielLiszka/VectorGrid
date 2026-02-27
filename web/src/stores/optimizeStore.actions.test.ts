import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  applyPreset,
  cancelOptimization,
  type GpuStatusSummary,
  presetKernel,
  refreshGpuStatus,
  runGpuParityCheck,
  runOptimization,
  runValidation,
} from '../services/optimizeService'
import { useOptimizeStore } from './optimizeStore'

vi.mock('../services/optimizeService', () => ({
  applyPreset: vi.fn(async () => undefined),
  cancelOptimization: vi.fn(async () => undefined),
  presetKernel: vi.fn(async () => undefined),
  refreshGpuStatus: vi.fn(async () => undefined),
  runGpuParityCheck: vi.fn(async () => undefined),
  runOptimization: vi.fn(async () => undefined),
  runValidation: vi.fn(async () => undefined),
}))

const applyPresetMock = vi.mocked(applyPreset)
const runOptimizationMock = vi.mocked(runOptimization)
const cancelOptimizationMock = vi.mocked(cancelOptimization)
const runValidationMock = vi.mocked(runValidation)
const presetKernelMock = vi.mocked(presetKernel)
const refreshGpuStatusMock = vi.mocked(refreshGpuStatus)
const runGpuParityCheckMock = vi.mocked(runGpuParityCheck)
const defaultGpuStatusSummary: GpuStatusSummary = {
  builtWithCuda: true,
  builtWithCudaBacktestKernel: true,
  cudaAvailable: true,
  lastUsedBackend: 'GpuKernel',
  lastUsedDeviceId: '0',
  text: 'GPU: build=cuda | kernel=on | cuda_available=true | last_used=GpuKernel/0',
}

interface LegacyField {
  value: string
}

interface DocumentHarness {
  fields: Record<string, LegacyField>
}

function installDocumentHarness(): DocumentHarness {
  const makeField = (_id: string, value = ''): LegacyField => ({ value })
  const fields: Record<string, LegacyField> = {
    fastStart: makeField('fastStart', '3'),
    backend: makeField('backend', 'Auto'),
  }
  vi.stubGlobal('document', {
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields }
}

describe('optimize store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    harness = installDocumentHarness()
    applyPresetMock.mockClear()
    applyPresetMock.mockResolvedValue(undefined)
    runOptimizationMock.mockClear()
    runOptimizationMock.mockResolvedValue(undefined)
    cancelOptimizationMock.mockClear()
    cancelOptimizationMock.mockResolvedValue(undefined)
    runValidationMock.mockClear()
    runValidationMock.mockResolvedValue(undefined)
    presetKernelMock.mockClear()
    presetKernelMock.mockResolvedValue(undefined)
    refreshGpuStatusMock.mockClear()
    refreshGpuStatusMock.mockResolvedValue(defaultGpuStatusSummary)
    runGpuParityCheckMock.mockClear()
    runGpuParityCheckMock.mockResolvedValue(undefined)
  })

  it('updates Vue optimize state without legacy DOM mirroring', () => {
    const store = useOptimizeStore()
    store.setInputField('fastStart', '5')
    store.setSelectField('backend', 'CpuOnly')

    expect(store.fastStart).toBe('5')
    expect(store.backend).toBe('CpuOnly')
    expect(store.source).toBe('vue')
    expect(harness.fields.fastStart?.value).toBe('3')
    expect(harness.fields.backend?.value).toBe('Auto')
  })

  it('dispatches optimize run/cancel/validation through optimize service', () => {
    const store = useOptimizeStore()
    store.runOptimization()
    store.cancelOptimization()
    store.runValidation()

    expect(runOptimizationMock).toHaveBeenCalled()
    expect(cancelOptimizationMock).toHaveBeenCalled()
    expect(runValidationMock).toHaveBeenCalled()
  })

  it('dispatches optimize preset and gpu utility actions through optimize service', () => {
    const store = useOptimizeStore()
    store.applyPreset()
    store.presetKernel()
    store.refreshGpuStatus()
    store.runGpuParityCheck()

    expect(applyPresetMock).toHaveBeenCalled()
    expect(presetKernelMock).toHaveBeenCalled()
    expect(refreshGpuStatusMock).toHaveBeenCalled()
    expect(runGpuParityCheckMock).toHaveBeenCalled()
  })

  it('syncs MA param/sweep state from provided payload', () => {
    const store = useOptimizeStore()
    store.syncMaParamState({
      fast: {
        params: { alpha: 3 },
        sweeps: { alpha: [1, 5, 2] },
      },
      slow: {
        params: { beta: 7 },
        sweeps: { beta: [2, 10, 4] },
      },
    })

    expect(store.fastMaParams).toEqual({ alpha: 3 })
    expect(store.slowMaParams).toEqual({ beta: 7 })
    expect(store.fastMaParamSweeps).toEqual({ alpha: [1, 5, 2] })
    expect(store.slowMaParamSweeps).toEqual({ beta: [2, 10, 4] })
    expect(store.source).toBe('vue')
  })
})
