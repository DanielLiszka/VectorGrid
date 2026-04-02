import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  canonicalSymbol,
  clearKeys,
  type ExecutionActionResult,
  type ExecutionRuntimeEventHandlers,
  refreshOrders,
  refreshStatuses,
  runOnce,
  startLive,
  startRunner,
  stopLive,
  stopRunner,
  subscribeExecutionRuntimeEvents,
  testConnection,
  optimisticOrderRowFromExecution,
} from '../services/executionService'
import { useExecutionStore } from './executionStore'

let runtimeEventHandlers: ExecutionRuntimeEventHandlers | null = null

vi.mock('../services/executionService', () => ({
  canonicalSymbol: vi.fn((raw: unknown) => String(raw ?? '').trim().replace(/\s+/g, '').toUpperCase()),
  clearKeys: vi.fn(async () => ({ statusLine: 'Cleared execution API keys.' })),
  loadPersistedExecutionKeys: vi.fn(() => ({ apiKey: '', apiSecret: '' })),
  optimisticOrderRowFromExecution: vi.fn((executionRaw: unknown) => {
    const execution = executionRaw as { order_submitted?: boolean; action?: string; order_id?: string; delta_qty?: number } | null
    return execution?.order_submitted
      ? { cells: ['2026-01-01 12:00:00', String(execution.action || ''), 'submitted*', String(execution.delta_qty || ''), '-', '-', String(execution.order_id || '')] }
      : null
  }),
  refreshOrders: vi.fn(async () => ({ statusLine: 'Loaded 0 orders.' })),
  refreshStatuses: vi.fn(async () => ({ statusLine: 'Execution status refreshed.' })),
  runOnce: vi.fn(async () => ({ statusLine: 'Signal processed.' })),
  startLive: vi.fn(async () => ({ statusLine: 'Live candles started.', liveActive: true, liveSymbol: 'AAPL', liveDelayed: true })),
  startRunner: vi.fn(async () => ({ statusLine: 'Auto-runner started.', runnerActive: true })),
  stopLive: vi.fn(async () => ({ statusLine: 'Live candles stopped.', liveActive: false })),
  stopRunner: vi.fn(async () => ({ statusLine: 'Auto-runner stopped.', runnerActive: false })),
  subscribeExecutionRuntimeEvents: vi.fn(async (handlers: ExecutionRuntimeEventHandlers) => {
    runtimeEventHandlers = handlers
    return () => {
      runtimeEventHandlers = null
    }
  }),
  testConnection: vi.fn(async () => ({ statusLine: 'Connection OK.' })),
}))

const canonicalSymbolMock = vi.mocked(canonicalSymbol)
const testConnectionMock = vi.mocked(testConnection)
const optimisticOrderRowFromExecutionMock = vi.mocked(optimisticOrderRowFromExecution)
const runOnceMock = vi.mocked(runOnce)
const refreshOrdersMock = vi.mocked(refreshOrders)
const refreshStatusesMock = vi.mocked(refreshStatuses)
const startLiveMock = vi.mocked(startLive)
const stopLiveMock = vi.mocked(stopLive)
const startRunnerMock = vi.mocked(startRunner)
const stopRunnerMock = vi.mocked(stopRunner)
const clearKeysMock = vi.mocked(clearKeys)
const subscribeExecutionRuntimeEventsMock = vi.mocked(subscribeExecutionRuntimeEvents)

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}

