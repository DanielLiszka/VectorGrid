
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import GuidedWorkspacePanel from './GuidedWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useExecutionStore } from '../stores/executionStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'

const clipboardMocks = vi.hoisted(() => ({
  copyTextToDesktopClipboard: vi.fn(async () => ({ ok: true, method: 'navigator-clipboard' as const })),
}))
const simpleSummaryMocks = vi.hoisted(() => ({
  saveSimpleSummaryMarkdown: vi.fn(async () => ({ path: 'C:/exports/simple-summary.md', bytes: 512 })),
  buildSimpleSummaryDefaultName: vi.fn(() => 'vectorgrid-simple-summary-test.md'),
}))

vi.mock('../services/desktopClipboard', () => ({
  copyTextToDesktopClipboard: clipboardMocks.copyTextToDesktopClipboard,
}))

vi.mock('../services/simpleSummaryService', () => ({
  saveSimpleSummaryMarkdown: simpleSummaryMocks.saveSimpleSummaryMarkdown,
  buildSimpleSummaryDefaultName: simpleSummaryMocks.buildSimpleSummaryDefaultName,
}))

describe('GuidedWorkspacePanel', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = `
      <section id="guidedShellGrid">
        <div id="guidedVueShellMount"></div>
      </section>
    `
    clipboardMocks.copyTextToDesktopClipboard.mockClear()
    clipboardMocks.copyTextToDesktopClipboard.mockResolvedValue({
      ok: true,
      method: 'navigator-clipboard',
    })
    simpleSummaryMocks.saveSimpleSummaryMarkdown.mockClear()
    simpleSummaryMocks.buildSimpleSummaryDefaultName.mockClear()
    simpleSummaryMocks.buildSimpleSummaryDefaultName.mockReturnValue('vectorgrid-simple-summary-test.md')
    simpleSummaryMocks.saveSimpleSummaryMarkdown.mockResolvedValue({
      path: 'C:/exports/simple-summary.md',
      bytes: 512,
    })
  })

  it('renders the focused guided path without removing advanced handoffs', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const optimizeStore = useOptimizeStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    const validationStore = useValidationStore(pinia)

    vi.spyOn(dataStore, 'hydrateStartupState').mockResolvedValue(undefined)
    settingsStore.setUiMode('guided')
    settingsStore.setGuidedStep('optimize')
    dataStore.$patch({
      dataId: 'dataset-1',
      datasetLabel: 'AAPL sample',
      dataLen: '1200',
      dataTimeframe: '15m',
      datasetRange: '15m · 2024-01-01 -> 2024-03-01',
    })
    optimizeStore.$patch({
      strategyKind: 'DoubleMa',
      objective: 'Sharpe',
      backend: 'Auto',
      runDisabled: false,
    })
    analysisStore.$patch({
      topRows: 12,
      selectedTopTitle: 'Rank 1 · Sharpe 1.42',
      selectedTopSummary: 'Return 12.4% · max drawdown 4.1%',
    })
    validationStore.$patch({
      tableVisible: true,
      tableRowCount: 8,
      validationAccelerationStatus: 'GPU validation path used for core checks.',
      validationShortlistMeta: '8 candidates admitted.',
      validationRouteSummary: 'Holdout validation complete.',
      validationWarningSummary: '-',
    })

    const wrapper = mount(GuidedWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Optimize backtests. Validate when needed.')
    expect(text).toContain('Simple view keeps the standard workflow focused')
    expect(text).toContain('Need the full workstation?')
    expect(text).toContain('Next')
    expect(text).toContain('Review results')
    expect(text).toContain('Run summary')
    expect(text).toContain('Copy summary')
    expect(text).toContain('Save summary')
    expect(text).toContain('Rank 1 · Sharpe 1.42')
    expect(text).toContain('AAPL sample · 15m')
    expect(text).toContain('Results ready.')
    expect(text).toContain('Validation optional')

    document.querySelector<HTMLButtonElement>('[data-qa-action="guided.advancedMode.open"]')?.click()
    await nextTick()
    expect(settingsStore.uiMode).toBe('advanced')

    wrapper.unmount()
  })

  it('shows a basic validation execution status in the Simple validation step', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const optimizeStore = useOptimizeStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    const validationStore = useValidationStore(pinia)
    vi.spyOn(dataStore, 'hydrateStartupState').mockResolvedValue(undefined)

    settingsStore.setUiMode('guided')
    settingsStore.setGuidedStep('validate')
    dataStore.$patch({
      dataId: 'dataset-1',
      datasetLabel: 'AAPL sample',
      dataTimeframe: '15m',
    })
    optimizeStore.$patch({
      strategyKind: 'DoubleMa',
      objective: 'Sharpe',
      backend: 'Auto',
      runDisabled: false,
    })
    analysisStore.$patch({
      topRows: 12,
      selectedTopTitle: 'Rank 1 · Sharpe 1.42',
    })
    validationStore.$patch({
      tableVisible: true,
      tableRowCount: 8,
      validationAccelerationStatus: 'GPU validation path used for core checks.',
      validationRouteSummary: 'Holdout validation complete.',
      validationWarningSummary: '-',
    })

    const wrapper = mount(GuidedWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Engine')
    expect(text).toContain('GPU validation path used for core checks.')

    wrapper.unmount()
  })

  it('copies a Markdown Simple run summary after optimization has results', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const optimizeStore = useOptimizeStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    vi.spyOn(dataStore, 'hydrateStartupState').mockResolvedValue(undefined)

    settingsStore.setUiMode('guided')
    settingsStore.setGuidedStep('optimize')
    dataStore.$patch({
      dataId: 'dataset-1',
      datasetLabel: 'AAPL sample',
      dataLen: '1200',
      dataTimeframe: '15m',
      datasetRange: '2024-01-01 -> 2024-03-01',
    })
    optimizeStore.$patch({
      strategyKind: 'DoubleMa',
      objective: 'Sharpe',
      backend: 'Auto',
      runDisabled: false,
    })
    analysisStore.$patch({
      topRows: 12,
      selectedTopTitle: 'Rank 1 · Sharpe 1.42',
      selectedTopSummary: 'Return 12.4% · max drawdown 4.1%',
      bestStats: [
        { label: 'Return', value: '12.4%', tone: 'success' },
        { label: 'Max DD', value: '4.1%', tone: 'warning' },
      ],
    })

    const wrapper = mount(GuidedWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    })
    await nextTick()
    await nextTick()

    const copyButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-panel="simple.optimize"] [data-qa-action="guided.summary.copy"]',
    )
    expect(copyButton).toBeTruthy()
    expect(copyButton?.disabled).toBe(false)
    copyButton?.click()
    await nextTick()
    await Promise.resolve()

    expect(clipboardMocks.copyTextToDesktopClipboard).toHaveBeenCalledTimes(1)
    const copyCalls = clipboardMocks.copyTextToDesktopClipboard.mock.calls as unknown[][]
    const copied = String(copyCalls[0]?.[0] || '')
    expect(copied).toContain('# VectorGrid Simple Run Summary')
    expect(copied).toContain('- Dataset: AAPL sample')
    expect(copied).toContain('- Best candidate: Rank 1 · Sharpe 1.42')
    expect(copied).toContain('- Return: 12.4%')
    expect(document.body.textContent || '').toContain('Copied Simple summary.')

    wrapper.unmount()
  })

  it('saves a Markdown Simple run summary after optimization has results', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const optimizeStore = useOptimizeStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    vi.spyOn(dataStore, 'hydrateStartupState').mockResolvedValue(undefined)

    settingsStore.setUiMode('guided')
    settingsStore.setGuidedStep('optimize')
    dataStore.$patch({
      dataId: 'dataset-1',
      datasetLabel: 'AAPL sample',
      dataLen: '1200',
      dataTimeframe: '15m',
      datasetRange: '2024-01-01 -> 2024-03-01',
    })
    optimizeStore.$patch({
      strategyKind: 'DoubleMa',
      objective: 'Sharpe',
      backend: 'Auto',
      runDisabled: false,
    })
    analysisStore.$patch({
      topRows: 12,
      selectedTopTitle: 'Rank 1 · Sharpe 1.42',
      selectedTopSummary: 'Return 12.4% · max drawdown 4.1%',
    })

    const wrapper = mount(GuidedWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    })
    await nextTick()
    await nextTick()

    const saveButton = document.querySelector<HTMLButtonElement>(
      '[data-qa-panel="simple.optimize"] [data-qa-action="guided.summary.save"]',
    )
    expect(saveButton).toBeTruthy()
    expect(saveButton?.disabled).toBe(false)
    saveButton?.click()
    await nextTick()
    await Promise.resolve()

    expect(simpleSummaryMocks.buildSimpleSummaryDefaultName).toHaveBeenCalledWith('AAPL sample')
    expect(simpleSummaryMocks.saveSimpleSummaryMarkdown).toHaveBeenCalledTimes(1)
    const saveCalls = simpleSummaryMocks.saveSimpleSummaryMarkdown.mock.calls as unknown[][]
    const saved = String(saveCalls[0]?.[0] || '')
    expect(saved).toContain('# VectorGrid Simple Run Summary')
    expect(saved).toContain('- Dataset: AAPL sample')
    expect(saveCalls[0]?.[1]).toBe('vectorgrid-simple-summary-test.md')
    expect(document.body.textContent || '').toContain('Saved Simple summary: C:/exports/simple-summary.md')

    wrapper.unmount()
  })

  it('promotes the next useful action from the current readiness state', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const dataStore = useDataStore(pinia)
    const optimizeStore = useOptimizeStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    vi.spyOn(dataStore, 'hydrateStartupState').mockResolvedValue(undefined)
    const runSpy = vi.spyOn(optimizeStore, 'runOptimization').mockImplementation(() => undefined)

    settingsStore.setUiMode('guided')
    dataStore.$patch({
      dataId: 'dataset-1',
      datasetLabel: 'AAPL sample',
      dataTimeframe: '15m',
    })
    optimizeStore.$patch({
      strategyKind: 'DoubleMa',
      runDisabled: false,
    })

    const wrapper = mount(GuidedWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    })
    await nextTick()
    await nextTick()

    const nextAction = document.querySelector<HTMLElement>('.guidedNextAction')
    const nextButton = nextAction?.querySelector<HTMLButtonElement>('button')

    expect(nextAction?.textContent || '').toContain('Run optimization')
    expect(nextButton?.disabled).toBe(false)
    nextButton?.click()
    await nextTick()

    expect(runSpy).toHaveBeenCalledTimes(1)

    wrapper.unmount()
  })

  it('promotes sample data as the first-run fast path when samples are available', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const dataStore = useDataStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    vi.spyOn(dataStore, 'hydrateStartupState').mockResolvedValue(undefined)
    const loadSampleSpy = vi.spyOn(dataStore, 'loadSampleCsv').mockImplementation(() => undefined)

    settingsStore.setUiMode('guided')
    dataStore.$patch({
      sampleCsvOptions: [
        {
          label: 'AAPL sample',
          path: 'C:/samples/aapl.csv',
          disabled: false,
          detail: 'Synthetic sample · 1m candles · 160 rows · Research only',
        },
      ],
    })

    const wrapper = mount(GuidedWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    })
    await nextTick()
    await nextTick()

    const nextAction = document.querySelector<HTMLElement>('.guidedNextAction')
    const nextButton = nextAction?.querySelector<HTMLButtonElement>('button')

    expect(nextAction?.textContent || '').toContain('Load sample data')
    expect(nextAction?.textContent || '').toContain('Synthetic sample · 1m candles')
    expect(document.body.textContent || '').toContain('Synthetic sample · 1m candles · 160 rows · Research only')
    expect(nextButton?.disabled).toBe(false)
    nextButton?.click()
    await nextTick()

    expect(settingsStore.guidedStep).toBe('data')
    expect(loadSampleSpy).toHaveBeenCalledWith(0, 'C:/samples/aapl.csv')

    wrapper.unmount()
  })

  it('uses templates to update existing optimization state', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const dataStore = useDataStore(pinia)
    const optimizeStore = useOptimizeStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    vi.spyOn(dataStore, 'hydrateStartupState').mockResolvedValue(undefined)
    settingsStore.setUiMode('guided')
    settingsStore.setGuidedStep('strategy')

    const wrapper = mount(GuidedWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    })
    await nextTick()
    await nextTick()

    const buttons = Array.from(document.querySelectorAll<HTMLButtonElement>('button'))
    const compositeButton = buttons.find((button) => button.textContent?.includes('Use Composite'))
    expect(compositeButton).toBeTruthy()
    compositeButton?.click()
    await nextTick()

    expect(optimizeStore.strategyKind).toBe('CompositeCpu')
    expect(optimizeStore.objective).toBe('Sharpe')
    expect(optimizeStore.backend).toBe('Auto')
    expect(settingsStore.guidedStep).toBe('optimize')
    expect(workflowStore.workflowStep).toBe('optimize')

    wrapper.unmount()
  })

  it('maps legacy trade-start state back to the validation handoff', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const dataStore = useDataStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    vi.spyOn(dataStore, 'hydrateStartupState').mockResolvedValue(undefined)
    settingsStore.setUiMode('guided')
    settingsStore.setGuidedStep('trade')

    const wrapper = mount(GuidedWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    })
    await nextTick()
    await nextTick()

    expect(document.body.textContent || '').toContain('Optional next step')
    expect(document.body.textContent || '').not.toContain('Start dry-run runner')

    document.querySelector<HTMLButtonElement>('[data-qa-action="guided.advancedMode.open"]')?.click()
    await nextTick()

    expect(settingsStore.uiMode).toBe('advanced')
    expect(workflowStore.workflowStep).toBe('validate')

    wrapper.unmount()
  })

  it('does not expose broker runner controls from the streamlined Simple path', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const dataStore = useDataStore(pinia)
    const executionStore = useExecutionStore(pinia)
    const optimizeStore = useOptimizeStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    vi.spyOn(dataStore, 'hydrateStartupState').mockResolvedValue(undefined)
    const startRunnerSpy = vi.spyOn(executionStore, 'startRunner')

    settingsStore.setUiMode('guided')
    settingsStore.setGuidedStep('trade')
    optimizeStore.$patch({ strategyKind: 'DoubleMa' })
    executionStore.$patch({
      dryRun: false,
      canStartRunner: true,
      symbol: 'AAPL',
      liveActive: true,
      liveSymbol: 'AAPL',
      storedHasApiKey: true,
      storedHasApiSecret: true,
      storedHasCredentials: true,
    })

    const wrapper = mount(GuidedWorkspacePanel, {
      attachTo: document.body,
      global: { plugins: [pinia] },
    })
    await nextTick()
    await nextTick()

    const startButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button'))
      .find((button) => button.textContent?.includes('Start dry-run runner'))
    const simulateButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button'))
      .find((button) => button.textContent?.includes('Simulate once'))
    expect(startButton).toBeFalsy()
    expect(simulateButton).toBeFalsy()
    expect(startRunnerSpy).not.toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Paper/live trading stays in Advanced Mode')

    wrapper.unmount()
  })
})
