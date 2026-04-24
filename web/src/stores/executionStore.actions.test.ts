import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  cancelOpenOrders,
  canonicalSymbol,
  clearKeys,
  emergencyHalt,
  flattenPosition,
  getStoredExecutionCredentialStatus,
  listSavedLiveSessions,
  loadExecutionRiskPolicy,
  loadSavedLiveSessionDetail,
  type ExecutionActionResult,
  type ExecutionRuntimeEventHandlers,
  migrateLegacyStoredExecutionKeysToSecureStorage,
  normalizeExecutionOperatorError,
  refreshOrders,
  refreshStatuses,
  resetLocalPaperBroker,
  runOnce,
  saveExecutionRiskPolicy,
  startLive,
  startRunner,
  stopLive,
  stopRunner,
  subscribeExecutionRuntimeEvents,
  testConnection,
  optimisticOrderRowFromExecution,
  previewExecutionRiskPolicy,
} from '../services/executionService'
import { useExecutionStore } from './executionStore'

let runtimeEventHandlers: ExecutionRuntimeEventHandlers | null = null

vi.mock('../services/executionService', () => ({
  cancelOpenOrders: vi.fn(async () => ({ statusLine: 'Cancelled 0 open broker orders.' })),
  canonicalSymbol: vi.fn((raw: unknown) => String(raw ?? '').trim().replace(/\s+/g, '').toUpperCase()),
  clearKeys: vi.fn(async () => ({ statusLine: 'Cleared execution API keys.' })),
  emergencyHalt: vi.fn(async () => ({ statusLine: 'Emergency halt applied.', recoveryReviewRequired: true, liveActive: false, runnerActive: false })),
  flattenPosition: vi.fn(async () => ({ statusLine: 'No open broker position to flatten.' })),
  getStoredExecutionCredentialStatus: vi.fn(async () => ({
    rememberAvailable: true,
    hasApiKey: false,
    hasApiSecret: false,
    hasCredentials: false,
    state: 'missing',
    storageBackend: 'keyring',
    message: '',
  })),
  listSavedLiveSessions: vi.fn(async () => []),
  loadExecutionRiskPolicy: vi.fn(async () => ({
    max_order_notional: null,
    max_position_notional: null,
    allow_short: null,
    require_bracket: null,
  })),
  loadSavedLiveSessionDetail: vi.fn(async () => null),
  migrateLegacyStoredExecutionKeysToSecureStorage: vi.fn(async () => ({
    rememberAvailable: true,
    hasApiKey: false,
    hasApiSecret: false,
    hasCredentials: false,
    state: 'missing',
    storageBackend: 'keyring',
    message: '',
  })),
  normalizeExecutionOperatorError: vi.fn((raw: unknown) => String(raw ?? '').trim()),
  optimisticOrderRowFromExecution: vi.fn((executionRaw: unknown) => {
    const execution = executionRaw as { order_submitted?: boolean; action?: string; order_id?: string; delta_qty?: number } | null
    return execution?.order_submitted
      ? { cells: ['2026-01-01 12:00:00', String(execution.action || ''), 'submitted*', String(execution.delta_qty || ''), '-', '-', String(execution.order_id || '')] }
      : null
  }),
  refreshOrders: vi.fn(async () => ({ statusLine: 'Loaded 0 orders.' })),
  refreshStatuses: vi.fn(async () => ({ statusLine: 'Execution status refreshed.' })),
  resetLocalPaperBroker: vi.fn(async () => ({ statusLine: 'Local paper broker reset.' })),
  runOnce: vi.fn(async () => ({ statusLine: 'Signal processed.' })),
  saveExecutionRiskPolicy: vi.fn(async (policy) => policy),
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
  previewExecutionRiskPolicy: vi.fn(async () => ({ statusLine: 'Policy preview allowed.', statusHint: 'Policy preview allowed.' })),
}))

const cancelOpenOrdersMock = vi.mocked(cancelOpenOrders)
const canonicalSymbolMock = vi.mocked(canonicalSymbol)
const testConnectionMock = vi.mocked(testConnection)
const emergencyHaltMock = vi.mocked(emergencyHalt)
const flattenPositionMock = vi.mocked(flattenPosition)
const getStoredExecutionCredentialStatusMock = vi.mocked(getStoredExecutionCredentialStatus)
const listSavedLiveSessionsMock = vi.mocked(listSavedLiveSessions)
const loadExecutionRiskPolicyMock = vi.mocked(loadExecutionRiskPolicy)
const loadSavedLiveSessionDetailMock = vi.mocked(loadSavedLiveSessionDetail)
const migrateLegacyStoredExecutionKeysToSecureStorageMock = vi.mocked(migrateLegacyStoredExecutionKeysToSecureStorage)
const normalizeExecutionOperatorErrorMock = vi.mocked(normalizeExecutionOperatorError)
const optimisticOrderRowFromExecutionMock = vi.mocked(optimisticOrderRowFromExecution)
const runOnceMock = vi.mocked(runOnce)
const saveExecutionRiskPolicyMock = vi.mocked(saveExecutionRiskPolicy)
const refreshOrdersMock = vi.mocked(refreshOrders)
const refreshStatusesMock = vi.mocked(refreshStatuses)
const resetLocalPaperBrokerMock = vi.mocked(resetLocalPaperBroker)
const startLiveMock = vi.mocked(startLive)
const stopLiveMock = vi.mocked(stopLive)
const startRunnerMock = vi.mocked(startRunner)
const stopRunnerMock = vi.mocked(stopRunner)
const clearKeysMock = vi.mocked(clearKeys)
const subscribeExecutionRuntimeEventsMock = vi.mocked(subscribeExecutionRuntimeEvents)
const previewExecutionRiskPolicyMock = vi.mocked(previewExecutionRiskPolicy)

async function flushMicrotasks(): Promise<void> {
  for (let index = 0; index < 6; index += 1) {
    await Promise.resolve()
  }
}

async function waitForStoreToSettle(store: ReturnType<typeof useExecutionStore>): Promise<void> {
  for (let index = 0; index < 10; index += 1) {
    await flushMicrotasks()
    if (!store.isBusy) break
  }
}

