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

  it('falls back to empty persisted execution keys when storage payload is malformed', () => {
    const getItem = vi.fn(() => '{not-json')
    vi.stubGlobal('localStorage', {
      getItem,
      setItem: vi.fn(),
      removeItem: vi.fn(),
    })

    const keys = loadPersistedExecutionKeys()

    expect(getItem).toHaveBeenCalledWith(EXEC_KEYS_STORAGE_KEY)
    expect(keys).toEqual({ apiKey: '', apiSecret: '' })
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

  it('uses the live Alpaca mode when testing a live broker connection', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'test_alpaca_connection') {
        return { account_id: 'acct_live', status: 'ACTIVE' }
      }
      return {}
    })

    await testConnection({
      ...baseOptions(),
      mode: 'live',
    })

    expect(invokeCommandMock).toHaveBeenCalledWith('test_alpaca_connection', {
      req: {
        mode: 'live',
        alpaca_api_key: 'key_1',
        alpaca_api_secret: 'secret_1',
      },
    })
  })

  it('uses the live Alpaca mode when refreshing a live broker snapshot', async () => {
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

    await refreshOrders({
      ...baseOptions(),
      mode: 'live',
    })

    expect(invokeCommandMock).toHaveBeenCalledWith('get_live_execution_snapshot', {
      req: expect.objectContaining({
        mode: 'live',
        symbol: 'AAPL',
      }),
    })
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

  it('maps broker refresh into confirmed flat review state and chart metadata', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_live_execution_snapshot') {
        return {
          symbol: 'AAPL',
          position: null,
          orders: [
            {
              id: 'ord_123',
              side: 'buy',
              status: 'filled',
              qty: '1',
              filled_qty: '1',
              filled_avg_price: '182.4',
              submitted_at: '2026-04-10T14:55:00Z',
            },
          ],
        }
      }
      return {}
    })

    const result = await refreshOrders(baseOptions())

    expect(result.positionMeta).toBe('Position: flat')
    expect(result.orderCount).toBe(1)
    expect(result.orderRows?.[0]?.cells?.[1]).toBe('buy')
    expect(result.chartMeta).toBe('Execution chart sync pending. symbol=AAPL orders=1')
    expect(result.outputPreview).toContain('"ord_123"')
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

  it('keeps Polygon live-status parsing aligned with the current backend event vocabulary', () => {
    expect(
      parsePolygonLiveStatusEvent({
        status: 'connecting',
        symbol: ' brk.b ',
        delayed: true,
        message: 'connecting to delayed websocket',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'connecting',
        symbol: 'BRK.B',
        active: true,
        delayed: true,
      }),
    )

    expect(
      parsePolygonLiveStatusEvent({
        status: 'subscribed',
        symbol: ' brk.b ',
        delayed: false,
        message: 'subscribed to minute aggregates',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'subscribed',
        symbol: 'BRK.B',
        active: true,
        delayed: false,
      }),
    )

    expect(
      parsePolygonLiveStatusEvent({
        status: 'closed',
        symbol: ' brk.b ',
        delayed: true,
        message: 'stream ended',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'closed',
        symbol: 'BRK.B',
        active: false,
        delayed: true,
      }),
    )

    expect(
      parsePolygonLiveStatusEvent({
        status: 'error',
        symbol: ' brk.b ',
        delayed: true,
        message: 'stream read failed',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'error',
        symbol: 'BRK.B',
        active: false,
        delayed: true,
      }),
    )

    expect(parsePolygonLiveStatusEvent({ status: 'stopped', symbol: ' msft ' }).active).toBe(false)
  })

  it('includes runner counters and mode details in the combined status refresh summary', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'live_execution_runner_status') {
        return {
          active: true,
          inflight: true,
          run_count: 4,
          skip_count: 1,
          error_count: 2,
          config: {
            symbol: ' brk.b ',
            mode: 'live',
            dry_run: false,
          },
        }
      }
      if (command === 'polygon_live_candle_stream_status') {
        return {
          active: true,
          stream: { symbol: ' brk.b ', delayed: false, market: 'stocks' },
        }
      }
      return {}
    })

    const result = await refreshStatuses()

    expect(result.runnerActive).toBe(true)
    expect(result.runnerStatusHint).toContain('BRK.B, live, broker')
    expect(result.runnerStatusHint).toContain('runs=4')
    expect(result.runnerStatusHint).toContain('skips=1')
    expect(result.runnerStatusHint).toContain('errs=2')
    expect(result.runnerStatusHint).toContain('[in-flight]')
    expect(result.liveActive).toBe(true)
    expect(result.liveSymbol).toBe('BRK.B')
    expect(result.liveDelayed).toBe(false)
  })

  it('degrades status refresh hints gracefully when live or runner status commands fail', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'live_execution_runner_status') {
        throw new Error('runner status unavailable')
      }
      if (command === 'polygon_live_candle_stream_status') {
        throw new Error('live status unavailable')
      }
      return {}
    })

    const result = await refreshStatuses()

    expect(result.statusLine).toBe('Execution status refreshed.')
    expect(result.runnerStatusHint).toBe('Auto-runner status unavailable.')
    expect(result.runnerActive).toBe(false)
    expect(result.liveStatusHint).toBe('Live status unavailable.')
    expect(result.liveActive).toBe(false)
    expect(result.liveSymbol).toBe('')
    expect(result.liveDelayed).toBe(false)
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
      'Live candles are active for TSLA, not AAPL.',
    )
  })

  it('starts the auto-runner through the direct tauri command path and reflects the returned status summary', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'polygon_live_candle_stream_status') {
        return {
          active: true,
          stream: { symbol: 'AAPL', delayed: true, market: 'stocks' },
        }
      }
      if (command === 'start_live_execution_runner') {
        return {
          active: true,
          inflight: false,
          run_count: 0,
          skip_count: 0,
          error_count: 0,
          config: {
            symbol: 'AAPL',
            mode: 'paper',
            dry_run: true,
          },
        }
      }
      return {}
    })

    const result = await startRunner(baseOptions())

    expect(invokeCommandMock).toHaveBeenCalledWith('start_live_execution_runner', {
      req: expect.objectContaining({
        symbol: 'AAPL',
        qty: 1,
        mode: 'paper',
        dry_run: true,
      }),
    })
    expect(result.statusLine).toBe('Auto-runner started for AAPL.')
    expect(result.runnerStatusHint).toContain('Auto-runner: on (AAPL, paper, dry-run)')
    expect(result.runnerActive).toBe(true)
    expect(result.liveSymbol).toBe('AAPL')
    expect(result.liveDelayed).toBe(true)
  })

  it('treats queued runner events as active and preserves error events as state-neutral', () => {
    expect(
      parseRunnerEvent({
        status: 'queued_latest',
        symbol: ' aapl ',
        message: 'runner queued latest live bar while prior run was still in flight',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'queued_latest',
        symbol: 'AAPL',
        active: true,
      }),
    )

    expect(
      parseRunnerEvent({
        status: 'error',
        symbol: ' aapl ',
        message: 'runner execution failed: network timeout',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'error',
        symbol: 'AAPL',
        active: null,
      }),
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

  it('executes runOnce with manual parameter source without reading a top-row snapshot', async () => {
    readLatestWorkflowRunSnapshotMock.mockReturnValue({
      req: null,
      res: {
        best_params: { fast_len: 5, slow_len: 20, fast_ma_type: 'sma', slow_ma_type: 'alma' },
      },
    } as never)
    buildExecutionStrategyFromRunContextMock.mockImplementation((params, runReq) => ({
      strategyKind: 'DoubleMa',
      strategy: {
        kind: 'double_ma',
        req: {
          data_id: String((runReq as Record<string, unknown> | null)?.data_id ?? 'data_1'),
          params,
        },
      },
    }))
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'execute_strategy_signal_once') {
        return {
          action: 'hold',
          order_submitted: false,
          warnings: [],
        }
      }
      return {}
    })

    const result = await runOnce({
      ...baseOptions(),
      paramSource: 'manual',
      manualFastLen: '9',
      manualSlowLen: '34',
    })

    expect(resolveExecutionTopRowSnapshotMock).not.toHaveBeenCalled()
    expect(buildExecutionStrategyFromRunContextMock).toHaveBeenCalledWith(
      expect.objectContaining({
        fast_len: 9,
        slow_len: 34,
        fast_ma_type: 'sma',
        slow_ma_type: 'alma',
      }),
      expect.objectContaining({
        data_id: 'data_1',
      }),
    )
    expect(invokeCommandMock).toHaveBeenCalledWith('execute_strategy_signal_once', {
      req: expect.objectContaining({
        symbol: 'AAPL',
        dry_run: true,
        strategy: expect.objectContaining({
          req: expect.objectContaining({
            params: expect.objectContaining({
              fast_len: 9,
              slow_len: 34,
              fast_ma_type: 'sma',
              slow_ma_type: 'alma',
            }),
          }),
        }),
      }),
    })
    expect(result.statusLine).toContain('Signal processed: action=hold.')
  })

  it('starts the auto-runner from the best parameter source when requested', async () => {
    readLatestWorkflowRunSnapshotMock.mockReturnValue({
      req: { strategy_kind: 'DoubleMa', data_id: 'data_1' },
      res: {
        best_params: { fast_len: 8, slow_len: 21, fast_ma_type: 'ema', slow_ma_type: 'sma' },
      },
    } as never)
    resolveExecutionTopRowSnapshotMock.mockReturnValue({
      params: {
        fast_len: 8,
        slow_len: 21,
        fast_ma_type: 'ema',
        slow_ma_type: 'sma',
      },
      metrics: { sharpe: 1.8 },
      source: 'best',
    })
    buildExecutionStrategyFromRunContextMock.mockImplementation((params, runReq) => ({
      strategyKind: 'DoubleMa',
      strategy: {
        kind: 'double_ma',
        req: {
          data_id: String((runReq as Record<string, unknown> | null)?.data_id ?? 'data_1'),
          params,
        },
      },
    }))
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'polygon_live_candle_stream_status') {
        return {
          active: true,
          stream: { symbol: 'AAPL', delayed: true, market: 'stocks' },
        }
      }
      if (command === 'start_live_execution_runner') {
        return {
          active: true,
          run_count: 0,
          skip_count: 0,
          error_count: 0,
          inflight: false,
          config: {
            symbol: 'AAPL',
            mode: 'paper',
            dry_run: true,
          },
        }
      }
      return {}
    })

    const result = await startRunner({
      ...baseOptions(),
      paramSource: 'best',
    })

    expect(resolveExecutionTopRowSnapshotMock).toHaveBeenCalledWith('best')
    expect(buildExecutionStrategyFromRunContextMock).toHaveBeenCalledWith(
      expect.objectContaining({
        fast_len: 8,
        slow_len: 21,
        fast_ma_type: 'ema',
        slow_ma_type: 'sma',
      }),
      expect.objectContaining({
        data_id: 'data_1',
      }),
    )
    expect(invokeCommandMock).toHaveBeenCalledWith('start_live_execution_runner', {
      req: expect.objectContaining({
        symbol: 'AAPL',
        dry_run: true,
        strategy: expect.objectContaining({
          req: expect.objectContaining({
            params: expect.objectContaining({
              fast_len: 8,
              slow_len: 21,
              fast_ma_type: 'ema',
              slow_ma_type: 'sma',
            }),
          }),
        }),
      }),
    })
    expect(result.statusLine).toContain('Auto-runner started for AAPL.')
    expect(result.runnerActive).toBe(true)
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

  it('keeps runner lifecycle parsing aligned with the current backend event vocabulary', () => {
    expect(
      parseRunnerEvent({
        status: 'started',
        symbol: ' aapl ',
        message: 'auto-runner started for AAPL (DoubleMa, dry-run, qty=1)',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'started',
        symbol: 'AAPL',
        active: true,
      }),
    )

    expect(
      parseRunnerEvent({
        status: 'triggered',
        symbol: ' aapl ',
        message: 'runner triggered on new live bar',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'triggered',
        symbol: 'AAPL',
        active: true,
      }),
    )

    expect(
      parseRunnerEvent({
        status: 'queued_latest',
        symbol: ' aapl ',
        message: 'runner queued latest live bar while prior run was still in flight',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'queued_latest',
        symbol: 'AAPL',
        active: true,
      }),
    )

    expect(
      parseRunnerEvent({
        status: 'signal_processed',
        symbol: ' aapl ',
        message: 'runner processed signal action=hold',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'signal_processed',
        symbol: 'AAPL',
        active: true,
      }),
    )

    expect(
      parseRunnerEvent({
        status: 'error',
        symbol: ' aapl ',
        message: 'runner execution failed: boom',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'error',
        symbol: 'AAPL',
        active: null,
      }),
    )

    expect(
      parseRunnerEvent({
        status: 'stopped',
        symbol: ' aapl ',
        message: 'auto-runner stopped',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'stopped',
        symbol: 'AAPL',
        active: false,
      }),
    )
  })
})
