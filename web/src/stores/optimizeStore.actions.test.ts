import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  invokeLegacyOptimizeApplyPresetRuntime,
  invokeLegacyOptimizeCancelRuntime,
  invokeLegacyOptimizeGpuParityCheckRuntime,
  invokeLegacyOptimizePresetKernelRuntime,
  invokeLegacyOptimizeRefreshGpuStatusRuntime,
  invokeLegacyOptimizeRunRuntime,
  invokeLegacyOptimizeValidateRuntime,
} from '../legacy/actions'
import { useOptimizeStore } from './optimizeStore'

vi.mock('../legacy/actions', () => ({
  invokeLegacyOptimizeApplyPresetRuntime: vi.fn(async () => true),
  invokeLegacyOptimizeRunRuntime: vi.fn(async () => true),
  invokeLegacyOptimizeCancelRuntime: vi.fn(async () => true),
  invokeLegacyOptimizeValidateRuntime: vi.fn(async () => true),
  invokeLegacyOptimizePresetKernelRuntime: vi.fn(async () => true),
  invokeLegacyOptimizeRefreshGpuStatusRuntime: vi.fn(async () => true),
  invokeLegacyOptimizeGpuParityCheckRuntime: vi.fn(async () => true),
}))

const invokeLegacyOptimizeApplyPresetRuntimeMock = vi.mocked(invokeLegacyOptimizeApplyPresetRuntime)
const invokeLegacyOptimizeRunRuntimeMock = vi.mocked(invokeLegacyOptimizeRunRuntime)
const invokeLegacyOptimizeCancelRuntimeMock = vi.mocked(invokeLegacyOptimizeCancelRuntime)
const invokeLegacyOptimizeValidateRuntimeMock = vi.mocked(invokeLegacyOptimizeValidateRuntime)
const invokeLegacyOptimizePresetKernelRuntimeMock = vi.mocked(invokeLegacyOptimizePresetKernelRuntime)
const invokeLegacyOptimizeRefreshGpuStatusRuntimeMock = vi.mocked(invokeLegacyOptimizeRefreshGpuStatusRuntime)
const invokeLegacyOptimizeGpuParityCheckRuntimeMock = vi.mocked(invokeLegacyOptimizeGpuParityCheckRuntime)

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
    fastStart: makeField('fastStart', '3'),
    backend: makeField('backend', 'Auto'),
  }
  vi.stubGlobal('document', {
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields, eventsById }
}

describe('optimize store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    harness = installDocumentHarness()
    invokeLegacyOptimizeApplyPresetRuntimeMock.mockClear()
    invokeLegacyOptimizeApplyPresetRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizeRunRuntimeMock.mockClear()
    invokeLegacyOptimizeRunRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizeCancelRuntimeMock.mockClear()
    invokeLegacyOptimizeCancelRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizeValidateRuntimeMock.mockClear()
    invokeLegacyOptimizeValidateRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizePresetKernelRuntimeMock.mockClear()
    invokeLegacyOptimizePresetKernelRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizeRefreshGpuStatusRuntimeMock.mockClear()
    invokeLegacyOptimizeRefreshGpuStatusRuntimeMock.mockResolvedValue(true)
    invokeLegacyOptimizeGpuParityCheckRuntimeMock.mockClear()
    invokeLegacyOptimizeGpuParityCheckRuntimeMock.mockResolvedValue(true)
  })

  it('dispatches typed optimize field updates via legacy DOM events', () => {
    const store = useOptimizeStore()
    store.setInputField('fastStart', '5')
    store.setSelectField('backend', 'CpuOnly')

    expect(harness.fields.fastStart?.value).toBe('5')
    expect(harness.fields.backend?.value).toBe('CpuOnly')
    expect(harness.eventsById.fastStart).toEqual(['input'])
    expect(harness.eventsById.backend).toEqual(['change'])
  })

  it('dispatches direct optimize run/cancel/validation runtime wrappers', () => {
    const store = useOptimizeStore()
    store.runOptimization()
    store.cancelOptimization()
    store.runValidation()

    expect(invokeLegacyOptimizeRunRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyOptimizeCancelRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyOptimizeValidateRuntimeMock).toHaveBeenCalled()
  })

  it('dispatches direct optimize preset and gpu utility runtime wrappers', () => {
    const store = useOptimizeStore()
    store.applyPreset()
    store.presetKernel()
    store.refreshGpuStatus()
    store.runGpuParityCheck()

    expect(invokeLegacyOptimizeApplyPresetRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyOptimizePresetKernelRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyOptimizeRefreshGpuStatusRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyOptimizeGpuParityCheckRuntimeMock).toHaveBeenCalled()
  })
})
