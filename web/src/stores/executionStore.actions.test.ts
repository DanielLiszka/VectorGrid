import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  clearKeys,
  refreshOrders,
  refreshStatuses,
  runOnce,
  startLive,
  startRunner,
  stopLive,
  stopRunner,
  testConnection,
} from '../services/executionService'
import { useExecutionStore } from './executionStore'

vi.mock('../services/executionService', () => ({
  clearKeys: vi.fn(async () => ({ statusLine: 'Cleared execution API keys.' })),
  loadPersistedExecutionKeys: vi.fn(() => ({ apiKey: '', apiSecret: '' })),
  refreshOrders: vi.fn(async () => ({ statusLine: 'Loaded 0 orders.' })),
  refreshStatuses: vi.fn(async () => ({ statusLine: 'Execution status refreshed.' })),
  runOnce: vi.fn(async () => ({ statusLine: 'Signal processed.' })),
  startLive: vi.fn(async () => ({ statusLine: 'Live candles started.', liveActive: true })),
  startRunner: vi.fn(async () => ({ statusLine: 'Auto-runner started.', runnerActive: true })),
  stopLive: vi.fn(async () => ({ statusLine: 'Live candles stopped.', liveActive: false })),
  stopRunner: vi.fn(async () => ({ statusLine: 'Auto-runner stopped.', runnerActive: false })),
  testConnection: vi.fn(async () => ({ statusLine: 'Connection OK.' })),
}))

const testConnectionMock = vi.mocked(testConnection)
const runOnceMock = vi.mocked(runOnce)
const refreshOrdersMock = vi.mocked(refreshOrders)
const refreshStatusesMock = vi.mocked(refreshStatuses)
const startLiveMock = vi.mocked(startLive)
const stopLiveMock = vi.mocked(stopLive)
const startRunnerMock = vi.mocked(startRunner)
const stopRunnerMock = vi.mocked(stopRunner)
const clearKeysMock = vi.mocked(clearKeys)

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}

describe('execution store actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    testConnectionMock.mockClear()
    testConnectionMock.mockResolvedValue({ statusLine: 'Connection OK.' })
    runOnceMock.mockClear()
    runOnceMock.mockResolvedValue({ statusLine: 'Signal processed.' })
    refreshOrdersMock.mockClear()
    refreshOrdersMock.mockResolvedValue({ statusLine: 'Loaded 0 orders.' })
    refreshStatusesMock.mockClear()
    refreshStatusesMock.mockResolvedValue({ statusLine: 'Execution status refreshed.' })
    startLiveMock.mockClear()
    startLiveMock.mockResolvedValue({ statusLine: 'Live candles started.', liveActive: true })
    stopLiveMock.mockClear()
    stopLiveMock.mockResolvedValue({ statusLine: 'Live candles stopped.', liveActive: false })
    startRunnerMock.mockClear()
    startRunnerMock.mockResolvedValue({ statusLine: 'Auto-runner started.', runnerActive: true })
    stopRunnerMock.mockClear()
    stopRunnerMock.mockResolvedValue({ statusLine: 'Auto-runner stopped.', runnerActive: false })
    clearKeysMock.mockClear()
    clearKeysMock.mockResolvedValue({ statusLine: 'Cleared execution API keys.' })
  })

  it('updates Vue execution state without legacy DOM mirroring', () => {
    const store = useExecutionStore()
    store.setInputField('execSymbol', 'AAPL')
    store.setInputField('execApiKey', 'key_1')
    store.setInputField('execApiSecret', 'secret_1')
    store.setSelectField('execMode', 'paper')
    store.setCheckboxField('execDryRun', true)

    expect(store.symbol).toBe('AAPL')
    expect(store.apiKey).toBe('key_1')
    expect(store.apiSecret).toBe('secret_1')
    expect(store.mode).toBe('paper')
    expect(store.dryRun).toBe(true)
    expect(store.source).toBe('vue')
  })

  it('dispatches execution actions through execution service', async () => {
    const store = useExecutionStore()

    store.setInputField('execApiKey', 'key_1')
    store.setInputField('execApiSecret', 'secret_1')
    store.setInputField('execSymbol', 'AAPL')

    store.testConnection()
    await flushMicrotasks()
    store.runOnce()
    await flushMicrotasks()
    store.refreshOrders()
    await flushMicrotasks()
    store.startLive()
    await flushMicrotasks()
    store.stopLive()
    await flushMicrotasks()
    store.startRunner()
    await flushMicrotasks()
    store.stopRunner()
    await flushMicrotasks()
    store.refreshStatuses()
    await flushMicrotasks()
    store.clearKeys()
    await flushMicrotasks()

    expect(testConnectionMock).toHaveBeenCalledWith(expect.objectContaining({
      mode: 'paper',
      symbol: 'AAPL',
      apiKey: 'key_1',
      apiSecret: 'secret_1',
    }))
    expect(runOnceMock).toHaveBeenCalledWith(expect.objectContaining({
      mode: 'paper',
      paramSource: 'selected',
      symbol: 'AAPL',
    }))
    expect(refreshOrdersMock).toHaveBeenCalledWith(expect.objectContaining({
      mode: 'paper',
      symbol: 'AAPL',
    }))
    expect(startLiveMock).toHaveBeenCalledWith(expect.objectContaining({
      symbol: 'AAPL',
      liveUseDelayed: true,
    }))
    expect(stopLiveMock).toHaveBeenCalled()
    expect(startRunnerMock).toHaveBeenCalledWith(expect.objectContaining({
      symbol: 'AAPL',
      dryRun: true,
    }))
    expect(stopRunnerMock).toHaveBeenCalled()
    expect(refreshStatusesMock).toHaveBeenCalled()
    expect(clearKeysMock).toHaveBeenCalled()
  })
})