function deferred<T>(): {
  promise: Promise<T>
  resolve: (value: T) => void
  reject: (error?: unknown) => void
} {
  let resolve!: (value: T) => void
  let reject!: (error?: unknown) => void
  const promise = new Promise<T>((innerResolve, innerReject) => {
    resolve = innerResolve
    reject = innerReject
  })
  return { promise, resolve, reject }
}

describe('execution store actions', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    runtimeEventHandlers = null
    canonicalSymbolMock.mockImplementation((raw: unknown) => String(raw ?? '').trim().replace(/\s+/g, '').toUpperCase())
    cancelOpenOrdersMock.mockClear()
    cancelOpenOrdersMock.mockResolvedValue({ statusLine: 'Cancelled 0 open broker orders.' })
    testConnectionMock.mockClear()
    testConnectionMock.mockResolvedValue({ statusLine: 'Connection OK.' })
    emergencyHaltMock.mockClear()
    emergencyHaltMock.mockResolvedValue({ statusLine: 'Emergency halt applied.', recoveryReviewRequired: true, liveActive: false, runnerActive: false })
    flattenPositionMock.mockClear()
    flattenPositionMock.mockResolvedValue({ statusLine: 'No open broker position to flatten.' })
    getStoredExecutionCredentialStatusMock.mockClear()
    listSavedLiveSessionsMock.mockClear()
    loadSavedLiveSessionDetailMock.mockClear()
    getStoredExecutionCredentialStatusMock.mockResolvedValue({
      rememberAvailable: true,
      hasApiKey: false,
      hasApiSecret: false,
      hasCredentials: false,
      state: 'missing',
      storageBackend: 'keyring',
      message: '',
    })
    migrateLegacyStoredExecutionKeysToSecureStorageMock.mockClear()
    loadExecutionRiskPolicyMock.mockClear()
    loadExecutionRiskPolicyMock.mockResolvedValue({
      max_order_notional: null,
      max_position_notional: null,
      allow_short: null,
      require_bracket: null,
    })
    listSavedLiveSessionsMock.mockResolvedValue([])
    loadSavedLiveSessionDetailMock.mockResolvedValue(null as never)
    migrateLegacyStoredExecutionKeysToSecureStorageMock.mockResolvedValue({
      rememberAvailable: true,
      hasApiKey: false,
      hasApiSecret: false,
      hasCredentials: false,
      state: 'missing',
      storageBackend: 'keyring',
      message: '',
    })
    normalizeExecutionOperatorErrorMock.mockClear()
    normalizeExecutionOperatorErrorMock.mockImplementation((raw: unknown) => String(raw ?? '').trim())
    optimisticOrderRowFromExecutionMock.mockClear()
    optimisticOrderRowFromExecutionMock.mockImplementation((executionRaw: unknown) => {
      const execution = executionRaw as { order_submitted?: boolean; action?: string; order_id?: string; delta_qty?: number } | null
      return execution?.order_submitted
        ? { cells: ['2026-01-01 12:00:00', String(execution.action || ''), 'submitted*', String(execution.delta_qty || ''), '-', '-', String(execution.order_id || '')] }
        : null
    })
    runOnceMock.mockClear()
    runOnceMock.mockResolvedValue({ statusLine: 'Signal processed.' })
    saveExecutionRiskPolicyMock.mockClear()
    saveExecutionRiskPolicyMock.mockImplementation(async (policy) => ({
      max_order_notional: policy.max_order_notional ?? null,
      max_position_notional: policy.max_position_notional ?? null,
      allow_short: policy.allow_short ?? null,
      require_bracket: policy.require_bracket ?? null,
    }))
    refreshOrdersMock.mockClear()
    refreshOrdersMock.mockResolvedValue({ statusLine: 'Loaded 0 orders.' })
    refreshStatusesMock.mockClear()
    refreshStatusesMock.mockResolvedValue({ statusLine: 'Execution status refreshed.' })
    resetLocalPaperBrokerMock.mockClear()
    resetLocalPaperBrokerMock.mockResolvedValue({ statusLine: 'Local paper broker reset.' })
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
    previewExecutionRiskPolicyMock.mockClear()
    previewExecutionRiskPolicyMock.mockResolvedValue({ statusLine: 'Policy preview allowed.', statusHint: 'Policy preview allowed.' })
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

  it('hydrates stored secure credential status without hydrating raw secrets into inputs', async () => {
    const store = useExecutionStore()
    migrateLegacyStoredExecutionKeysToSecureStorageMock.mockResolvedValueOnce({
      rememberAvailable: true,
      hasApiKey: true,
      hasApiSecret: true,
      hasCredentials: true,
      state: 'stored',
      storageBackend: 'keyring',
      message: '',
    })

    await store.hydrateStoredCredentialStatus()

    expect(migrateLegacyStoredExecutionKeysToSecureStorageMock).toHaveBeenCalledTimes(1)
    expect(store.apiKey).toBe('')
    expect(store.apiSecret).toBe('')
    expect(store.rememberKeys).toBe(true)
    expect(store.hasExecutionCredentials).toBe(true)
    expect(store.keyStateLabel).toBe('Stored securely')
  })

  it('dispatches execution actions through execution service', async () => {
    const store = useExecutionStore()

    store.setInputField('execApiKey', 'key_1')
    store.setInputField('execApiSecret', 'secret_1')
    store.setInputField('execSymbol', 'AAPL')

    store.testConnection()
    await waitForStoreToSettle(store)
    store.runOnce()
    await waitForStoreToSettle(store)
    store.refreshOrders()
    await waitForStoreToSettle(store)
    store.cancelOpenOrders()
    await waitForStoreToSettle(store)
    store.flattenPosition()
    await waitForStoreToSettle(store)
    store.startLive()
    await waitForStoreToSettle(store)
    store.startRunner()
    await waitForStoreToSettle(store)
    store.stopRunner()
    await waitForStoreToSettle(store)
    store.stopLive()
    await waitForStoreToSettle(store)
    store.refreshStatuses()
    await waitForStoreToSettle(store)
    store.clearKeys()
    await waitForStoreToSettle(store)

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
    expect(cancelOpenOrdersMock).toHaveBeenCalledWith(expect.objectContaining({
      mode: 'paper',
      symbol: 'AAPL',
    }))
    expect(flattenPositionMock).toHaveBeenCalledWith(expect.objectContaining({
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

  it('loads, saves, and previews execution risk policy through the execution service', async () => {
    const store = useExecutionStore()
    loadExecutionRiskPolicyMock.mockResolvedValueOnce({
      max_order_notional: 500,
      max_position_notional: 1500,
      allow_short: false,
      require_bracket: true,
    })

    await store.loadRiskPolicy()

    expect(store.riskMaxOrderNotional).toBe('500')
    expect(store.riskMaxPositionNotional).toBe('1500')
    expect(store.riskAllowShort).toBe(false)
    expect(store.riskRequireBracket).toBe(true)
    expect(store.riskPolicyStatusHint).toContain('max order=500')

    store.setInputField('execRiskMaxOrderNotional', '750')
    store.setInputField('execRiskMaxPositionNotional', '1750')
    store.setCheckboxField('execRiskAllowShort', true)
    store.setCheckboxField('execRiskRequireBracket', false)

    await store.saveRiskPolicy()

    expect(saveExecutionRiskPolicyMock).toHaveBeenCalledWith({
      max_order_notional: 750,
      max_position_notional: 1750,
      allow_short: true,
      require_bracket: false,
    })
    expect(store.statusLine).toBe('Execution risk policy saved.')

    store.setInputField('execSymbol', 'AAPL')
    store.previewRiskPolicy()
    await waitForStoreToSettle(store)

    expect(previewExecutionRiskPolicyMock).toHaveBeenCalledWith(expect.objectContaining({
      symbol: 'AAPL',
      riskMaxOrderNotional: '750',
      riskMaxPositionNotional: '1750',
      riskAllowShort: true,
      riskRequireBracket: false,
    }))
    expect(store.statusLine).toBe('Policy preview allowed.')
  })

  it('confirms local paper reset through the execution service only in local-paper mode', async () => {
    const store = useExecutionStore()

    expect(store.canResetLocalPaper).toBe(false)
    store.setInputField('execExecutionTarget', 'local_paper')
    expect(store.canResetLocalPaper).toBe(true)

    store.resetLocalPaper()
    await waitForStoreToSettle(store)

    expect(resetLocalPaperBrokerMock).toHaveBeenCalledWith(expect.objectContaining({
      executionTarget: 'local_paper',
    }))
    expect(store.statusLine).toBe('Local paper broker reset.')
  })

  it('does not save invalid risk-policy notional text as an unset cap', async () => {
    const store = useExecutionStore()

    store.setInputField('execRiskMaxOrderNotional', '-25')

    await store.saveRiskPolicy()

    expect(saveExecutionRiskPolicyMock).not.toHaveBeenCalled()
    expect(store.statusLine).toBe('Risk policy save failed: Error: Max order notional must be a positive number.')
    expect(store.statusHint).toBe('Risk policy save failed: Error: Max order notional must be a positive number.')
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
    await waitForStoreToSettle(store)
    expect(store.pending.runOnce).toBe(false)
  })

  it('keeps the last known live and runner state when a status refresh returns warnings only', async () => {
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
    refreshStatusesMock.mockResolvedValueOnce({
      statusLine: 'Execution status refreshed with warnings.',
      statusHint: 'Execution status refreshed with warnings. Auto-runner status refresh unavailable. Live-feed status refresh unavailable.',
      runnerStatusHint: 'Auto-runner status unavailable.',
      liveStatusHint: 'Live status unavailable.',
    })

    await store.refreshStatuses()

    expect(store.liveActive).toBe(true)
    expect(store.liveLifecycleState).toBe('started')
    expect(store.liveSymbol).toBe('AAPL')
    expect(store.runnerActive).toBe(true)
    expect(store.runnerLifecycleState).toBe('started')
    expect(store.liveStatusHint).toBe('Live status unavailable.')
    expect(store.runnerStatusHint).toBe('Auto-runner status unavailable.')
    expect(store.statusLine).toBe('Execution status refreshed with warnings.')
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

  it('derives operator-facing blocked reasons for runOnce, live feed, and runner start', () => {
    const store = useExecutionStore()

    expect(store.canRunOnce).toBe(false)
    expect(store.canRefreshOrders).toBe(false)
    expect(store.runOnceBlockedReasons).toContain('Enter a symbol before manual execution.')
    expect(store.refreshOrdersBlockedReasons).toContain('Enter a symbol before broker order review.')
    expect(store.startLiveBlockedReasons).toContain('Enter a symbol before starting live candles.')
    expect(store.startRunnerBlockedReasons).toContain('Enter a symbol before starting the auto-runner.')

    store.setInputField('execSymbol', 'AAPL')
    store.setCheckboxField('execDryRun', false)

    expect(store.canRunOnce).toBe(false)
    expect(store.canRefreshOrders).toBe(false)
    expect(store.runOnceBlockedReasons).toContain('API key + secret are required for broker-armed execution.')
    expect(store.refreshOrdersBlockedReasons).toContain('API key + secret are required for broker-backed order review.')
    expect(store.canStartLive).toBe(true)
    expect(store.startLiveBlockedReasons).toHaveLength(0)
    expect(store.startRunnerBlockedReasons).toContain(
      'Start live candles first. The auto-runner only reacts to active live bars.',
    )
    expect(store.startRunnerBlockedReasons).toContain(
      'API key + secret are required for broker-armed auto-runner start.',
    )

    store.setInputField('execApiKey', 'key_1')
    store.setInputField('execApiSecret', 'secret_1')
    expect(store.canRunOnce).toBe(true)

    store.applyPolygonLiveStatusEvent({
      active: true,
      symbol: 'TSLA',
      delayed: true,
      hint: 'Live candles active for TSLA [delayed].',
      status: 'started',
      message: 'tsla live',
    })

    expect(store.startRunnerBlockedReasons).toContain('Live candles are active for TSLA, not AAPL.')
  })

  it('derives safety-control availability and keeps emergency halt local to runtime state', async () => {
    const store = useExecutionStore()

    expect(store.canCancelOpenOrders).toBe(false)
    expect(store.canFlattenPosition).toBe(false)
    expect(store.canEmergencyHalt).toBe(false)
    expect(store.cancelOpenOrdersBlockedReasons).toContain(
      'Enter a symbol before cancelling open broker orders.',
    )
    expect(store.flattenPositionBlockedReasons).toContain(
      'Enter a symbol before flattening the broker position.',
    )
    expect(store.emergencyHaltBlockedReasons).toContain('Live automation is already disarmed.')

    store.setInputField('execSymbol', 'AAPL')
    store.setInputField('execApiKey', 'key_1')
    store.setInputField('execApiSecret', 'secret_1')

    expect(store.canCancelOpenOrders).toBe(true)
    expect(store.canFlattenPosition).toBe(true)

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

    expect(store.canCancelOpenOrders).toBe(false)
    expect(store.canFlattenPosition).toBe(false)
    expect(store.canEmergencyHalt).toBe(true)
    expect(store.cancelOpenOrdersBlockedReasons).toContain(
      'Stop the auto-runner before cancelling broker open orders.',
    )
    expect(store.flattenPositionBlockedReasons).toContain(
      'Stop the auto-runner before flattening the broker position.',
    )

    store.emergencyHalt()
    await waitForStoreToSettle(store)

    expect(emergencyHaltMock).toHaveBeenCalledTimes(1)
    expect(store.recoveryReviewRequired).toBe(true)
    expect(store.liveActive).toBe(false)
    expect(store.runnerActive).toBe(false)
    expect(store.canStartRunner).toBe(false)
  })

  it('treats stored secure credentials as effective credentials without hydrating input values', async () => {
    const store = useExecutionStore()
    getStoredExecutionCredentialStatusMock.mockResolvedValueOnce({
      rememberAvailable: true,
      hasApiKey: true,
      hasApiSecret: true,
      hasCredentials: true,
      state: 'stored',
      storageBackend: 'keyring',
      message: '',
    })

    await store.refreshStoredCredentialStatus()
    store.setInputField('execSymbol', 'AAPL')
    store.setCheckboxField('execDryRun', false)

    expect(store.apiKey).toBe('')
    expect(store.apiSecret).toBe('')
    expect(store.hasExecutionCredentials).toBe(true)
    expect(store.canRunOnce).toBe(true)
    expect(store.runOnceBlockedReasons).not.toContain('API key + secret are required for broker-armed execution.')
  })

  it('tracks key state across missing, session-only, stored, and pending secure-store cases', async () => {
    const store = useExecutionStore()

    expect(store.keyStateLabel).toBe('Missing')

    store.setInputField('execApiKey', 'key_1')
    expect(store.keyStateLabel).toBe('Incomplete')

    store.setInputField('execApiSecret', 'secret_1')
    expect(store.keyStateLabel).toBe('Session only')

    store.setCheckboxField('execRememberKeys', true)
    expect(store.keyStateLabel).toBe('Will store on next successful broker action')

    getStoredExecutionCredentialStatusMock.mockResolvedValueOnce({
      rememberAvailable: true,
      hasApiKey: true,
      hasApiSecret: true,
      hasCredentials: true,
      state: 'stored',
      storageBackend: 'keyring',
      message: '',
    })
    await store.refreshStoredCredentialStatus()
    store.typedCredentialStorePending = false

    expect(store.keyStateLabel).toBe('Stored securely')

    store.setInputField('execApiKey', 'key_2')
    store.setInputField('execApiSecret', 'secret_2')
    expect(store.keyStateLabel).toBe('Will update stored credentials')
  })

  it('shows remember-unavailable when secure storage is unavailable', async () => {
    const store = useExecutionStore()
    migrateLegacyStoredExecutionKeysToSecureStorageMock.mockResolvedValueOnce({
      rememberAvailable: false,
      hasApiKey: false,
      hasApiSecret: false,
      hasCredentials: false,
      state: 'unavailable',
      storageBackend: '',
      message: 'Secure storage unavailable.',
    })

    await store.hydrateStoredCredentialStatus()

    expect(store.keyStateLabel).toBe('Remember unavailable')
    expect(store.keyStateDetail).toContain('Secure storage unavailable.')
  })

  it('refuses to keep remember mode enabled when secure storage is unavailable', async () => {
    const store = useExecutionStore()
    migrateLegacyStoredExecutionKeysToSecureStorageMock.mockResolvedValueOnce({
      rememberAvailable: false,
      hasApiKey: false,
      hasApiSecret: false,
      hasCredentials: false,
      state: 'unavailable',
      storageBackend: '',
      message: 'Secure storage unavailable.',
    })

    await store.hydrateStoredCredentialStatus()
    store.setCheckboxField('execRememberKeys', true)

    expect(store.rememberKeys).toBe(false)
    expect(store.keyStateLabel).toBe('Remember unavailable')
  })

  it('derives the current execution mode matrix labels conservatively', () => {
    const store = useExecutionStore()

    expect(store.submissionModeLabel).toBe('Simulation')

    store.setCheckboxField('execDryRun', false)
    expect(store.submissionModeLabel).toBe('Paper order')

    store.setSelectField('execMode', 'live')
    expect(store.brokerModeLabel).toBe('Live')
    expect(store.submissionModeLabel).toBe('Live order')

    store.setCheckboxField('execDryRun', true)
    expect(store.submissionModeLabel).toBe('Simulation')
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

  it('reuses a single runtime subscription install while concurrent callers wait', async () => {
    const store = useExecutionStore()
    const setup = deferred<() => void>()
    subscribeExecutionRuntimeEventsMock.mockImplementationOnce(
      async (handlers: ExecutionRuntimeEventHandlers) => {
        runtimeEventHandlers = handlers
        return await setup.promise
      },
    )

    const first = store.ensureRuntimeSubscriptions()
    const second = store.ensureRuntimeSubscriptions()
    await flushMicrotasks()

    expect(subscribeExecutionRuntimeEventsMock).toHaveBeenCalledTimes(1)

    setup.resolve(() => {
      runtimeEventHandlers = null
    })

    await Promise.all([first, second])
    await store.ensureRuntimeSubscriptions()

    expect(subscribeExecutionRuntimeEventsMock).toHaveBeenCalledTimes(1)
  })

  it('retries runtime subscription installation after an initial failure', async () => {
    const store = useExecutionStore()
    subscribeExecutionRuntimeEventsMock
      .mockRejectedValueOnce(new Error('listen unavailable'))
      .mockImplementationOnce(async (handlers: ExecutionRuntimeEventHandlers) => {
        runtimeEventHandlers = handlers
        return () => {
          runtimeEventHandlers = null
        }
      })

    await expect(store.ensureRuntimeSubscriptions()).rejects.toThrow('listen unavailable')
    await store.ensureRuntimeSubscriptions()

    expect(subscribeExecutionRuntimeEventsMock).toHaveBeenCalledTimes(2)
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

  it('keeps runner active across state-neutral error events while still surfacing the error', () => {
    const store = useExecutionStore()
    store.applyLiveExecutionRunnerEvent({
      status: 'started',
      symbol: 'AAPL',
      message: 'runner started',
      hint: 'Auto-runner: started',
      active: true,
      execution: null,
    })

    store.applyLiveExecutionRunnerEvent({
      status: 'error',
      symbol: 'AAPL',
      message: 'runner execution failed: timeout',
      hint: 'Auto-runner: runner execution failed: timeout',
      active: null,
      execution: null,
    })

    expect(store.runnerActive).toBe(true)
    expect(store.runnerStatusHint).toBe('Auto-runner: runner execution failed: timeout')
    expect(store.runnerStateLabel).toBe('Active · recent error')
    expect(store.statusLine).toBe('runner execution failed: timeout')
  })

  it('enters a review-required inactive posture after feed closure and backend runner stop', () => {
    const store = useExecutionStore()
    store.setInputField('execSymbol', 'AAPL')
    store.setCheckboxField('execDryRun', false)
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

    store.applyPolygonLiveStatusEvent({
      active: false,
      symbol: 'AAPL',
      delayed: true,
      hint: 'Live feed for AAPL [delayed] disconnected. Refresh broker review before re-arming automation.',
      status: 'closed',
      message: 'Live feed for AAPL [delayed] disconnected. Refresh broker review before re-arming automation.',
    })
    store.applyLiveExecutionRunnerEvent({
      status: 'stopped',
      symbol: 'AAPL',
      message: 'auto-runner stopped',
      hint: 'Auto-runner: stopped',
      active: false,
      execution: null,
    })

    expect(store.liveActive).toBe(false)
    expect(store.runnerActive).toBe(false)
    expect(store.feedStateLabel).toBe('Disconnected · AAPL [delayed]')
    expect(store.liveLifecycleLabel).toBe('Disconnected')
    expect(store.liveStatusHint).toContain('disconnected')
    expect(store.runnerStatusHint).toBe('Auto-runner: stopped')
    expect(store.runnerStateLabel).toBe('Stopped · review required')
    expect(store.canStartRunner).toBe(false)
    expect(store.reviewMeta).toContain('pending broker refresh')
  })

  it('keeps broker-armed runner restart blocked after same-symbol live restart until broker review refresh completes', () => {
    const store = useExecutionStore()
    const freshNow = Date.now()
    store.setInputField('execSymbol', 'AAPL')
    store.setCheckboxField('execDryRun', false)
    store.setInputField('execApiKey', 'key_1')
    store.setInputField('execApiSecret', 'secret_1')

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

    store.applyPolygonLiveStatusEvent({
      active: false,
      symbol: 'AAPL',
      delayed: true,
      hint: 'Live feed for AAPL [delayed] disconnected. Refresh broker review before re-arming automation.',
      status: 'closed',
      message: 'Live feed for AAPL [delayed] disconnected. Refresh broker review before re-arming automation.',
    })
    store.applyLiveExecutionRunnerEvent({
      status: 'stopped',
      symbol: 'AAPL',
      message: 'auto-runner stopped',
      hint: 'Auto-runner: stopped',
      active: false,
      execution: null,
    })

    store.applyPolygonLiveStatusEvent({
      active: true,
      symbol: 'AAPL',
      delayed: true,
      hint: 'Live candles started for AAPL [delayed].',
      status: 'started',
      message: 'live restarted',
    })

    expect(store.liveActive).toBe(true)
    expect(store.recoveryReviewRequired).toBe(true)
    expect(store.canStartRunner).toBe(false)
    expect(store.startRunnerBlockedReasons).toContain(
      'Refresh broker review after the last live interruption before re-arming the auto-runner.',
    )
    expect(store.recoveryStatusLabel).toContain('Feed interruption:')

    store.applyActionResult({
      statusLine: 'Loaded 0 recent orders for AAPL.',
      accountReview: {
        state: 'broker_confirmed',
        mode: 'paper',
        sourceCommand: 'get_live_execution_snapshot',
        fetchedAtUnixMs: freshNow,
        accountId: '',
        accountNumber: 'ABC123',
        status: 'ACTIVE',
        currency: 'USD',
        buyingPower: '100000',
        cash: '25000',
        portfolioValue: '125000',
      },
      brokerReview: {
        sourceCommand: 'get_live_execution_snapshot',
        mode: 'paper',
        symbol: 'AAPL',
        fetchedAtUnixMs: freshNow,
        staleThresholdMs: 90_000,
        account: {
          state: 'broker_confirmed',
          mode: 'paper',
          sourceCommand: 'get_live_execution_snapshot',
          fetchedAtUnixMs: freshNow,
          accountId: '',
          accountNumber: 'ABC123',
          status: 'ACTIVE',
          currency: 'USD',
          buyingPower: '100000',
          cash: '25000',
          portfolioValue: '125000',
        },
        position: {
          state: 'flat',
          symbol: 'AAPL',
          qty: 0,
          side: '',
          avgEntryPrice: null,
          currentPrice: null,
          marketValue: null,
          unrealizedPl: null,
          unrealizedPlpc: null,
        },
        orders: {
          state: 'broker_confirmed',
          scope: 'symbol_scoped',
          sourceCommand: 'get_live_execution_snapshot',
          fetchedAtUnixMs: freshNow,
          totalCount: 0,
          openCount: 0,
        },
        chart: {
          state: 'loaded_broker_markers',
          symbol: 'AAPL',
          datasetId: 'AAPL',
          datasetLabel: 'AAPL',
          datasetSymbol: 'AAPL',
          orderCount: 0,
          sourceCommand: 'get_live_execution_snapshot',
          fetchedAtUnixMs: freshNow,
        },
      },
      orderRows: [],
      orderCount: 0,
    })

    expect(store.recoveryReviewRequired).toBe(false)
    expect(store.canStartRunner).toBe(true)
    expect(store.recoveryStatusLabel).toBe('No recovery review pending.')
  })

  it('preserves active runner state when backend emits an error event without an active flag change', () => {
    const store = useExecutionStore()
    store.applyLiveExecutionRunnerEvent({
      status: 'started',
      symbol: 'AAPL',
      message: 'runner started',
      hint: 'Auto-runner: started',
      active: true,
      execution: null,
    })

    store.applyLiveExecutionRunnerEvent({
      status: 'error',
      symbol: 'AAPL',
      message: 'runner execution failed: boom',
      hint: 'Auto-runner: runner execution failed: boom',
      active: null,
      execution: null,
    })

    expect(store.runnerActive).toBe(true)
    expect(store.runnerStatusHint).toBe('Auto-runner: runner execution failed: boom')
    expect(store.statusLine).toBe('runner execution failed: boom')
    expect(store.statusHint).toBe('runner execution failed: boom')
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

  it('surfaces clear-failed credential truth when secure clearing fails', async () => {
    const store = useExecutionStore()
    clearKeysMock.mockRejectedValueOnce(new Error('Keyring clear failed'))
    store.setCheckboxField('execRememberKeys', true)
    store.setInputField('execApiKey', 'key_1')
    store.setInputField('execApiSecret', 'secret_1')

    store.clearKeys()
    await waitForStoreToSettle(store)

    expect(store.storedCredentialClearFailed).toBe(true)
    expect(store.keyStateLabel).toBe('Clear failed')
    expect(store.keyStateDetail).toContain('Keyring clear failed')
    expect(store.statusLine).toContain('Clear keys failed')
  })

  it('redacts typed execution secrets from surfaced action failures', async () => {
    const store = useExecutionStore()
    store.setCheckboxField('execDryRun', false)
    store.setInputField('execSymbol', 'AAPL')
    store.setInputField('execApiKey', 'key_1')
    store.setInputField('execApiSecret', 'secret_1')
    testConnectionMock.mockRejectedValueOnce(new Error('broker rejected key_1 / secret_1'))

    store.testConnection()
    await waitForStoreToSettle(store)

    expect(store.statusLine).toContain('[redacted]')
    expect(store.statusLine).not.toContain('key_1')
    expect(store.statusLine).not.toContain('secret_1')
    expect(store.outputPreview).toContain('[redacted]')
  })

  it('replaces optimistic rows with broker-confirmed review rows on refresh', async () => {
    const store = useExecutionStore()
    const freshNow = Date.now()
    store.setInputField('execSymbol', 'AAPL')
    store.setInputField('execApiKey', 'key_1')
    store.setInputField('execApiSecret', 'secret_1')
    store.applyLiveExecutionRunnerEvent({
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
    expect(store.orderRows[0]?.cells?.[2]).toBe('submitted*')

    refreshOrdersMock.mockResolvedValueOnce({
      statusLine: 'Loaded 1 recent order for AAPL.',
      orderRows: [
        {
          cells: ['2026-04-10 14:55:00', 'buy', 'filled', '1', '1', '182.40', 'ord_123'],
        },
      ],
      orderCount: 1,
      accountReview: {
        state: 'broker_confirmed',
        mode: 'paper',
        sourceCommand: 'get_live_execution_snapshot',
        fetchedAtUnixMs: freshNow,
        accountId: '',
        accountNumber: 'ABC123',
        status: 'ACTIVE',
        currency: 'USD',
        buyingPower: '100000',
        cash: '25000',
        portfolioValue: '125000',
      },
      brokerReview: {
        sourceCommand: 'get_live_execution_snapshot',
        mode: 'paper',
        symbol: 'AAPL',
        fetchedAtUnixMs: freshNow,
        staleThresholdMs: 90_000,
        account: {
          state: 'broker_confirmed',
          mode: 'paper',
          sourceCommand: 'get_live_execution_snapshot',
          fetchedAtUnixMs: freshNow,
          accountId: '',
          accountNumber: 'ABC123',
          status: 'ACTIVE',
          currency: 'USD',
          buyingPower: '100000',
          cash: '25000',
          portfolioValue: '125000',
        },
        position: {
          state: 'flat',
          symbol: 'AAPL',
          qty: 0,
          side: '',
          avgEntryPrice: null,
          currentPrice: null,
          marketValue: null,
          unrealizedPl: null,
          unrealizedPlpc: null,
        },
        orders: {
          state: 'broker_confirmed',
          scope: 'symbol_scoped',
          sourceCommand: 'get_live_execution_snapshot',
          fetchedAtUnixMs: freshNow,
          totalCount: 1,
          openCount: 0,
        },
        chart: {
          state: 'sync_pending',
          symbol: 'AAPL',
          datasetId: 'AAPL',
          datasetLabel: 'AAPL',
          datasetSymbol: 'AAPL',
          orderCount: 1,
          sourceCommand: 'get_live_execution_snapshot',
          fetchedAtUnixMs: freshNow,
        },
      },
      positionMeta: 'Position: flat',
      chartMeta: 'Execution chart loaded for AAPL with 1 broker-confirmed order marker.',
      reviewMeta: 'Broker review: paper symbol-scoped AAPL · as of 2026-04-10 14:55:00 UTC · status=ACTIVE · 0 open orders',
      outputPreview: '{"orders":[{"id":"ord_123"}]}',
    })

    store.refreshOrders()
    await waitForStoreToSettle(store)

    expect(store.orderRows[0]?.cells?.[2]).toBe('filled')
    expect(store.orderCount).toBe(1)
    expect(store.positionMeta).toBe('Position: flat')
    expect(store.chartMeta).toBe('Execution chart loaded for AAPL with 1 broker-confirmed order marker.')
    expect(store.reviewMeta).toContain('Broker review: paper symbol-scoped AAPL')
    expect(store.reconciliationState).toBe('broker_confirmed_flat')
    expect(store.accountReview.accountNumber).toBe('ABC123')
  })

  it('treats optimistic local orders as pending manual broker refresh even when a prior broker review was fresh', () => {
    const store = useExecutionStore()
    const freshNow = Date.now()

    store.setInputField('execSymbol', 'AAPL')
    store.accountReview = {
      state: 'broker_confirmed',
      mode: 'paper',
      sourceCommand: 'get_live_execution_snapshot',
      fetchedAtUnixMs: freshNow,
      accountId: '',
      accountNumber: 'ABC123',
      status: 'ACTIVE',
      currency: 'USD',
      buyingPower: '100000',
      cash: '25000',
      portfolioValue: '125000',
    }
    store.brokerReview = {
      sourceCommand: 'get_live_execution_snapshot',
      mode: 'paper',
      symbol: 'AAPL',
      fetchedAtUnixMs: freshNow,
      staleThresholdMs: 90_000,
      account: store.accountReview,
      position: {
        state: 'flat',
        symbol: 'AAPL',
        qty: 0,
        side: '',
        avgEntryPrice: null,
        currentPrice: null,
        marketValue: null,
        unrealizedPl: null,
        unrealizedPlpc: null,
      },
      orders: {
        state: 'broker_confirmed',
        scope: 'symbol_scoped',
        sourceCommand: 'get_live_execution_snapshot',
        fetchedAtUnixMs: freshNow,
        totalCount: 0,
        openCount: 0,
      },
      chart: {
        state: 'loaded_no_markers',
        symbol: 'AAPL',
        datasetId: 'AAPL',
        datasetLabel: 'AAPL',
        datasetSymbol: 'AAPL',
        orderCount: 0,
        sourceCommand: 'get_live_execution_snapshot',
        fetchedAtUnixMs: freshNow,
      },
    }

    store.applyActionResult({
      statusLine: 'Submitted BUY order.',
      prependOrderRows: [
        {
          cells: ['2026-04-10 14:55:00', 'buy', 'submitted*', '1', '-', '-', 'ord_123'],
        },
      ],
      brokerReviewRefreshRequired: true,
    })

    expect(store.reconciliationState).toBe('optimistic_local_pending')
    expect(store.reviewMeta).toContain('waiting for broker confirmation')
    expect(store.positionMeta).toBe('Position: pending broker refresh.')
    expect(store.chartMeta).toContain('pending refreshed broker review')
    expect(store.reconciliationAdvisories).toEqual([
      'Broker review is waiting on 1 optimistic local order. Refresh orders before the next broker-armed action.',
    ])
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

  it('derives reconciliation advisories for optimistic, stale, and open broker states', () => {
    const nowSpy = vi.spyOn(Date, 'now').mockReturnValue(1_744_400_000_000)
    const store = useExecutionStore()

    store.setInputField('execSymbol', 'AAPL')
    store.orderRows = [
      {
        cells: ['2026-01-01 12:00:00', 'buy', 'submitted*', '1', '-', '-', 'ord_001'],
      },
    ]
    store.orderCount = 1
    expect(store.reconciliationAdvisories).toEqual([
      'Broker review is waiting on 1 optimistic local order. Refresh orders before the next broker-armed action.',
    ])

    store.brokerReview = {
      sourceCommand: 'get_live_execution_snapshot',
      mode: 'paper',
      symbol: 'AAPL',
      fetchedAtUnixMs: 1_744_399_800_000,
      staleThresholdMs: 90_000,
      account: {
        state: 'broker_confirmed',
        mode: 'paper',
        sourceCommand: 'get_live_execution_snapshot',
        fetchedAtUnixMs: 1_744_399_800_000,
        accountId: '',
        accountNumber: 'ABC123',
        status: 'ACTIVE',
        currency: 'USD',
        buyingPower: '100000',
        cash: '25000',
        portfolioValue: '125000',
      },
      position: {
        state: 'flat',
        symbol: 'AAPL',
        qty: 0,
        side: '',
        avgEntryPrice: null,
        currentPrice: null,
        marketValue: null,
        unrealizedPl: null,
        unrealizedPlpc: null,
      },
      orders: {
        state: 'broker_confirmed',
        scope: 'symbol_scoped',
        sourceCommand: 'get_live_execution_snapshot',
        fetchedAtUnixMs: 1_744_399_800_000,
        totalCount: 2,
        openCount: 2,
      },
      chart: {
        state: 'stale_broker_review',
        symbol: 'AAPL',
        datasetId: 'AAPL',
        datasetLabel: 'AAPL',
        datasetSymbol: 'AAPL',
        orderCount: 2,
        sourceCommand: 'get_live_execution_snapshot',
        fetchedAtUnixMs: 1_744_399_800_000,
      },
    }
    expect(store.reconciliationAdvisories).toEqual([
      'Broker snapshot is stale. Refresh orders before the next broker-armed action.',
    ])

    store.brokerReview = {
      ...store.brokerReview,
      fetchedAtUnixMs: 1_744_399_980_000,
      staleThresholdMs: 90_000,
      position: {
        ...store.brokerReview.position,
        state: 'open',
        qty: 10,
        side: 'long',
      },
      orders: {
        ...store.brokerReview.orders,
        fetchedAtUnixMs: 1_744_399_980_000,
        openCount: 0,
      },
      chart: {
        ...store.brokerReview.chart,
        fetchedAtUnixMs: 1_744_399_980_000,
      },
    }
    expect(store.reconciliationAdvisories).toEqual([
      'Broker-confirmed position is already open. Review the current position before the next broker-armed action.',
    ])

    nowSpy.mockRestore()
  })

  it('refreshes saved live session evidence and loads the default detail', async () => {
    const store = useExecutionStore()
    listSavedLiveSessionsMock.mockResolvedValueOnce([
      {
        ts: '2026-04-12T15:40:00Z',
        path: 'C:/runs/vectorgrid_live_session_latest.json',
        sessionId: 'sess_latest',
        mode: 'paper',
        symbol: 'AAPL',
        strategyKind: 'DoubleMa',
        dataId: 'data_aapl',
        dryRun: false,
        paramSource: 'selected',
        sessionState: 'completed',
        createdAtUnixMs: 1_900_000_000_000,
        finalizedAtUnixMs: 1_900_000_000_500,
        eventCount: 2,
        brokerSnapshotCount: 1,
      },
      {
        ts: '2026-04-12T15:30:00Z',
        path: 'C:/runs/vectorgrid_live_session_older.json',
        sessionId: 'sess_older',
        mode: 'paper',
        symbol: 'MSFT',
        strategyKind: 'DoubleMa',
        dataId: 'data_msft',
        dryRun: true,
        paramSource: 'selected',
        sessionState: 'completed',
        createdAtUnixMs: 1_899_999_999_000,
        finalizedAtUnixMs: 1_899_999_999_500,
        eventCount: 1,
        brokerSnapshotCount: 0,
      },
    ] as never)
    loadSavedLiveSessionDetailMock.mockResolvedValueOnce({
      ts: '2026-04-12T15:40:00Z',
      artifactPath: 'C:/runs/vectorgrid_live_session_latest.json',
      sessionId: 'sess_latest',
      sessionState: 'completed',
      mode: 'paper',
      symbol: 'AAPL',
      qty: 2,
      dryRun: false,
      strategyKind: 'DoubleMa',
      dataId: 'data_aapl',
      paramSource: 'selected',
      createdAtUnixMs: 1_900_000_000_000,
      finalizedAtUnixMs: 1_900_000_000_500,
      sourceRunArtifactPath: 'C:/runs/vectorgrid_run_2026-04-12.json',
      sourceRunConfigFingerprint: 'cfg1-fnv1a64-live',
      sourceRunRouteSummary:
        'requested=GpuOnly -> resolved=GpuKernel -> realized=CpuOnly (fallback: cuda unavailable)',
      eventCount: 2,
      brokerSnapshotCount: 1,
      events: [
        {
          provenance: 'local_control',
          status: 'start_runner',
          message: 'runner armed',
          symbol: 'AAPL',
          atUnixMs: 1_900_000_000_000,
          barOpenUnixMs: null,
          action: '',
          orderId: '',
          deltaQty: null,
          orderSubmitted: null,
          warnings: [],
        },
      ],
      brokerSnapshots: [],
    } as never)

    await store.refreshSavedSessions()
    await waitForStoreToSettle(store)

    expect(listSavedLiveSessionsMock).toHaveBeenCalledTimes(1)
    expect(loadSavedLiveSessionDetailMock).toHaveBeenCalledWith(
      'C:/runs/vectorgrid_live_session_latest.json',
    )
    expect(store.savedSessions).toHaveLength(2)
    expect(store.selectedSavedSessionPath).toBe('C:/runs/vectorgrid_live_session_latest.json')
    expect(store.savedSessionDetail?.sessionId).toBe('sess_latest')
    expect(store.savedSessionStatusHint).toBe('Loaded 2 saved live session artifacts.')
  })

  it('loads an explicitly selected saved live session detail', async () => {
    const store = useExecutionStore()
    store.savedSessions = [
      {
        ts: '2026-04-12T15:40:00Z',
        path: 'C:/runs/vectorgrid_live_session_latest.json',
        sessionId: 'sess_latest',
        mode: 'paper',
        symbol: 'AAPL',
        strategyKind: 'DoubleMa',
        dataId: 'data_aapl',
        dryRun: false,
        paramSource: 'selected',
        sessionState: 'completed',
        createdAtUnixMs: 1_900_000_000_000,
        finalizedAtUnixMs: 1_900_000_000_500,
        eventCount: 2,
        brokerSnapshotCount: 1,
      },
      {
        ts: '2026-04-12T15:30:00Z',
        path: 'C:/runs/vectorgrid_live_session_older.json',
        sessionId: 'sess_older',
        mode: 'paper',
        symbol: 'MSFT',
        strategyKind: 'DoubleMa',
        dataId: 'data_msft',
        dryRun: true,
        paramSource: 'selected',
        sessionState: 'completed',
        createdAtUnixMs: 1_899_999_999_000,
        finalizedAtUnixMs: 1_899_999_999_500,
        eventCount: 1,
        brokerSnapshotCount: 0,
      },
    ]
    loadSavedLiveSessionDetailMock.mockResolvedValueOnce({
      ts: '2026-04-12T15:30:00Z',
      artifactPath: 'C:/runs/vectorgrid_live_session_older.json',
      sessionId: 'sess_older',
      sessionState: 'completed',
      mode: 'paper',
      symbol: 'MSFT',
      qty: 1,
      dryRun: true,
      strategyKind: 'DoubleMa',
      dataId: 'data_msft',
      paramSource: 'selected',
      createdAtUnixMs: 1_899_999_999_000,
      finalizedAtUnixMs: 1_899_999_999_500,
      sourceRunArtifactPath: '',
      sourceRunConfigFingerprint: '',
      sourceRunRouteSummary: '',
      eventCount: 1,
      brokerSnapshotCount: 0,
      events: [
        {
          provenance: 'interruption',
          status: 'stream_closed',
          message: 'feed closed',
          symbol: 'MSFT',
          atUnixMs: 1_899_999_999_500,
          barOpenUnixMs: null,
          action: '',
          orderId: '',
          deltaQty: null,
          orderSubmitted: null,
          warnings: [],
        },
      ],
      brokerSnapshots: [],
    } as never)

    await store.selectSavedSession('C:/runs/vectorgrid_live_session_older.json')
    await waitForStoreToSettle(store)

    expect(loadSavedLiveSessionDetailMock).toHaveBeenCalledWith(
      'C:/runs/vectorgrid_live_session_older.json',
    )
    expect(store.selectedSavedSessionPath).toBe('C:/runs/vectorgrid_live_session_older.json')
    expect(store.savedSessionDetail?.sessionId).toBe('sess_older')
    expect(store.savedSessionStatusHint).toBe('Saved live session evidence loaded.')
  })
})
