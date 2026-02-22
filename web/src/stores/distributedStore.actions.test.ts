import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  invokeLegacyDistributedCancelRuntime,
  invokeLegacyDistributedFillFromPolygonRuntime,
  invokeLegacyDistributedGenerateTokenRuntime,
  invokeLegacyDistributedRunAssetsRuntime,
  invokeLegacyDistributedRunTilesRuntime,
  invokeLegacyDistributedStartWorkerRuntime,
  invokeLegacyDistributedStopWorkerRuntime,
  invokeLegacyDistributedUseCurrentCsvRuntime,
  invokeLegacyDistributedWorkerStatusRuntime,
} from '../legacy/actions'
import { useDistributedStore } from './distributedStore'

vi.mock('../legacy/actions', () => ({
  invokeLegacyDistributedGenerateTokenRuntime: vi.fn(async () => true),
  invokeLegacyDistributedStartWorkerRuntime: vi.fn(async () => true),
  invokeLegacyDistributedStopWorkerRuntime: vi.fn(async () => true),
  invokeLegacyDistributedWorkerStatusRuntime: vi.fn(async () => true),
  invokeLegacyDistributedFillFromPolygonRuntime: vi.fn(async () => true),
  invokeLegacyDistributedUseCurrentCsvRuntime: vi.fn(async () => true),
  invokeLegacyDistributedRunAssetsRuntime: vi.fn(async () => true),
  invokeLegacyDistributedRunTilesRuntime: vi.fn(async () => true),
  invokeLegacyDistributedCancelRuntime: vi.fn(async () => true),
}))

const invokeLegacyDistributedGenerateTokenRuntimeMock = vi.mocked(invokeLegacyDistributedGenerateTokenRuntime)
const invokeLegacyDistributedStartWorkerRuntimeMock = vi.mocked(invokeLegacyDistributedStartWorkerRuntime)
const invokeLegacyDistributedStopWorkerRuntimeMock = vi.mocked(invokeLegacyDistributedStopWorkerRuntime)
const invokeLegacyDistributedWorkerStatusRuntimeMock = vi.mocked(invokeLegacyDistributedWorkerStatusRuntime)
const invokeLegacyDistributedFillFromPolygonRuntimeMock = vi.mocked(invokeLegacyDistributedFillFromPolygonRuntime)
const invokeLegacyDistributedUseCurrentCsvRuntimeMock = vi.mocked(invokeLegacyDistributedUseCurrentCsvRuntime)
const invokeLegacyDistributedRunAssetsRuntimeMock = vi.mocked(invokeLegacyDistributedRunAssetsRuntime)
const invokeLegacyDistributedRunTilesRuntimeMock = vi.mocked(invokeLegacyDistributedRunTilesRuntime)
const invokeLegacyDistributedCancelRuntimeMock = vi.mocked(invokeLegacyDistributedCancelRuntime)

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
    distWorkerBindAddr: makeField('distWorkerBindAddr', ''),
    distLeaderMode: makeField('distLeaderMode', 'assets'),
  }
  vi.stubGlobal('document', {
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields, eventsById }
}

describe('distributed store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    harness = installDocumentHarness()
    invokeLegacyDistributedGenerateTokenRuntimeMock.mockClear()
    invokeLegacyDistributedGenerateTokenRuntimeMock.mockResolvedValue(true)
    invokeLegacyDistributedStartWorkerRuntimeMock.mockClear()
    invokeLegacyDistributedStartWorkerRuntimeMock.mockResolvedValue(true)
    invokeLegacyDistributedStopWorkerRuntimeMock.mockClear()
    invokeLegacyDistributedStopWorkerRuntimeMock.mockResolvedValue(true)
    invokeLegacyDistributedWorkerStatusRuntimeMock.mockClear()
    invokeLegacyDistributedWorkerStatusRuntimeMock.mockResolvedValue(true)
    invokeLegacyDistributedFillFromPolygonRuntimeMock.mockClear()
    invokeLegacyDistributedFillFromPolygonRuntimeMock.mockResolvedValue(true)
    invokeLegacyDistributedUseCurrentCsvRuntimeMock.mockClear()
    invokeLegacyDistributedUseCurrentCsvRuntimeMock.mockResolvedValue(true)
    invokeLegacyDistributedRunAssetsRuntimeMock.mockClear()
    invokeLegacyDistributedRunAssetsRuntimeMock.mockResolvedValue(true)
    invokeLegacyDistributedRunTilesRuntimeMock.mockClear()
    invokeLegacyDistributedRunTilesRuntimeMock.mockResolvedValue(true)
    invokeLegacyDistributedCancelRuntimeMock.mockClear()
    invokeLegacyDistributedCancelRuntimeMock.mockResolvedValue(true)
  })

  it('dispatches distributed field updates via legacy DOM events', () => {
    const store = useDistributedStore()
    store.setInputField('distWorkerBindAddr', '127.0.0.1:48991')
    store.setSelectField('distLeaderMode', 'tiles')

    expect(harness.fields.distWorkerBindAddr?.value).toBe('127.0.0.1:48991')
    expect(harness.fields.distLeaderMode?.value).toBe('tiles')
    expect(harness.eventsById.distWorkerBindAddr).toEqual(['input'])
    expect(harness.eventsById.distLeaderMode).toEqual(['change'])
  })

  it('dispatches direct distributed runtime wrappers for control actions', () => {
    const store = useDistributedStore()
    store.generateToken()
    store.startWorker()
    store.stopWorker()
    store.refreshWorkerStatus()
    store.fillFromPolygon()
    store.useCurrentCsv()
    store.runAssets()
    store.runTiles()
    store.cancel()

    expect(invokeLegacyDistributedGenerateTokenRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDistributedStartWorkerRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDistributedStopWorkerRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDistributedWorkerStatusRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDistributedFillFromPolygonRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDistributedUseCurrentCsvRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDistributedRunAssetsRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDistributedRunTilesRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyDistributedCancelRuntimeMock).toHaveBeenCalled()
  })
})
