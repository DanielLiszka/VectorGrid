import { beforeEach, describe, expect, it, vi } from 'vitest'
import { EXEC_KEYS_STORAGE_KEY } from '../utils/core/storageKeys'
import { invokeCommand } from '../tauri/invoke'
import { listenEvent } from '../tauri/events'
import { readLatestWorkflowRunSnapshot } from '../stores/workflowArtifactsStore'
import {
  buildExecutionStrategyFromRunContext,
  resolveExecutionTopRowSnapshot,
} from './analysisService'
import {
  canonicalSymbol,
  clearKeys,
  loadPersistedExecutionKeys,
  parseRunnerEvent,
  parsePolygonLiveStatusEvent,
  refreshOrders,
  refreshStatuses,
  runOnce,
  startRunner,
  subscribeExecutionRuntimeEvents,
  testConnection,
} from './executionService'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => ({})),
}))

vi.mock('../tauri/events', () => ({
  listenEvent: vi.fn(async (_event: string, _handler: (payload: unknown) => void) => () => {}),
}))

vi.mock('../stores/workflowArtifactsStore', () => ({
  readLatestWorkflowRunSnapshot: vi.fn(() => ({
    req: { strategy_kind: 'DoubleMa', data_id: 'data_1' },
    res: { best_params: { fast_len: 5, slow_len: 20, fast_ma_type: 'sma', slow_ma_type: 'alma' } },
  })),
}))

vi.mock('./analysisService', () => ({
  resolveExecutionTopRowSnapshot: vi.fn(() => ({
    params: { fast_len: 5, slow_len: 20, fast_ma_type: 'sma', slow_ma_type: 'alma' },
    metrics: { sharpe: 1.2 },
    source: 'selected',
  })),
  buildExecutionStrategyFromRunContext: vi.fn(() => ({
    strategyKind: 'DoubleMa',
    strategy: {
      kind: 'double_ma',
      req: { data_id: 'data_1', params: { fast_len: 5, slow_len: 20, fast_ma_type: 'sma', slow_ma_type: 'alma' } },
    },
  })),
}))

vi.mock('./optimizeService', () => ({
  buildOptimizationRequestFromVueStores: vi.fn(async () => ({
    strategy_kind: 'DoubleMa',
    data_id: 'data_1',
    fast_ma_types: ['sma'],
    slow_ma_types: ['alma'],
    fast_range: [5, 5, 1],
    slow_range: [20, 20, 1],
    strategy: {},
  })),
}))

const invokeCommandMock = vi.mocked(invokeCommand)
const listenEventMock = vi.mocked(listenEvent)
const readLatestWorkflowRunSnapshotMock = vi.mocked(readLatestWorkflowRunSnapshot)
const resolveExecutionTopRowSnapshotMock = vi.mocked(resolveExecutionTopRowSnapshot)
const buildExecutionStrategyFromRunContextMock = vi.mocked(buildExecutionStrategyFromRunContext)

function baseOptions() {
  return {
    mode: 'paper',
    symbol: 'AAPL',
    qty: '1',
    paramSource: 'selected',
    riskStopLossPct: '',
    riskTakeProfitPct: '',
    manualFastLen: '',
    manualSlowLen: '',
    dryRun: true,
    rememberKeys: true,
    apiKey: 'key_1',
    apiSecret: 'secret_1',
    liveUseDelayed: true,
  }
}

