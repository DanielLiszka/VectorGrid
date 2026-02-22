import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  invokeLegacyExecutionClearKeysRuntime,
  invokeLegacyExecutionLiveStartRuntime,
  invokeLegacyExecutionLiveStopRuntime,
  invokeLegacyExecutionRefreshRuntime,
  invokeLegacyExecutionRunOnceRuntime,
  invokeLegacyExecutionRunnerStartRuntime,
  invokeLegacyExecutionRunnerStopRuntime,
  invokeLegacyExecutionTestConnectionRuntime,
} from '../legacy/actions'
import { useExecutionStore } from './executionStore'

vi.mock('../legacy/actions', () => ({
  invokeLegacyExecutionTestConnectionRuntime: vi.fn(async () => true),
  invokeLegacyExecutionRunOnceRuntime: vi.fn(async () => true),
  invokeLegacyExecutionRefreshRuntime: vi.fn(async () => true),
  invokeLegacyExecutionLiveStartRuntime: vi.fn(async () => true),
  invokeLegacyExecutionLiveStopRuntime: vi.fn(async () => true),
  invokeLegacyExecutionRunnerStartRuntime: vi.fn(async () => true),
  invokeLegacyExecutionRunnerStopRuntime: vi.fn(async () => true),
  invokeLegacyExecutionClearKeysRuntime: vi.fn(async () => true),
}))

const invokeLegacyExecutionTestConnectionRuntimeMock = vi.mocked(invokeLegacyExecutionTestConnectionRuntime)
const invokeLegacyExecutionRunOnceRuntimeMock = vi.mocked(invokeLegacyExecutionRunOnceRuntime)
const invokeLegacyExecutionRefreshRuntimeMock = vi.mocked(invokeLegacyExecutionRefreshRuntime)
const invokeLegacyExecutionLiveStartRuntimeMock = vi.mocked(invokeLegacyExecutionLiveStartRuntime)
const invokeLegacyExecutionLiveStopRuntimeMock = vi.mocked(invokeLegacyExecutionLiveStopRuntime)
const invokeLegacyExecutionRunnerStartRuntimeMock = vi.mocked(invokeLegacyExecutionRunnerStartRuntime)
const invokeLegacyExecutionRunnerStopRuntimeMock = vi.mocked(invokeLegacyExecutionRunnerStopRuntime)
const invokeLegacyExecutionClearKeysRuntimeMock = vi.mocked(invokeLegacyExecutionClearKeysRuntime)

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
    execSymbol: {
      value: '',
      dispatchEvent: (event: Event) => trackEvent('execSymbol', event),
    },
    execMode: {
      value: 'paper',
      dispatchEvent: (event: Event) => trackEvent('execMode', event),
    },
    execDryRun: {
      checked: false,
      type: 'checkbox',
      dispatchEvent: (event: Event) => trackEvent('execDryRun', event),
    },
  }
  vi.stubGlobal('document', {
    getElementById: (id: string) => fields[id] ?? null,
  })
  return { fields, eventsById }
}

describe('execution store actions', () => {
  let harness: DocumentHarness

  beforeEach(() => {
    setActivePinia(createPinia())
    harness = installDocumentHarness()
    invokeLegacyExecutionTestConnectionRuntimeMock.mockClear()
    invokeLegacyExecutionTestConnectionRuntimeMock.mockResolvedValue(true)
    invokeLegacyExecutionRunOnceRuntimeMock.mockClear()
    invokeLegacyExecutionRunOnceRuntimeMock.mockResolvedValue(true)
    invokeLegacyExecutionRefreshRuntimeMock.mockClear()
    invokeLegacyExecutionRefreshRuntimeMock.mockResolvedValue(true)
    invokeLegacyExecutionLiveStartRuntimeMock.mockClear()
    invokeLegacyExecutionLiveStartRuntimeMock.mockResolvedValue(true)
    invokeLegacyExecutionLiveStopRuntimeMock.mockClear()
    invokeLegacyExecutionLiveStopRuntimeMock.mockResolvedValue(true)
    invokeLegacyExecutionRunnerStartRuntimeMock.mockClear()
    invokeLegacyExecutionRunnerStartRuntimeMock.mockResolvedValue(true)
    invokeLegacyExecutionRunnerStopRuntimeMock.mockClear()
    invokeLegacyExecutionRunnerStopRuntimeMock.mockResolvedValue(true)
    invokeLegacyExecutionClearKeysRuntimeMock.mockClear()
    invokeLegacyExecutionClearKeysRuntimeMock.mockResolvedValue(true)
  })

  it('dispatches execution field updates via legacy DOM events', () => {
    const store = useExecutionStore()
    store.setInputField('execSymbol', 'AAPL')
    store.setSelectField('execMode', 'paper')
    store.setCheckboxField('execDryRun', true)

    const dryRun = harness.fields.execDryRun as LegacyCheckboxField
    expect((harness.fields.execSymbol as LegacyTextField).value).toBe('AAPL')
    expect((harness.fields.execMode as LegacyTextField).value).toBe('paper')
    expect(dryRun.checked).toBe(true)
    expect(harness.eventsById.execSymbol).toEqual(['input'])
    expect(harness.eventsById.execMode).toEqual(['change'])
    expect(harness.eventsById.execDryRun).toEqual(['change'])
  })

  it('dispatches direct execution runtime wrappers for run/control actions', () => {
    const store = useExecutionStore()
    store.testConnection()
    store.runOnce()
    store.refreshOrders()
    store.startLive()
    store.stopLive()
    store.startRunner()
    store.stopRunner()
    store.clearKeys()

    expect(invokeLegacyExecutionTestConnectionRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyExecutionRunOnceRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyExecutionRefreshRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyExecutionLiveStartRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyExecutionLiveStopRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyExecutionRunnerStartRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyExecutionRunnerStopRuntimeMock).toHaveBeenCalled()
    expect(invokeLegacyExecutionClearKeysRuntimeMock).toHaveBeenCalled()
  })
})
