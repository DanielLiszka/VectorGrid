import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { computed, nextTick, ref } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ExecutionWorkspacePanel from './ExecutionWorkspacePanel.vue'
import { useDataStore } from '../stores/dataStore'
import { useExecutionStore } from '../stores/executionStore'
import { useWorkflowStore } from '../stores/workflowStore'

const executionChartServiceMock = vi.hoisted(() => ({
  syncExecutionChart: vi.fn(async (_host: HTMLElement | null, input: {
    dataId: string
    datasetLabel: string
    symbol: string
    brokerReviewLoaded: boolean
    fetchedAtUnixMs: number | null
    staleThresholdMs: number
    orderRows: Array<{ cells: string[] }>
  }) => {
    const symbol = String(input.symbol || '').trim().toUpperCase()
    const datasetId = String(input.dataId || '').trim()
    const datasetLabel = String(input.datasetLabel || '').trim() || datasetId || '-'
    const stale =
      input.fetchedAtUnixMs != null &&
      Number.isFinite(input.fetchedAtUnixMs) &&
      Date.now() - input.fetchedAtUnixMs > input.staleThresholdMs
    const brokerMarkerCount = input.orderRows.filter((row) => !String(row.cells?.[2] || '').trim().endsWith('*')).length
    if (!symbol) {
      return {
        state: 'no_symbol',
        summary: 'Execution chart unavailable: select a symbol first.',
        symbol: '',
        datasetId,
        datasetLabel,
        datasetSymbol: '',
        fetchedAtUnixMs: input.fetchedAtUnixMs,
        markerCount: 0,
      }
    }
    if (!datasetId) {
      return {
        state: 'no_dataset',
        summary: 'Execution chart unavailable: load a workflow dataset first.',
        symbol,
        datasetId: '',
        datasetLabel: '-',
        datasetSymbol: '',
        fetchedAtUnixMs: input.fetchedAtUnixMs,
        markerCount: 0,
      }
    }
    if (!input.brokerReviewLoaded) {
      return {
        state: 'provenance_suppressed',
        summary: `Execution chart loaded for ${symbol}. Broker-confirmed markers stay suppressed until a symbol-scoped broker review is refreshed.`,
        symbol,
        datasetId,
        datasetLabel,
        datasetSymbol: symbol,
        fetchedAtUnixMs: null,
        markerCount: 0,
      }
    }
    if (stale) {
      return {
        state: 'stale_broker_review',
        summary: `Execution chart loaded for ${symbol}, but broker review is stale as of ${input.fetchedAtUnixMs}.`,
        symbol,
        datasetId,
        datasetLabel,
        datasetSymbol: symbol,
        fetchedAtUnixMs: input.fetchedAtUnixMs,
        markerCount: brokerMarkerCount,
      }
    }
    if (!brokerMarkerCount) {
      return {
        state: 'loaded_no_markers',
        summary: `Execution chart loaded for ${symbol}. No broker-confirmed order markers are present in the current review.`,
        symbol,
        datasetId,
        datasetLabel,
        datasetSymbol: symbol,
        fetchedAtUnixMs: input.fetchedAtUnixMs,
        markerCount: 0,
      }
    }
    return {
      state: 'loaded_broker_markers',
      summary: `Execution chart loaded for ${symbol} with ${brokerMarkerCount} broker-confirmed order marker${brokerMarkerCount === 1 ? '' : 's'}.`,
      symbol,
      datasetId,
      datasetLabel,
      datasetSymbol: symbol,
      fetchedAtUnixMs: input.fetchedAtUnixMs,
      markerCount: brokerMarkerCount,
    }
  }),
  fitExecutionChartContent: vi.fn(() => '[0.0..1.0]'),
  panExecutionChart: vi.fn((direction: 'left' | 'right') => direction === 'left' ? '[0.0..1.0]' : '[1.0..2.0]'),
  disposeExecutionChart: vi.fn(),
}))

vi.mock('../services/executionChartService', () => executionChartServiceMock)