describe('executionService', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn(() => null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
    })
  })

  it('reads persisted execution keys', () => {
    const getItem = vi.fn(() => JSON.stringify({ api_key: 'abc', api_secret: 'xyz' }))
    vi.stubGlobal('localStorage', {
      getItem,
      setItem: vi.fn(),
      removeItem: vi.fn(),
    })

    const keys = loadPersistedExecutionKeys()
    expect(getItem).toHaveBeenCalledWith(EXEC_KEYS_STORAGE_KEY)
    expect(keys).toEqual({ apiKey: 'abc', apiSecret: 'xyz' })
  })

  it('canonicalizes symbols without stripping meaningful punctuation', () => {
    expect(canonicalSymbol(' brk.b ')).toBe('BRK.B')
    expect(canonicalSymbol(' spy/iv ')).toBe('SPY/IV')
  })

  it('clears persisted execution keys and returns reset summary', async () => {
    const removeItem = vi.fn()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn(() => null),
      setItem: vi.fn(),
      removeItem,
    })

    const result = await clearKeys()
    expect(removeItem).toHaveBeenCalledWith(EXEC_KEYS_STORAGE_KEY)
    expect(result.statusLine).toContain('Cleared execution API keys')
    expect(result.outputPreview).toBe('-')
    expect(result.liveActive).toBeUndefined()
    expect(result.runnerActive).toBeUndefined()
    expect(result.liveStatusHint).toBeUndefined()
    expect(result.runnerStatusHint).toBeUndefined()
  })

  it('tests connection through direct tauri command path', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'test_alpaca_connection') {
        return { account_number: 'ABC123', status: 'ACTIVE' }
      }
      return {}
    })

    const result = await testConnection(baseOptions())
    expect(invokeCommandMock).toHaveBeenCalledWith('test_alpaca_connection', {
      req: {
        mode: 'paper',
        alpaca_api_key: 'key_1',
        alpaca_api_secret: 'secret_1',
      },
    })
    expect(result.statusHint).toContain('connection OK')
    expect(result.outputPreview).toContain('ABC123')
  })

  it('refreshes orders with the lean broker snapshot flags', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_live_execution_snapshot') {
        return {
          symbol: 'AAPL',
          position: null,
          orders: [],
        }
      }
      return {}
    })

    const result = await refreshOrders(baseOptions())

    expect(invokeCommandMock).toHaveBeenCalledWith('get_live_execution_snapshot', {
      req: expect.objectContaining({
        mode: 'paper',
        symbol: 'AAPL',
        include_account: false,
        include_orders: true,
        include_position: true,
      }),
    })
    expect(result.statusLine).toContain('recent orders for AAPL')
  })

  it('parses rich live stream status metadata during status refresh', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'live_execution_runner_status') {
        return { active: false }
      }
      if (command === 'polygon_live_candle_stream_status') {
        return {
          active: true,
          stream: { symbol: ' brk.b ', delayed: true, market: 'stocks' },
        }
      }
      return {}
    })

    const result = await refreshStatuses()

    expect(result.liveActive).toBe(true)
    expect(result.liveSymbol).toBe('BRK.B')
    expect(result.liveDelayed).toBe(true)
    expect(result.liveStatusHint).toContain('BRK.B')
    expect(result.liveStatusHint).toContain('[delayed]')
  })

  it('rejects auto-runner start when live candles are active for another symbol', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'polygon_live_candle_stream_status') {
        return {
          active: true,
          stream: { symbol: 'TSLA', delayed: true, market: 'stocks' },
        }
      }
      return {}
    })

    await expect(startRunner(baseOptions())).rejects.toThrow(
      'Live candles are currently running for TSLA.',
    )
  })

  it('executes runOnce via direct command path and returns optimistic order feedback without auto-refresh', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'execute_strategy_signal_once') {
        return {
          action: 'buy',
          order_submitted: true,
          order_id: 'ord_123',
          delta_qty: 1,
          warnings: [],
        }
      }
      return {}
    })

    const result = await runOnce({
      ...baseOptions(),
      dryRun: false,
    })

    expect(readLatestWorkflowRunSnapshotMock).toHaveBeenCalled()
    expect(resolveExecutionTopRowSnapshotMock).toHaveBeenCalledWith('selected')
    expect(buildExecutionStrategyFromRunContextMock).toHaveBeenCalled()
    expect(invokeCommandMock).toHaveBeenCalledWith('execute_strategy_signal_once', expect.any(Object))
    expect(invokeCommandMock).toHaveBeenCalledTimes(1)
    expect(result.statusLine).toContain('Submitted BUY order')
    expect(result.prependOrderRows?.[0]?.cells?.[1]).toBe('buy')
  })

  it('parses runtime events and subscribes both execution channels', async () => {
    const handlers = new Map<
      string,
      (payload: unknown, meta: { event: string; id?: number }) => void
    >()
    const offPolygon = vi.fn()
    const offRunner = vi.fn()
    listenEventMock.mockImplementation(async (
      event: string,
      handler: (payload: unknown, meta: { event: string; id?: number }) => void,
    ) => {
      handlers.set(event, handler)
      return event === 'polygon_live_status' ? offPolygon : offRunner
    })

    const onPolygonLiveStatus = vi.fn()
    const onRunnerEvent = vi.fn()
    const teardown = await subscribeExecutionRuntimeEvents({
      onPolygonLiveStatus,
      onRunnerEvent,
    })

    expect(listenEventMock).toHaveBeenCalledTimes(2)
    handlers.get('polygon_live_status')?.({
      status: 'started',
      symbol: ' brk.b ',
      delayed: true,
      message: 'live started',
    }, { event: 'polygon_live_status' })
    handlers.get('live_execution_runner_event')?.({
      status: 'order_submitted',
      symbol: ' aapl ',
      message: 'runner submitted buy order',
      execution: {
        order_submitted: true,
        action: 'buy',
        order_id: 'ord_123',
        delta_qty: 1,
      },
    }, { event: 'live_execution_runner_event' })

    expect(onPolygonLiveStatus).toHaveBeenCalledWith(
      expect.objectContaining({
        active: true,
        symbol: 'BRK.B',
        delayed: true,
      }),
    )
    expect(onRunnerEvent).toHaveBeenCalledWith(
      expect.objectContaining({
        status: 'order_submitted',
        symbol: 'AAPL',
        active: true,
      }),
    )

    teardown()
    expect(offPolygon).toHaveBeenCalledTimes(1)
    expect(offRunner).toHaveBeenCalledTimes(1)
    expect(parsePolygonLiveStatusEvent({ status: 'stopped', symbol: ' msft ' }).active).toBe(false)
    expect(parseRunnerEvent({ status: 'cancelled', symbol: ' msft ' }).active).toBe(false)
  })
})
