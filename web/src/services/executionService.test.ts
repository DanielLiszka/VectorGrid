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
  cancelOpenOrders,
  clearKeys,
  clearStoredExecutionCredentialsSecurely,
  emergencyHalt,
  flattenPosition,
  getStoredExecutionCredentialStatus,
  listSavedLiveSessions,
  loadPersistedExecutionKeys,
  loadSavedLiveSessionDetail,
  migrateLegacyStoredExecutionKeysToSecureStorage,
  normalizeExecutionOperatorError,
  parseRunnerEvent,
  parsePolygonLiveStatusEvent,
  refreshOrders,
  refreshStatuses,
  runOnce,
  startRunner,
  storeExecutionCredentialsSecurely,
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
    artifactPath: 'C:/runs/vectorgrid_run_2026-04-12T12-00-00Z.json',
    configFingerprint: 'cfg1-fnv1a64-demo',
    routeProvenance: {
      requestedRoute: 'GpuOnly',
      resolvedRoute: 'CpuOnly',
      realizedRoute: 'CpuOnly',
      routeChanged: true,
      fallbackReason: 'gpu unavailable',
    },
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

  it('clears secure and legacy execution credentials and returns reset summary', async () => {
    const removeItem = vi.fn()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn(() => null),
      setItem: vi.fn(),
      removeItem,
    })

    const result = await clearKeys()
    expect(invokeCommandMock).toHaveBeenCalledWith('alpaca_clear_api_credentials')
    expect(removeItem).toHaveBeenCalledWith(EXEC_KEYS_STORAGE_KEY)
    expect(result.statusLine).toContain('Cleared execution API keys')
    expect(result.outputPreview).toBe('-')
    expect(result.liveActive).toBeUndefined()
    expect(result.runnerActive).toBeUndefined()
    expect(result.liveStatusHint).toBeUndefined()
    expect(result.runnerStatusHint).toBeUndefined()
  })

  it('surfaces secure clear failures while still clearing legacy local storage', async () => {
    const removeItem = vi.fn()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn(() => null),
      setItem: vi.fn(),
      removeItem,
    })
    invokeCommandMock.mockRejectedValueOnce(new Error('Keyring clear failed'))

    await expect(clearKeys()).rejects.toThrow('Keyring clear failed')

    expect(invokeCommandMock).toHaveBeenCalledWith('alpaca_clear_api_credentials')
    expect(removeItem).toHaveBeenCalledWith(EXEC_KEYS_STORAGE_KEY)
  })

  it('reads secure credential storage status without secret readback', async () => {
    invokeCommandMock.mockResolvedValue({
      remember_available: true,
      has_api_key: true,
      has_api_secret: true,
      has_credentials: true,
      state: 'stored',
      storage_backend: 'keyring',
    })

    const status = await getStoredExecutionCredentialStatus()

    expect(invokeCommandMock).toHaveBeenCalledWith('alpaca_get_credential_status')
    expect(status).toEqual({
      rememberAvailable: true,
      hasApiKey: true,
      hasApiSecret: true,
      hasCredentials: true,
      state: 'stored',
      storageBackend: 'keyring',
      message: '',
    })
  })

  it('stores execution credentials through the secure backend command path', async () => {
    invokeCommandMock.mockResolvedValue({
      remember_available: true,
      has_api_key: true,
      has_api_secret: true,
      has_credentials: true,
      state: 'stored',
      storage_backend: 'keyring',
    })

    const status = await storeExecutionCredentialsSecurely('key_1', 'secret_1')

    expect(invokeCommandMock).toHaveBeenCalledWith('alpaca_set_api_credentials', {
      req: { api_key: 'key_1', api_secret: 'secret_1' },
    })
    expect(status.state).toBe('stored')
    expect(status.hasCredentials).toBe(true)
  })

  it('clears secure execution credentials through the backend command path', async () => {
    invokeCommandMock.mockResolvedValue({
      remember_available: true,
      has_api_key: false,
      has_api_secret: false,
      has_credentials: false,
      state: 'missing',
      storage_backend: 'keyring',
    })

    const status = await clearStoredExecutionCredentialsSecurely()

    expect(invokeCommandMock).toHaveBeenCalledWith('alpaca_clear_api_credentials')
    expect(status).toEqual({
      rememberAvailable: true,
      hasApiKey: false,
      hasApiSecret: false,
      hasCredentials: false,
      state: 'missing',
      storageBackend: 'keyring',
      message: '',
    })
  })

  it('migrates legacy stored execution keys into secure storage and clears the legacy payload after success', async () => {
    const removeItem = vi.fn()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn(() => JSON.stringify({ api_key: 'legacy_key', api_secret: 'legacy_secret' })),
      setItem: vi.fn(),
      removeItem,
    })
    invokeCommandMock.mockResolvedValue({
      remember_available: true,
      has_api_key: true,
      has_api_secret: true,
      has_credentials: true,
      state: 'stored',
      storage_backend: 'keyring',
    })

    const status = await migrateLegacyStoredExecutionKeysToSecureStorage()

    expect(invokeCommandMock).toHaveBeenCalledWith('alpaca_set_api_credentials', {
      req: { api_key: 'legacy_key', api_secret: 'legacy_secret' },
    })
    expect(removeItem).toHaveBeenCalledWith(EXEC_KEYS_STORAGE_KEY)
    expect(status.state).toBe('stored')
    expect(status.hasCredentials).toBe(true)
  })

  it('keeps the legacy payload when secure migration fails', async () => {
    const removeItem = vi.fn()
    vi.stubGlobal('localStorage', {
      getItem: vi.fn(() => JSON.stringify({ api_key: 'legacy_key', api_secret: 'legacy_secret' })),
      setItem: vi.fn(),
      removeItem,
    })
    invokeCommandMock.mockRejectedValue(new Error('Secure storage unavailable'))

    await expect(migrateLegacyStoredExecutionKeysToSecureStorage()).rejects.toThrow(
      'Secure storage unavailable',
    )

    expect(removeItem).not.toHaveBeenCalled()
  })

  it('tests connection through direct tauri command path', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'test_alpaca_connection') {
        return { account_number: 'ABC123', status: 'ACTIVE', fetched_at_unix_ms: 1_744_297_700_000 }
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
    expect(result.accountMeta).toContain('Account review: paper')
    expect(result.accountReview).toEqual(
      expect.objectContaining({
        state: 'broker_confirmed',
        sourceCommand: 'test_alpaca_connection',
        accountNumber: 'ABC123',
      }),
    )
    expect(result.outputPreviewMeta).toEqual(
      expect.objectContaining({
        sourceCommand: 'test_alpaca_connection',
        scopeLabel: 'paper account verification',
        generatedAtUnixMs: 1_744_297_700_000,
      }),
    )
    expect(result.outputPreview).toContain('ABC123')
  })

  it('omits Alpaca typed override fields when broker actions rely on stored secure credentials', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'test_alpaca_connection') {
        return { account_number: 'ABC123', status: 'ACTIVE' }
      }
      return {}
    })
    const getItem = vi.fn(() => JSON.stringify({ api_key: 'legacy_key', api_secret: 'legacy_secret' }))
    vi.stubGlobal('localStorage', {
      getItem,
      setItem: vi.fn(),
      removeItem: vi.fn(),
    })

    await testConnection({
      ...baseOptions(),
      apiKey: '',
      apiSecret: '',
    })

    expect(invokeCommandMock).toHaveBeenCalledWith('test_alpaca_connection', {
      req: {
        mode: 'paper',
      },
    })
    expect(getItem).not.toHaveBeenCalled()
  })

  it('persists remembered typed execution credentials to secure storage only after a successful broker action', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'test_alpaca_connection') {
        return { account_number: 'ABC123', status: 'ACTIVE' }
      }
      if (command === 'alpaca_set_api_credentials') {
        return {
          remember_available: true,
          has_api_key: true,
          has_api_secret: true,
          has_credentials: true,
          state: 'stored',
          storage_backend: 'keyring',
        }
      }
      return {}
    })

    await testConnection({
      ...baseOptions(),
      rememberKeys: true,
    })

    expect(invokeCommandMock.mock.calls).toEqual(
      expect.arrayContaining([
        [
          'test_alpaca_connection',
          {
            req: {
              mode: 'paper',
              alpaca_api_key: 'key_1',
              alpaca_api_secret: 'secret_1',
            },
          },
        ],
        [
          'alpaca_set_api_credentials',
          {
            req: {
              api_key: 'key_1',
              api_secret: 'secret_1',
            },
          },
        ],
      ]),
    )
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

  it('cancels open orders and refreshes broker review for the current symbol', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'cancel_live_execution_open_orders') {
        return {
          mode: 'paper',
          symbol: 'AAPL',
          open_order_count_before: 2,
          cancelled_count: 2,
          cancelled_order_ids: ['ord_1', 'ord_2'],
        }
      }
      if (command === 'get_live_execution_snapshot') {
        return {
          symbol: 'AAPL',
          account_number: 'ABC123',
          account_status: 'ACTIVE',
          fetched_at_unix_ms: 1_900_000_000_000,
          position: null,
          orders: [],
        }
      }
      return {}
    })

    const result = await cancelOpenOrders({
      ...baseOptions(),
      dryRun: false,
    })

    expect(invokeCommandMock).toHaveBeenCalledWith('cancel_live_execution_open_orders', {
      req: {
        mode: 'paper',
        symbol: 'AAPL',
        alpaca_api_key: 'key_1',
        alpaca_api_secret: 'secret_1',
      },
    })
    expect(invokeCommandMock).toHaveBeenCalledWith('get_live_execution_snapshot', {
      req: {
        mode: 'paper',
        symbol: 'AAPL',
        limit: 200,
        include_account: true,
        include_orders: true,
        include_position: true,
        alpaca_api_key: 'key_1',
        alpaca_api_secret: 'secret_1',
      },
    })
    expect(result.statusLine).toContain('Cancelled 2 open broker orders for AAPL.')
    expect(result.reviewMeta).toContain('symbol-scoped AAPL')
    expect(result.accountReview).toEqual(expect.objectContaining({ accountNumber: 'ABC123' }))
  })

  it('flattens the current position and refreshes broker review', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'flatten_live_execution_position') {
        return {
          mode: 'paper',
          symbol: 'AAPL',
          position_found: true,
          open_order_count_before: 0,
          order_submitted: true,
          order_id: 'ord_flatten',
          action: 'sell',
          qty: 1,
        }
      }
      if (command === 'get_live_execution_snapshot') {
        return {
          symbol: 'AAPL',
          account_number: 'ABC123',
          account_status: 'ACTIVE',
          fetched_at_unix_ms: 1_900_000_000_000,
          position: null,
          orders: [],
        }
      }
      return {}
    })

    const result = await flattenPosition({
      ...baseOptions(),
      dryRun: false,
    })

    expect(invokeCommandMock).toHaveBeenCalledWith('flatten_live_execution_position', {
      req: {
        mode: 'paper',
        symbol: 'AAPL',
        alpaca_api_key: 'key_1',
        alpaca_api_secret: 'secret_1',
      },
    })
    expect(result.statusLine).toContain('Submitted SELL flatten order (id=ord_flatten) for AAPL.')
    expect(result.statusHint).toContain('Broker review refreshed.')
  })

  it('keeps successful flatten submission visible when broker review refresh fails afterward', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'flatten_live_execution_position') {
        return {
          mode: 'paper',
          symbol: 'AAPL',
          position_found: true,
          open_order_count_before: 0,
          order_submitted: true,
          order_id: 'ord_flatten',
          action: 'sell',
          qty: 1,
        }
      }
      if (command === 'get_live_execution_snapshot') {
        throw new Error('snapshot timeout')
      }
      return {}
    })

    const result = await flattenPosition({
      ...baseOptions(),
      dryRun: false,
    })

    expect(result.statusLine).toContain('Submitted SELL flatten order (id=ord_flatten) for AAPL.')
    expect(result.statusLine).toContain('Broker review refresh failed.')
    expect(result.statusHint).toContain('Broker-side flatten handling still succeeded.')
    expect(result.statusHint).toContain('Refresh orders manually now.')
    expect(result.brokerReviewRefreshRequired).toBe(true)
    expect(result.prependOrderRows?.[0]?.cells?.[2]).toBe('submitted*')
  })

  it('emergency halt disarms local live automation and requires manual review', async () => {
    invokeCommandMock.mockResolvedValue({
      runner_stopped: true,
      live_stopped: true,
      live_symbol: 'AAPL',
      requires_manual_review: true,
      status_message: 'Emergency halt applied. Live feed and auto-runner stopped. Refresh orders before re-arming.',
    })

    const result = await emergencyHalt()

    expect(invokeCommandMock).toHaveBeenCalledWith('emergency_halt_live_execution')
    expect(result.statusLine).toContain('Emergency halt applied.')
    expect(result.liveStatusHint).toContain('Live candles stopped for AAPL.')
    expect(result.runnerStatusHint).toBe('Auto-runner stopped.')
    expect(result.recoveryReviewRequired).toBe(true)
    expect(result.liveActive).toBe(false)
    expect(result.runnerActive).toBe(false)
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
          fetched_at_unix_ms: 1_744_297_700_000,
        }
      }
      return {}
    })

    const result = await refreshOrders(baseOptions())

    expect(invokeCommandMock).toHaveBeenCalledWith('get_live_execution_snapshot', {
      req: expect.objectContaining({
        mode: 'paper',
        symbol: 'AAPL',
        include_account: true,
        include_orders: true,
        include_position: true,
      }),
    })
    expect(result.statusLine).toContain('recent orders for AAPL')
  })

  it('refreshes orders without reading legacy local storage when no typed override is provided', async () => {
    const getItem = vi.fn(() => JSON.stringify({ api_key: 'legacy_key', api_secret: 'legacy_secret' }))
    vi.stubGlobal('localStorage', {
      getItem,
      setItem: vi.fn(),
      removeItem: vi.fn(),
    })
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
      apiKey: '',
      apiSecret: '',
    })

    expect(invokeCommandMock).toHaveBeenCalledWith('get_live_execution_snapshot', {
      req: expect.objectContaining({
        mode: 'paper',
        symbol: 'AAPL',
      }),
    })
    const request = invokeCommandMock.mock.calls[0]?.[1] as { req: Record<string, unknown> } | undefined
    expect(request?.req).not.toHaveProperty('alpaca_api_key')
    expect(request?.req).not.toHaveProperty('alpaca_api_secret')
    expect(getItem).not.toHaveBeenCalled()
  })

  it('rejects broker order review without an explicit symbol', async () => {
    await expect(
      refreshOrders({
        ...baseOptions(),
        symbol: '',
      }),
    ).rejects.toThrow('Execution symbol is required')
  })

  it('maps broker refresh into confirmed flat review state and chart metadata', async () => {
    const freshNow = Date.now()
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'get_live_execution_snapshot') {
        return {
          symbol: 'AAPL',
          account_number: 'ABC123',
          account_status: 'ACTIVE',
          currency: 'USD',
          fetched_at_unix_ms: freshNow,
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
    expect(result.chartMeta).toBe('Execution chart loaded for AAPL with 1 broker-confirmed order marker.')
    expect(result.reviewMeta).toContain('Broker review: paper symbol-scoped AAPL')
    expect(result.reviewMeta).toContain('0 open orders')
    expect(result.accountReview).toEqual(
      expect.objectContaining({
        state: 'broker_confirmed',
        accountNumber: 'ABC123',
        status: 'ACTIVE',
      }),
    )
    expect(result.brokerReview).toEqual(
      expect.objectContaining({
        symbol: 'AAPL',
        sourceCommand: 'get_live_execution_snapshot',
        fetchedAtUnixMs: freshNow,
      }),
    )
    expect(result.outputPreviewMeta).toEqual(
      expect.objectContaining({
        sourceCommand: 'get_live_execution_snapshot',
        scopeLabel: 'symbol-scoped AAPL',
      }),
    )
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
          stream: { symbol: ' brk.b ', delayed: true, market: 'stocks', status: 'started' },
        }
      }
      return {}
    })

    const result = await refreshStatuses()

    expect(result.liveActive).toBe(true)
    expect(result.liveSymbol).toBe('BRK.B')
    expect(result.liveDelayed).toBe(true)
    expect(result.liveLifecycleState).toBe('started')
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
        message: 'stream read failed: websocket reset',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'error',
        symbol: 'BRK.B',
        active: false,
        delayed: true,
        message: 'Live feed for BRK.B [delayed] disconnected while reading market data. Check Polygon connectivity, then refresh broker review before re-arming automation.',
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
          last_action: 'buy',
          last_order_id: 'ord_987',
          last_completed_at_unix_ms: 1_700_000_123_000,
          last_error: 'runner execution failed: failed to submit Alpaca order: timeout',
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
    expect(result.runnerStatusHint).toContain('last action=buy')
    expect(result.runnerStatusHint).toContain('order=ord_987')
    expect(result.runnerStatusHint).toContain('completed=2023-11-14 22:15:23')
    expect(result.runnerStatusHint).toContain('Last failure: Execution failed. Broker order submission failed.')
    expect(result.liveActive).toBe(true)
    expect(result.liveSymbol).toBe('BRK.B')
    expect(result.liveDelayed).toBe(false)
  })

  it('falls back to the latest triggered time when the runner has not completed a cycle yet', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'live_execution_runner_status') {
        return {
          active: true,
          inflight: false,
          run_count: 0,
          skip_count: 0,
          error_count: 0,
          last_triggered_at_unix_ms: 1_700_000_180_000,
          config: {
            symbol: 'AAPL',
            mode: 'paper',
            dry_run: true,
          },
        }
      }
      if (command === 'polygon_live_candle_stream_status') {
        return {
          active: true,
          stream: { symbol: 'AAPL', delayed: true, market: 'stocks', status: 'started' },
        }
      }
      return {}
    })

    const result = await refreshStatuses()

    expect(result.runnerStatusHint).toContain('triggered=2023-11-14 22:16:20')
    expect(result.runnerStatusHint).not.toContain('completed=')
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

    expect(result.statusLine).toBe('Execution status refreshed with warnings.')
    expect(result.statusHint).toContain('Auto-runner status refresh unavailable.')
    expect(result.statusHint).toContain('Live-feed status refresh unavailable.')
    expect(result.runnerStatusHint).toBe('Auto-runner status unavailable.')
    expect(result.runnerActive).toBeUndefined()
    expect(result.liveStatusHint).toBe('Live status unavailable.')
    expect(result.liveActive).toBeUndefined()
    expect(result.liveSymbol).toBeUndefined()
    expect(result.liveDelayed).toBeUndefined()
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
        message: 'runner execution failed: failed to submit Alpaca order: network timeout',
      }),
    ).toEqual(
      expect.objectContaining({
        status: 'error',
        symbol: 'AAPL',
        active: null,
        message: 'Auto-runner for AAPL execution failed. Broker order submission failed. Confirm credentials, buying power, and broker availability, then retry.',
      }),
    )
  })

  it('normalizes broker-facing operator errors into action-oriented wording', () => {
    expect(normalizeExecutionOperatorError('failed to fetch Alpaca orders: timeout')).toBe(
      'Broker order review failed. Check Alpaca connectivity, then refresh again.',
    )
    expect(normalizeExecutionOperatorError('failed to submit Alpaca order: timeout')).toBe(
      'Broker order submission failed. Confirm credentials, buying power, and broker availability, then retry.',
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
    expect(invokeCommandMock).toHaveBeenCalledWith('alpaca_set_api_credentials', {
      req: {
        api_key: 'key_1',
        api_secret: 'secret_1',
      },
    })
    expect(result.statusLine).toContain('Submitted BUY order')
    expect(result.prependOrderRows?.[0]?.cells?.[1]).toBe('buy')
    expect(result.brokerReviewRefreshRequired).toBe(true)
  })

  it('keeps submitted runOnce success visible when secure credential persistence fails afterward', async () => {
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
      if (command === 'alpaca_set_api_credentials') {
        throw new Error('keyring write failed for key_1 / secret_1')
      }
      return {}
    })

    const result = await runOnce({
      ...baseOptions(),
      dryRun: false,
    })

    expect(result.statusLine).toContain('Submitted BUY order')
    expect(result.statusLine).toContain('Secure credential storage did not update.')
    expect(result.statusHint).toContain('Signal execution still completed and a broker order may already have been submitted.')
    expect(result.statusHint).toContain('[redacted]')
    expect(result.statusHint).not.toContain('key_1')
    expect(result.statusHint).not.toContain('secret_1')
    expect(result.brokerReviewRefreshRequired).toBe(true)
  })

  it('passes source-run provenance through runOnce request payloads', async () => {
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

    await runOnce(baseOptions())

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'execute_strategy_signal_once')
    expect(call?.[1]).toEqual({
      req: expect.objectContaining({
        param_source: 'selected',
        source_run_artifact_path: 'C:/runs/vectorgrid_run_2026-04-12T12-00-00Z.json',
        source_run_config_fingerprint: 'cfg1-fnv1a64-demo',
        source_run_route_provenance: expect.objectContaining({
          requestedRoute: 'GpuOnly',
          resolvedRoute: 'CpuOnly',
          realizedRoute: 'CpuOnly',
        }),
      }),
    })
  })

  it('omits Alpaca typed override fields when runOnce relies on stored secure credentials', async () => {
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

    await runOnce({
      ...baseOptions(),
      dryRun: false,
      apiKey: '',
      apiSecret: '',
    })

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'execute_strategy_signal_once')
    expect(call).toBeTruthy()
    expect(call?.[1]).toEqual({
      req: expect.not.objectContaining({
        alpaca_api_key: expect.anything(),
        alpaca_api_secret: expect.anything(),
      }),
    })
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

  it('lists saved live session artifacts via the new tauri command surface', async () => {
    invokeCommandMock.mockResolvedValueOnce([
      {
        ts: '2026-04-12T16:10:00Z',
        path: 'C:/runs/vectorgrid_live_session_2026-04-12T16-10-00Z.json',
        sessionId: 'live-session-1',
        mode: 'paper',
        symbol: 'AAPL',
        strategyKind: 'DoubleMa',
        dataId: 'data_1',
        dryRun: false,
        paramSource: 'best',
        sessionState: 'completed',
        createdAtUnixMs: 1_700_000_000_000,
        finalizedAtUnixMs: 1_700_000_000_500,
        eventCount: 3,
        brokerSnapshotCount: 1,
      },
    ])

    await expect(listSavedLiveSessions()).resolves.toEqual([
      expect.objectContaining({
        path: 'C:/runs/vectorgrid_live_session_2026-04-12T16-10-00Z.json',
        symbol: 'AAPL',
        sessionState: 'completed',
        eventCount: 3,
      }),
    ])
    expect(invokeCommandMock).toHaveBeenCalledWith('list_live_session_artifacts')
  })

  it('loads saved live session detail as read-only review data', async () => {
    invokeCommandMock.mockResolvedValueOnce({
      ts: '2026-04-12T16:10:00Z',
      artifactPath: 'C:/runs/vectorgrid_live_session_2026-04-12T16-10-00Z.json',
      sessionId: 'live-session-1',
      sessionState: 'completed',
      mode: 'paper',
      symbol: 'AAPL',
      qty: 1,
      dryRun: false,
      strategyKind: 'DoubleMa',
      dataId: 'data_1',
      paramSource: 'best',
      createdAtUnixMs: 1_700_000_000_000,
      finalizedAtUnixMs: 1_700_000_000_500,
      sourceRunArtifactPath: 'C:/runs/vectorgrid_run_2026-04-12T12-00-00Z.json',
      sourceRunConfigFingerprint: 'cfg1-fnv1a64-demo',
      sourceRunRouteProvenance: {
        requestedRoute: 'GpuOnly',
        resolvedRoute: 'CpuOnly',
        realizedRoute: 'CpuOnly',
        routeChanged: true,
        fallbackReason: 'gpu unavailable',
      },
      eventCount: 2,
      brokerSnapshotCount: 1,
      events: [
        {
          provenance: 'local_control',
          status: 'execute_strategy_signal_once',
          message: 'manual execution requested for AAPL',
          symbol: 'AAPL',
          atUnixMs: 1_700_000_000_000,
        },
      ],
      brokerSnapshots: [
        {
          sourceCommand: 'get_live_execution_snapshot',
          mode: 'paper',
          symbol: 'AAPL',
          atUnixMs: 1_700_000_000_100,
          accountStatus: 'ACTIVE',
          positionState: 'flat',
          openOrderCount: 0,
          orderCount: 0,
        },
      ],
    })

    await expect(
      loadSavedLiveSessionDetail('C:/runs/vectorgrid_live_session_2026-04-12T16-10-00Z.json'),
    ).resolves.toEqual(
      expect.objectContaining({
        artifactPath: 'C:/runs/vectorgrid_live_session_2026-04-12T16-10-00Z.json',
        sourceRunRouteSummary: expect.stringContaining('requested=GpuOnly'),
        events: [expect.objectContaining({ provenance: 'local_control' })],
        brokerSnapshots: [expect.objectContaining({ sourceCommand: 'get_live_execution_snapshot' })],
      }),
    )
    expect(invokeCommandMock).toHaveBeenCalledWith('load_live_session_artifact_detail', {
      path: 'C:/runs/vectorgrid_live_session_2026-04-12T16-10-00Z.json',
    })
  })

  it('omits Alpaca typed override fields when auto-runner start relies on stored secure credentials', async () => {
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
            dry_run: false,
          },
        }
      }
      return {}
    })

    await startRunner({
      ...baseOptions(),
      dryRun: false,
      apiKey: '',
      apiSecret: '',
    })

    const call = invokeCommandMock.mock.calls.find(([command]) => command === 'start_live_execution_runner')
    expect(call).toBeTruthy()
    expect(call?.[1]).toEqual({
      req: expect.not.objectContaining({
        alpaca_api_key: expect.anything(),
        alpaca_api_secret: expect.anything(),
      }),
    })
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