vi.mock('../composables/useVirtualTableRows', () => ({
  useVirtualTableRows: (rows: { value: unknown[] }) => ({
    scrollElementRef: ref(null),
    virtualRows: computed(() => rows.value.map((row, index) => ({ index, row }))),
    topSpacerPx: computed(() => 0),
    bottomSpacerPx: computed(() => 0),
    scrollToTop: vi.fn(),
  }),
}))

describe('ExecutionWorkspacePanel', () => {
  function findButton(label: string): HTMLButtonElement | undefined {
    return Array.from(document.querySelectorAll('button')).find((button) =>
      String(button.textContent || '').trim() === label,
    ) as HTMLButtonElement | undefined
  }

  function primeExecutionPanelStores(pinia = createPinia()) {
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    const dataStore = useDataStore(pinia)
    vi.spyOn(executionStore, 'hydrateStoredCredentialStatus').mockResolvedValue()
    return { pinia, workflowStore, executionStore, dataStore }
  }

  beforeEach(() => {
    localStorage.clear()
    executionChartServiceMock.syncExecutionChart.mockReset()
    executionChartServiceMock.fitExecutionChartContent.mockReset()
    executionChartServiceMock.panExecutionChart.mockReset()
    executionChartServiceMock.disposeExecutionChart.mockReset()
    executionChartServiceMock.syncExecutionChart.mockImplementation(async (_host: HTMLElement | null, input: {
      dataId: string
      datasetLabel: string
      symbol: string
      brokerReviewLoaded: boolean
      fetchedAtUnixMs: number | null
      staleThresholdMs: number
      orderRows: Array<{ cells: string[] }>
    }) => {
      const symbol = String(input.symbol || '').trim().toUpperCase()
      const datasetId = String(input.dataId || '').trim()
      const datasetLabel = String(input.datasetLabel || '').trim() || datasetId || '-'
      const stale =
        input.fetchedAtUnixMs != null &&
        Number.isFinite(input.fetchedAtUnixMs) &&
        Date.now() - input.fetchedAtUnixMs > input.staleThresholdMs
      const brokerMarkerCount = input.orderRows.filter((row) => !String(row.cells?.[2] || '').trim().endsWith('*')).length
      if (!symbol) {
        return {
          state: 'no_symbol',
          summary: 'Execution chart unavailable: select a symbol first.',
          symbol: '',
          datasetId,
          datasetLabel,
          datasetSymbol: '',
          fetchedAtUnixMs: input.fetchedAtUnixMs,
          markerCount: 0,
        }
      }
      if (!datasetId) {
        return {
          state: 'no_dataset',
          summary: 'Execution chart unavailable: load a workflow dataset first.',
          symbol,
          datasetId: '',
          datasetLabel: '-',
          datasetSymbol: '',
          fetchedAtUnixMs: input.fetchedAtUnixMs,
          markerCount: 0,
        }
      }
      if (!input.brokerReviewLoaded) {
        return {
          state: 'provenance_suppressed',
          summary: `Execution chart loaded for ${symbol}. Broker-confirmed markers stay suppressed until a symbol-scoped broker review is refreshed.`,
          symbol,
          datasetId,
          datasetLabel,
          datasetSymbol: symbol,
          fetchedAtUnixMs: null,
          markerCount: 0,
        }
      }
      if (stale) {
        return {
          state: 'stale_broker_review',
          summary: `Execution chart loaded for ${symbol}, but broker review is stale as of ${input.fetchedAtUnixMs}.`,
          symbol,
          datasetId,
          datasetLabel,
          datasetSymbol: symbol,
          fetchedAtUnixMs: input.fetchedAtUnixMs,
          markerCount: brokerMarkerCount,
        }
      }
      if (!brokerMarkerCount) {
        return {
          state: 'loaded_no_markers',
          summary: `Execution chart loaded for ${symbol}. No broker-confirmed order markers are present in the current review.`,
          symbol,
          datasetId,
          datasetLabel,
          datasetSymbol: symbol,
          fetchedAtUnixMs: input.fetchedAtUnixMs,
          markerCount: 0,
        }
      }
      return {
        state: 'loaded_broker_markers',
        summary: `Execution chart loaded for ${symbol} with ${brokerMarkerCount} broker-confirmed order marker${brokerMarkerCount === 1 ? '' : 's'}.`,
        symbol,
        datasetId,
        datasetLabel,
        datasetSymbol: symbol,
        fetchedAtUnixMs: input.fetchedAtUnixMs,
        markerCount: brokerMarkerCount,
      }
    })
    executionChartServiceMock.fitExecutionChartContent.mockImplementation(() => '[0.0..1.0]')
    executionChartServiceMock.panExecutionChart.mockImplementation((direction: 'left' | 'right') =>
      direction === 'left' ? '[0.0..1.0]' : '[1.0..2.0]',
    )
    executionChartServiceMock.disposeExecutionChart.mockImplementation(() => {})
    document.body.innerHTML = `
      <div id="liveShellGrid">
        <div id="executionShellPane">
          <div id="executionVueShellMount"></div>
        </div>
      </div>
    `
  })

  it('renders the empty broker-order state when no orders exist', async () => {
    const { pinia, workflowStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Broker order activity')
    expect(text).toContain('No broker orders for selected instrument.')
  })

  it('renders unknown review metadata distinctly before broker refresh confirms flat state', async () => {
    const { pinia, workflowStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Account review: not verified.')
    expect(text).toContain('Broker review: unavailable until the selected symbol is refreshed.')
    expect(text).toContain('Position: -')
    expect(text).not.toContain('Position: flat')
    expect(text).toContain('Execution chart unavailable: load a workflow dataset first.')
  })

  it('surfaces degraded feed and runner posture in the summary cards', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.symbol = 'AAPL'
    executionStore.liveActive = false
    executionStore.liveSymbol = 'AAPL'
    executionStore.liveDelayed = true
    executionStore.liveLifecycleState = 'closed'
    executionStore.liveStatusHint = 'Live feed for AAPL [delayed] disconnected. Refresh broker review before re-arming automation.'
    executionStore.runnerActive = false
    executionStore.runnerLifecycleState = 'stopped'
    executionStore.recoveryReviewRequired = true
    executionStore.runnerStatusHint = 'Auto-runner: stopped'

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const feedCard = document.querySelector('.executionPostureCard[data-label="Feed"]')
    const runnerCard = document.querySelector('.executionPostureCard[data-label="Runner"]')
    expect(feedCard?.getAttribute('data-tone')).toBe('warn')
    expect(feedCard?.textContent || '').toContain('Disconnected · AAPL [delayed]')
    expect(runnerCard?.getAttribute('data-tone')).toBe('warn')
    expect(runnerCard?.textContent || '').toContain('Stopped · review required')
  })

  it('renders broker order rows through the shared headless table path', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.orderRows = [
      {
        cells: [
          '2026-03-28 14:55:00',
          'BUY',
          'filled',
          '10',
          '10',
          '182.40',
          'order-1',
          'stop 178.5',
          'stop_loss · stop_loss_first',
          'parent-1',
        ],
      },
    ]

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const tableText = document.querySelector('.execOrdersTableVue')?.textContent || ''
    expect(tableText).toContain('Time (UTC)')
    expect(tableText).toContain('Trigger')
    expect(tableText).toContain('Bracket')
    expect(tableText).toContain('BUY')
    expect(tableText).toContain('order-1')
    expect(tableText).toContain('stop_loss')
  })

  it('renders confirmed review metadata and raw broker output together', async () => {
    const { pinia, workflowStore, executionStore, dataStore } = primeExecutionPanelStores()
    const freshNow = Date.now()
    workflowStore.selectWorkspace('live')
    dataStore.dataId = 'AAPL'
    dataStore.datasetLabel = 'AAPL'
    executionStore.symbol = 'AAPL'
    executionChartServiceMock.syncExecutionChart.mockResolvedValue({
      state: 'loaded_broker_markers',
      summary: 'Execution chart loaded for AAPL with 1 broker-confirmed order marker.',
      symbol: 'AAPL',
      datasetId: 'AAPL',
      datasetLabel: 'AAPL',
      datasetSymbol: 'AAPL',
      fetchedAtUnixMs: freshNow,
      markerCount: 1,
    })
    executionStore.accountReview = {
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
    executionStore.brokerReview = {
      sourceCommand: 'get_live_execution_snapshot',
      mode: 'paper',
      symbol: 'AAPL',
      fetchedAtUnixMs: freshNow,
      staleThresholdMs: 90_000,
      account: executionStore.accountReview,
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
    }
    executionStore.outputPreviewMeta = {
      sourceCommand: 'get_live_execution_snapshot',
      scopeLabel: 'symbol-scoped AAPL',
      generatedAtUnixMs: freshNow,
    }
    executionStore.reconciliationState = 'broker_confirmed_flat'
    executionStore.reconciliationFetchedAtUnixMs = freshNow
    executionStore.reconciliationOpenOrderCount = 0
    executionStore.reconciliationOptimisticOrderCount = 0
    executionStore.orderRows = [
      {
        cells: [
          '2026-04-10 14:55:00',
          'BUY',
          'filled',
          '10',
          '10',
          '182.40',
          'ord_123',
        ],
      },
    ]
    executionStore.positionMeta = 'Position: flat'
    executionStore.chartMeta = 'Execution chart loaded for AAPL with 1 broker-confirmed order marker.'
    executionStore.accountMeta = 'Account review: paper · account=ABC123 · status=ACTIVE'
    executionStore.reviewMeta = 'Broker review: paper symbol-scoped AAPL · as of 2026-04-10 14:55:00 UTC · status=ACTIVE · 0 open orders'
    executionStore.outputPreview = '{"orders":[{"id":"ord_123"}]}'

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Account review: paper · account=ABC123 · status=ACTIVE')
    expect(text).toContain('Broker review: paper symbol-scoped AAPL')
    expect(text).toContain('Position: flat')
    expect(text).toContain('Execution chart unavailable: load a workflow dataset first.')
    expect(text).toContain('Broker-confirmed flat.')
    expect(text).toContain('symbol-scoped AAPL')
    expect(text).toContain('Source: get_live_execution_snapshot')
    expect(text).toContain('"ord_123"')
  })

  it('prefers broker review chart truth when local chart sync downgrades to no symbol', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.symbol = 'AAPL'
    const freshNow = Date.now()
    executionStore.brokerReview = {
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
        accountNumber: 'ABC123',
        accountId: '',
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
        state: 'no_dataset',
        symbol: 'AAPL',
        datasetId: '',
        datasetLabel: '',
        datasetSymbol: '',
        orderCount: 1,
        sourceCommand: 'get_live_execution_snapshot',
        fetchedAtUnixMs: freshNow,
      },
    }
    executionStore.chartMeta = 'Execution chart unavailable: load a workflow dataset first.'
    executionStore.orderRows = [
      {
        cells: [
          '2026-04-10 14:55:00',
          'BUY',
          'filled',
          '10',
          '10',
          '182.40',
          'ord_123',
        ],
      },
    ]
    executionChartServiceMock.syncExecutionChart.mockResolvedValueOnce({
      state: 'no_symbol',
      summary: 'Execution chart unavailable: select a symbol first.',
      symbol: '',
      datasetId: '',
      datasetLabel: '-',
      datasetSymbol: '',
      fetchedAtUnixMs: freshNow,
      markerCount: 0,
    })

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Execution chart unavailable: load a workflow dataset first.')
    expect(text).not.toContain('Execution chart unavailable: select a symbol first.')
  })

  it('shows stored secure credential status without hydrating API inputs', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.storedHasApiKey = true
    executionStore.storedHasApiSecret = true
    executionStore.storedHasCredentials = true
    executionStore.storedCredentialState = 'stored'
    executionStore.storedRememberAvailable = true
    executionStore.rememberKeys = true

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const apiKeyInput = document.querySelector('#vueExecApiKey') as HTMLInputElement | null
    const apiSecretInput = document.querySelector('#vueExecApiSecret') as HTMLInputElement | null
    expect(apiKeyInput?.type).toBe('password')
    expect(apiSecretInput?.type).toBe('password')
    expect(apiKeyInput?.value).toBe('')
    expect(apiSecretInput?.value).toBe('')
    expect(document.body.textContent || '').toContain('Stored securely')
  })

  it('disables remember mode when secure storage is unavailable', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.storedRememberAvailable = false
    executionStore.storedCredentialState = 'unavailable'
    executionStore.storedCredentialMessage = 'Secure storage unavailable.'

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const rememberInput = Array.from(document.querySelectorAll('label'))
      .find((label) => String(label.textContent || '').includes('Remember keys'))
      ?.querySelector('input[type="checkbox"]') as HTMLInputElement | null
    expect(rememberInput?.disabled).toBe(true)
    expect(document.body.textContent || '').toContain('Remember unavailable')
  })

  it('renders clear-failed secure credential state without hydrating API inputs', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.storedCredentialClearFailed = true
    executionStore.storedCredentialMessage = 'Keyring clear failed'

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const apiKeyInput = document.querySelector('#vueExecApiKey') as HTMLInputElement | null
    const apiSecretInput = document.querySelector('#vueExecApiSecret') as HTMLInputElement | null
    expect(apiKeyInput?.value).toBe('')
    expect(apiSecretInput?.value).toBe('')
    const text = document.body.textContent || ''
    expect(text).toContain('Clear failed')
    expect(text).toContain('Keyring clear failed')
  })

  it('runs dry-run one-shot directly with dynamic simulate copy', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.setInputField('execSymbol', 'AAPL')
    executionStore.setCheckboxField('execDryRun', true)
    const runOnceSpy = vi.spyOn(executionStore, 'runOnce').mockImplementation(() => {})

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const simulateButton = findButton('Simulate once')
    expect(simulateButton).toBeTruthy()
    simulateButton?.click()
    await nextTick()

    expect(runOnceSpy).toHaveBeenCalledTimes(1)
    expect(document.body.textContent || '').not.toContain('Confirm submit')
  })

  it('requires confirmation before broker-armed one-shot execution', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.setInputField('execSymbol', 'AAPL')
    executionStore.setCheckboxField('execDryRun', false)
    executionStore.setInputField('execApiKey', 'key_1')
    executionStore.setInputField('execApiSecret', 'secret_1')
    const runOnceSpy = vi.spyOn(executionStore, 'runOnce').mockImplementation(() => {})

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const brokerButton = findButton('Submit paper order once')
    expect(brokerButton).toBeTruthy()
    brokerButton?.click()
    await nextTick()

    expect(runOnceSpy).not.toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Confirm submit paper order once?')
    expect(document.body.textContent || '').toContain('Broker review: Broker refresh required.')

    const confirmButton = document.querySelector(
      '.confirmationDialogActions button:last-child',
    ) as HTMLButtonElement | null
    expect(confirmButton).toBeTruthy()
    confirmButton?.click()
    await nextTick()

    expect(runOnceSpy).toHaveBeenCalledTimes(1)
  })

  it('surfaces live-mode safety copy and the live submission label when broker submission is armed', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.setInputField('execSymbol', 'AAPL')
    executionStore.setSelectField('execMode', 'live')
    executionStore.setCheckboxField('execDryRun', false)
    executionStore.setInputField('execApiKey', 'key_1')
    executionStore.setInputField('execApiSecret', 'secret_1')

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Broker submission is armed in live mode.')
    expect(text).toContain('Submit live order once')
  })

  it('requires confirmation before starting the live feed', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.setInputField('execSymbol', 'AAPL')
    const startLiveSpy = vi.spyOn(executionStore, 'startLive').mockImplementation(() => {})

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const startLiveButton = findButton('Start live')
    expect(startLiveButton).toBeTruthy()
    startLiveButton?.click()
    await nextTick()

    expect(startLiveSpy).not.toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Start live candle feed?')

    const confirmButton = document.querySelector(
      '.confirmationDialogActions button:last-child',
    ) as HTMLButtonElement | null
    expect(confirmButton).toBeTruthy()
    confirmButton?.click()
    await nextTick()

    expect(startLiveSpy).toHaveBeenCalledTimes(1)
  })

  it('requires confirmation before starting the auto-runner', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.setInputField('execSymbol', 'AAPL')
    executionStore.applyPolygonLiveStatusEvent({
      active: true,
      symbol: 'AAPL',
      delayed: true,
      hint: 'Live candles active for AAPL [delayed].',
      status: 'started',
      message: 'live started',
    })
    const startRunnerSpy = vi.spyOn(executionStore, 'startRunner').mockImplementation(() => {})

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const startRunnerButton = findButton('Start runner')
    expect(startRunnerButton).toBeTruthy()
    startRunnerButton?.click()
    await nextTick()

    expect(startRunnerSpy).not.toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Start the auto-runner?')
    expect(document.body.textContent || '').toContain('Recovery: No recovery review pending.')
    expect(document.body.textContent || '').toContain('Broker review: Broker refresh required.')

    const confirmButton = document.querySelector(
      '.confirmationDialogActions button:last-child',
    ) as HTMLButtonElement | null
    expect(confirmButton).toBeTruthy()
    confirmButton?.click()
    await nextTick()

    expect(startRunnerSpy).toHaveBeenCalledTimes(1)
  })

  it('renders the live safety matrix and requires confirmation before broker safety controls fire', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.setInputField('execSymbol', 'AAPL')
    executionStore.setInputField('execApiKey', 'key_1')
    executionStore.setInputField('execApiSecret', 'secret_1')
    executionStore.applyPolygonLiveStatusEvent({
      active: true,
      symbol: 'AAPL',
      delayed: true,
      hint: 'Live candles active for AAPL [delayed].',
      status: 'started',
      message: 'live started',
    })
    const cancelSpy = vi.spyOn(executionStore, 'cancelOpenOrders').mockImplementation(() => {})
    const flattenSpy = vi.spyOn(executionStore, 'flattenPosition').mockImplementation(() => {})
    const haltSpy = vi.spyOn(executionStore, 'emergencyHalt').mockImplementation(() => {})

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Live execution safety control matrix')
    expect(text).toContain('Stop runner')
    expect(text).toContain('Cancel open orders')
    expect(text).toContain('Flatten position')
    expect(text).toContain('Emergency halt')

    findButton('Cancel open orders')?.click()
    await nextTick()
    expect(cancelSpy).not.toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Cancel current-symbol open orders?')
    expect(document.body.textContent || '').toContain('does not flatten any existing broker position')
    expect(document.body.textContent || '').toContain('Broker review: Broker refresh required.')
    ;(document.querySelector('.confirmationDialogActions button:last-child') as HTMLButtonElement | null)?.click()
    await nextTick()
    expect(cancelSpy).toHaveBeenCalledTimes(1)

    findButton('Flatten position')?.click()
    await nextTick()
    expect(flattenSpy).not.toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Flatten the current broker position?')
    expect(document.body.textContent || '').toContain('does not cancel open orders or stop the live feed')
    ;(document.querySelector('.confirmationDialogActions button:last-child') as HTMLButtonElement | null)?.click()
    await nextTick()
    expect(flattenSpy).toHaveBeenCalledTimes(1)

    findButton('Emergency halt')?.click()
    await nextTick()
    expect(haltSpy).not.toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Emergency halt live automation?')
    expect(document.body.textContent || '').toContain('does not cancel broker orders or flatten positions')
    ;(document.querySelector('.confirmationDialogActions button:last-child') as HTMLButtonElement | null)?.click()
    await nextTick()
    expect(haltSpy).toHaveBeenCalledTimes(1)
  })

  it('requires confirmation before clearing credentials', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.setInputField('execApiKey', 'key_1')
    executionStore.setInputField('execApiSecret', 'secret_1')
    const clearKeysSpy = vi.spyOn(executionStore, 'clearKeys').mockImplementation(() => {})

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const clearKeysButton = findButton('Clear keys')
    expect(clearKeysButton).toBeTruthy()
    clearKeysButton?.click()
    await nextTick()

    expect(clearKeysSpy).not.toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Clear stored execution credentials?')
    expect(document.body.textContent || '').toContain('clears stored secure execution credentials')

    const confirmButton = document.querySelector(
      '.confirmationDialogActions button:last-child',
    ) as HTMLButtonElement | null
    expect(confirmButton).toBeTruthy()
    confirmButton?.click()
    await nextTick()

    expect(clearKeysSpy).toHaveBeenCalledTimes(1)
  })

  it('shows broker-armed blocked reasons when credentials are missing', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.setInputField('execSymbol', 'AAPL')
    executionStore.setCheckboxField('execDryRun', false)

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Credential state')
    expect(text).toContain('Missing')
    expect(text).toContain('API key + secret are required for broker-armed execution.')

    const brokerButton = findButton('Submit paper order once')
    expect(brokerButton).toBeTruthy()
    expect(brokerButton?.disabled).toBe(true)
  })

  it('disables broker order review until a symbol is selected', async () => {
    const { pinia, workflowStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const refreshOrdersButton = findButton('Refresh orders')
    expect(refreshOrdersButton?.disabled).toBe(true)
    expect(document.body.textContent || '').toContain('Enter a symbol before broker order review.')
  })

  it('surfaces feed and runner preflight reasons before automation is available', async () => {
    const { pinia, workflowStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Feed: Enter a symbol before starting live candles.')
    expect(text).toContain('Runner: Enter a symbol before starting the auto-runner.')
    expect(text).toContain('Runner: Start live candles first. The auto-runner only reacts to active live bars.')
  })

  it('surfaces broker reconciliation advisories inside the broker review card', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.setInputField('execSymbol', 'AAPL')
    executionStore.setCheckboxField('execDryRun', true)
    executionStore.orderRows = [
      {
        cells: ['2026-01-01 12:00:00', 'buy', 'submitted*', '1', '-', '-', 'ord_001'],
      },
    ]
    executionStore.orderCount = 1

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()
    await Promise.resolve()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Broker review is waiting on 1 optimistic local order. Refresh orders before the next broker-armed action.')
    expect(text).toContain('Advisory')
  })

  it('keeps recovery, local paper, and saved-session controls accessible', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.setInputField('execSymbol', 'AAPL')
    executionStore.setInputField('execExecutionTarget', 'local_paper')
    executionStore.recoveryReviewRequired = true
    executionStore.recoveryCause = 'Policy block: max_order_notional.'
    executionStore.accountReview = {
      state: 'broker_confirmed',
      mode: 'paper',
      sourceCommand: 'get_local_paper_execution_snapshot',
      fetchedAtUnixMs: 1_900_000_000_000,
      accountId: 'local-paper',
      accountNumber: 'local-paper',
      status: 'simulated',
      currency: 'USD',
      buyingPower: '100000.00',
      cash: '100000.00',
      portfolioValue: '100000.00',
    }
    executionStore.orderRows = [
      {
        cells: ['2026-04-12 15:40:00', 'buy', 'submitted*', '2', '0', '-', 'local_order_1'],
      },
    ]
    executionStore.orderCount = 1
    executionStore.savedSessions = [
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
    ]
    executionStore.selectedSavedSessionPath = 'C:/runs/vectorgrid_live_session_latest.json'
    executionStore.savedSessionStatusHint = 'Loaded 1 saved live session artifact.'
    executionStore.savedSessionDetail = {
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
      sourceRunRouteSummary: 'requested=GpuOnly -> resolved=GpuKernel',
      eventCount: 2,
      brokerSnapshotCount: 1,
      events: [],
      brokerSnapshots: [],
    }

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    expect(document.querySelector('[aria-label="Execution runtime status"]')).toBeTruthy()
    expect(document.querySelector('[aria-label="Recovery checklist"]')).toBeTruthy()
    expect(document.querySelector('[aria-label="Local paper account"]')).toBeTruthy()
    expect(document.querySelector('[aria-label="Saved live session evidence selection"]')).toBeTruthy()
    expect(document.querySelector('table caption')?.textContent || '').toContain('Broker order activity for the current execution symbol')
    expect(document.querySelectorAll('[role="status"][aria-live="polite"]').length).toBeGreaterThan(0)

    const text = document.body.textContent || ''
    expect(text).toContain('Policy block: max_order_notional.')
    expect(text).toContain('Recovery cleared')
    expect(text).toContain('Current comparison')
    expect(text).toContain('local-paper')

    const resetButton = findButton('Reset local paper')
    expect(resetButton).toBeTruthy()
    expect(resetButton?.disabled).toBe(false)
    resetButton?.click()
    await nextTick()

    const dialog = document.querySelector('[role="alertdialog"]')
    expect(dialog).toBeTruthy()
    expect(dialog?.getAttribute('aria-modal')).toBe('true')
    expect(dialog?.getAttribute('aria-labelledby')).toBe('execution_confirmation_title')
    expect(dialog?.getAttribute('aria-describedby')).toBe('execution_confirmation_body')
    expect(document.body.textContent || '').toContain('Reset local paper broker?')
  })

  it('renders saved session evidence as a read-only review surface', async () => {
    const { pinia, workflowStore, executionStore } = primeExecutionPanelStores()
    workflowStore.selectWorkspace('live')
    executionStore.savedSessions = [
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
    ]
    executionStore.selectedSavedSessionPath = 'C:/runs/vectorgrid_live_session_latest.json'
    executionStore.savedSessionStatusHint = 'Loaded 1 saved live session artifact.'
    executionStore.savedSessionDetail = {
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
      sourceRunRouteSummary: 'requested=GpuOnly -> resolved=GpuKernel -> realized=CpuOnly (fallback: cuda unavailable)',
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
        {
          provenance: 'submission_attempt',
          status: 'order_submitted',
          message: 'submitted buy order',
          symbol: 'AAPL',
          atUnixMs: 1_900_000_000_250,
          barOpenUnixMs: null,
          action: 'buy',
          orderId: 'ord_123',
          deltaQty: 2,
          orderSubmitted: true,
          warnings: ['paper only'],
        },
      ],
      brokerSnapshots: [
        {
          sourceCommand: 'get_live_execution_snapshot',
          mode: 'paper',
          symbol: 'AAPL',
          atUnixMs: 1_900_000_000_300,
          accountStatus: 'ACTIVE',
          currency: 'USD',
          positionState: 'flat',
          positionQty: 0,
          positionSide: '',
          openOrderCount: 0,
          orderCount: 1,
        },
      ],
    }

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Saved session evidence')
    expect(text).toContain('Loaded 1 saved live session artifact.')
    expect(text).toContain('sess_latest')
    const sourceRunPath = 'C:/runs/vectorgrid_run_2026-04-12.json'
    const sourceRunEvidence = document.querySelector(
      '.compactEvidenceText[aria-label^="Source run:"]',
    )
    expect(sourceRunEvidence?.textContent).toBe('vectorgrid_run_2026-04-12.json')
    expect(sourceRunEvidence?.getAttribute('title')).toBe(sourceRunPath)
    expect(sourceRunEvidence?.getAttribute('aria-label')).toBe(
      `Source run: ${sourceRunPath}`,
    )
    expect(text).toContain('order_submitted')
    expect(text).toContain('submitted buy order')
    expect(text).toContain('get_live_execution_snapshot')
  })
})