describe('execution store actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    runtimeEventHandlers = null
    canonicalSymbolMock.mockImplementation((raw: unknown) => String(raw ?? '').trim().replace(/\s+/g, '').toUpperCase())
    testConnectionMock.mockClear()
    testConnectionMock.mockResolvedValue({ statusLine: 'Connection OK.' })
    optimisticOrderRowFromExecutionMock.mockClear()
    optimisticOrderRowFromExecutionMock.mockImplementation((executionRaw: unknown) => {
      const execution = executionRaw as { order_submitted?: boolean; action?: string; order_id?: string; delta_qty?: number } | null
      return execution?.order_submitted
        ? { cells: ['2026-01-01 12:00:00', String(execution.action || ''), 'submitted*', String(execution.delta_qty || ''), '-', '-', String(execution.order_id || '')] }
        : null
    })
    runOnceMock.mockClear()
    runOnceMock.mockResolvedValue({ statusLine: 'Signal processed.' })
    refreshOrdersMock.mockClear()
    refreshOrdersMock.mockResolvedValue({ statusLine: 'Loaded 0 orders.' })
    refreshStatusesMock.mockClear()
    refreshStatusesMock.mockResolvedValue({ statusLine: 'Execution status refreshed.' })
    startLiveMock.mockClear()
    startLiveMock.mockResolvedValue({ statusLine: 'Live candles started.', liveActive: true, liveSymbol: 'AAPL', liveDelayed: true })
    stopLiveMock.mockClear()
    stopLiveMock.mockResolvedValue({ statusLine: 'Live candles stopped.', liveActive: false })
    startRunnerMock.mockClear()
    startRunnerMock.mockResolvedValue({ statusLine: 'Auto-runner started.', runnerActive: true })
    stopRunnerMock.mockClear()
    stopRunnerMock.mockResolvedValue({ statusLine: 'Auto-runner stopped.', runnerActive: false })
    clearKeysMock.mockClear()
    clearKeysMock.mockResolvedValue({ statusLine: 'Cleared execution API keys.' })
    subscribeExecutionRuntimeEventsMock.mockClear()
    subscribeExecutionRuntimeEventsMock.mockImplementation(async (handlers: ExecutionRuntimeEventHandlers) => {
      runtimeEventHandlers = handlers
      return () => {
        runtimeEventHandlers = null
      }
    })
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
    store.startRunner()
    await flushMicrotasks()
    store.stopRunner()
    await flushMicrotasks()
    store.stopLive()
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

  it('keeps stop and status actions available while another action is pending', async () => {
    const store = useExecutionStore()
    let resolveRunOnce: ((value: ExecutionActionResult) => void) | undefined
    runOnceMock.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveRunOnce = resolve
        }),
    )

    store.setInputField('execSymbol', 'AAPL')
    store.applyPolygonLiveStatusEvent({
      active: true,
      symbol: 'AAPL',
      delayed: true,
      hint: 'Live candles active for AAPL [delayed].',
      status: 'started',
      message: 'live started',
    })
    store.applyLiveExecutionRunnerEvent({
      status: 'started',
      symbol: 'AAPL',
      message: 'runner started',
      hint: 'Auto-runner: started',
      active: true,
      execution: null,
    })

    store.runOnce()
    await flushMicrotasks()

    expect(store.isBusy).toBe(true)
    expect(store.canStopRunner).toBe(true)

    await store.refreshStatuses()
    store.stopRunner()
    await flushMicrotasks()

    expect(refreshStatusesMock).toHaveBeenCalled()
    expect(stopRunnerMock).toHaveBeenCalled()

    if (!resolveRunOnce) {
      throw new Error('runOnce resolver was not assigned')
    }
    resolveRunOnce({ statusLine: 'Signal processed.' })
    await flushMicrotasks()
    expect(store.pending.runOnce).toBe(false)
  })

  it('requires a matching live symbol before the runner can start', () => {
    const store = useExecutionStore()
    store.setInputField('execSymbol', 'AAPL')
    store.applyPolygonLiveStatusEvent({
      active: true,
      symbol: 'TSLA',
      delayed: false,
      hint: 'Live candles active for TSLA.',
      status: 'started',
      message: 'tsla live',
    })
    expect(store.canStartRunner).toBe(false)

    store.applyPolygonLiveStatusEvent({
      active: true,
      symbol: 'AAPL',
      delayed: true,
      hint: 'Live candles active for AAPL [delayed].',
      status: 'started',
      message: 'aapl live',
    })
    expect(store.canStartRunner).toBe(true)
  })

  it('reconciles runtime events into live and runner state with optimistic rows', async () => {
    const store = useExecutionStore()

    await store.ensureRuntimeSubscriptions()
    runtimeEventHandlers?.onPolygonLiveStatus?.({
      active: true,
      symbol: 'AAPL',
      delayed: true,
      hint: 'Live candles active for AAPL [delayed].',
      status: 'started',
      message: 'live started',
    })
    runtimeEventHandlers?.onRunnerEvent?.({
      status: 'order_submitted',
      symbol: 'AAPL',
      message: 'runner submitted buy order',
      hint: 'Auto-runner: runner submitted buy order',
      active: true,
      execution: {
        order_submitted: true,
        action: 'buy',
        order_id: 'ord_123',
        delta_qty: 1,
      },
    })

    expect(store.liveActive).toBe(true)
    expect(store.liveSymbol).toBe('AAPL')
    expect(store.liveDelayed).toBe(true)
    expect(store.runnerActive).toBe(true)
    expect(store.orderRows[0]?.cells?.[1]).toBe('buy')
  })

  it('ignores repeated identical live status events', () => {
    const store = useExecutionStore()
    const nowSpy = vi.spyOn(Date, 'now')
    nowSpy.mockReturnValueOnce(101).mockReturnValueOnce(202)

    const liveEvent = {
      active: true as const,
      symbol: 'AAPL',
      delayed: true,
      hint: 'Live candles active for AAPL [delayed].',
      status: 'started',
      message: 'live started',
    }

    store.applyPolygonLiveStatusEvent(liveEvent)
    const firstUpdatedAt = store.updatedAtUnixMs

    store.applyPolygonLiveStatusEvent(liveEvent)

    expect(firstUpdatedAt).toBe(101)
    expect(store.updatedAtUnixMs).toBe(firstUpdatedAt)
    nowSpy.mockRestore()
  })

  it('ignores repeated identical runner events after optimistic order insertion', () => {
    const store = useExecutionStore()
    const nowSpy = vi.spyOn(Date, 'now')
    nowSpy.mockReturnValueOnce(303).mockReturnValueOnce(404)

    const runnerEvent = {
      status: 'order_submitted',
      symbol: 'AAPL',
      message: 'runner submitted buy order',
      hint: 'Auto-runner: runner submitted buy order',
      active: true,
      execution: {
        order_submitted: true,
        action: 'buy',
        order_id: 'ord_123',
        delta_qty: 1,
      },
    }

    store.applyLiveExecutionRunnerEvent(runnerEvent)
    const firstUpdatedAt = store.updatedAtUnixMs
    const firstOrderRows = store.orderRows

    store.applyLiveExecutionRunnerEvent(runnerEvent)

    expect(firstUpdatedAt).toBe(303)
    expect(store.updatedAtUnixMs).toBe(firstUpdatedAt)
    expect(store.orderRows).toBe(firstOrderRows)
    expect(store.orderRows).toHaveLength(1)
    nowSpy.mockRestore()
  })

  it('preserves meaningful runner transitions after no-op candidates', () => {
    const store = useExecutionStore()
    const nowSpy = vi.spyOn(Date, 'now')
    nowSpy.mockReturnValueOnce(505).mockReturnValueOnce(606)

    store.applyLiveExecutionRunnerEvent({
      status: 'started',
      symbol: 'AAPL',
      message: 'runner started',
      hint: 'Auto-runner: started',
      active: true,
      execution: null,
    })

    store.applyLiveExecutionRunnerEvent({
      status: 'stopped',
      symbol: 'AAPL',
      message: 'runner stopped',
      hint: 'Auto-runner: stopped',
      active: false,
      execution: null,
    })

    expect(store.updatedAtUnixMs).toBe(606)
    expect(store.runnerActive).toBe(false)
    expect(store.runnerStatusHint).toBe('Auto-runner: stopped')
    expect(store.statusLine).toBe('runner stopped')
    nowSpy.mockRestore()
  })

  it('keeps runtime state intact when clearing credentials', async () => {
    const store = useExecutionStore()
    store.applyPolygonLiveStatusEvent({
      active: true,
      symbol: 'AAPL',
      delayed: true,
      hint: 'Live candles active for AAPL [delayed].',
      status: 'started',
      message: 'live started',
    })
    store.applyLiveExecutionRunnerEvent({
      status: 'started',
      symbol: 'AAPL',
      message: 'runner started',
      hint: 'Auto-runner: started',
      active: true,
      execution: null,
    })
    store.setCheckboxField('execRememberKeys', true)
    store.setInputField('execApiKey', 'key_1')
    store.setInputField('execApiSecret', 'secret_1')

    store.clearKeys()
    await flushMicrotasks()

    expect(store.apiKey).toBe('')
    expect(store.apiSecret).toBe('')
    expect(store.rememberKeys).toBe(true)
    expect(store.liveActive).toBe(true)
    expect(store.runnerActive).toBe(true)
  })

  it('bounds optimistic rows and dedupes by order id', () => {
    const store = useExecutionStore()
    const optimisticRows = Array.from({ length: 60 }, (_, index) => ({
      cells: [
        '2026-01-01 12:00:00',
        index % 2 === 0 ? 'buy' : 'sell',
        'submitted*',
        '1',
        '-',
        '-',
        index === 59 ? 'ord_010' : `ord_${String(index).padStart(3, '0')}`,
      ],
    }))

    store.applyActionResult({
      statusLine: 'Signal processed.',
      prependOrderRows: optimisticRows,
    })

    expect(store.orderRows).toHaveLength(50)
    expect(store.orderRows.filter((row) => row.cells[6] === 'ord_010')).toHaveLength(1)
    expect(store.orderCount).toBe(50)
  })
})
