import { beforeEach, describe, expect, it, vi } from 'vitest'
import { EXEC_KEYS_STORAGE_KEY } from '../utils/core/storageKeys'
import { invokeCommand } from '../tauri/invoke'
import { readLatestWorkflowRunSnapshot } from '../stores/workflowArtifactsStore'
import {
  buildExecutionStrategyFromRunContext,
  resolveExecutionTopRowSnapshot,
} from './analysisService'
import {
  clearKeys,
  loadPersistedExecutionKeys,
  runOnce,
  testConnection,
} from './executionService'

vi.mock('../tauri/invoke', () => ({
  invokeCommand: vi.fn(async () => ({})),
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
    expect(result.orderCount).toBe(0)
    expect(result.outputPreview).toBe('-')
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

  it('executes runOnce via direct commands and hydrates order snapshot when order is submitted', async () => {
    invokeCommandMock.mockImplementation(async (command: string) => {
      if (command === 'execute_strategy_signal_once') {
        return {
          action: 'buy',
          order_submitted: true,
          order_id: 'ord_123',
          warnings: [],
        }
      }
      if (command === 'get_live_execution_snapshot') {
        return {
          symbol: 'AAPL',
          position: { qty: 1, side: 'long', avg_entry_price: 190.1, current_price: 191.2, unrealized_pl: 1.1, unrealized_plpc: 0.0057 },
          orders: [
            {
              id: 'ord_123',
              side: 'buy',
              status: 'filled',
              qty: 1,
              filled_qty: 1,
              filled_avg_price: 190.1,
              submitted_at: '2026-02-26T12:00:00Z',
            },
          ],
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
    expect(invokeCommandMock).toHaveBeenCalledWith('get_live_execution_snapshot', expect.any(Object))
    expect(result.statusLine).toContain('Submitted BUY order')
    expect(result.orderCount).toBe(1)
    expect(result.orderRows?.[0]?.cells?.[1]).toBe('buy')
  })
})
