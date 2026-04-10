// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { computed, nextTick, ref } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ExecutionWorkspacePanel from './ExecutionWorkspacePanel.vue'
import { useExecutionStore } from '../stores/executionStore'
import { useWorkflowStore } from '../stores/workflowStore'

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

  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = `
      <div id="settingsShellGrid">
        <div id="executionShellPane">
          <div id="executionVueShellMount"></div>
        </div>
      </div>
    `
  })

  it('renders the empty broker-order state when no orders exist', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('settings')

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Broker order activity')
    expect(text).toContain('No broker orders for current symbol.')
  })

  it('renders unknown review metadata distinctly before broker refresh confirms flat state', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('settings')

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Position: -')
    expect(text).not.toContain('Position: flat')
    expect(text).toContain('Load a dataset and refresh orders to overlay broker events.')
  })

  it('renders broker order rows through the shared headless table path', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('settings')
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
    expect(tableText).toContain('BUY')
    expect(tableText).toContain('order-1')
  })

  it('renders confirmed review metadata and raw broker output together', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('settings')
    executionStore.positionMeta = 'Position: flat'
    executionStore.chartMeta = 'Execution chart sync pending. symbol=AAPL orders=1'
    executionStore.outputPreview = '{"orders":[{"id":"ord_123"}]}'

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Position: flat')
    expect(text).toContain('Execution chart sync pending. symbol=AAPL orders=1')
    expect(text).toContain('"ord_123"')
  })

  it('runs dry-run one-shot directly with dynamic simulate copy', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('settings')
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
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('settings')
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

    const confirmButton = document.querySelector(
      '.confirmationDialogActions button:last-child',
    ) as HTMLButtonElement | null
    expect(confirmButton).toBeTruthy()
    confirmButton?.click()
    await nextTick()

    expect(runOnceSpy).toHaveBeenCalledTimes(1)
  })

  it('surfaces live-mode safety copy and the live submission label when broker submission is armed', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('settings')
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
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('settings')
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
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('settings')
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

    const confirmButton = document.querySelector(
      '.confirmationDialogActions button:last-child',
    ) as HTMLButtonElement | null
    expect(confirmButton).toBeTruthy()
    confirmButton?.click()
    await nextTick()

    expect(startRunnerSpy).toHaveBeenCalledTimes(1)
  })

  it('requires confirmation before clearing credentials', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('settings')
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

    const confirmButton = document.querySelector(
      '.confirmationDialogActions button:last-child',
    ) as HTMLButtonElement | null
    expect(confirmButton).toBeTruthy()
    confirmButton?.click()
    await nextTick()

    expect(clearKeysSpy).toHaveBeenCalledTimes(1)
  })

  it('shows broker-armed blocked reasons when credentials are missing', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('settings')
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

  it('surfaces feed and runner preflight reasons before automation is available', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('settings')

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
})
