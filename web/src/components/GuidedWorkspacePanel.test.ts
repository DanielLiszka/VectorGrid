// @vitest-environment jsdom
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

describe('GuidedWorkspacePanel', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = `
      <section id="guidedShellGrid">
        <div id="guidedVueShellMount"></div>
      </section>
    `
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
    expect(text).toContain('Optimize, validate, then dry-run.')
    expect(text).toContain('Advanced Mode keeps every research')
    expect(text).toContain('Next')
    expect(text).toContain('Review trade readiness')
    expect(text).toContain('Rank 1 · Sharpe 1.42')
    expect(text).toContain('AAPL sample · 15m')
    expect(text).toContain('Validation run')

    document.querySelector<HTMLButtonElement>('button.guidedSecondary')?.click()
    await nextTick()
    expect(settingsStore.uiMode).toBe('advanced')

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

  it('does not start a broker-armed runner from guided mode', async () => {
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
    expect(startButton).toBeTruthy()
    expect(startButton?.disabled).toBe(true)
    startButton?.click()
    expect(startRunnerSpy).not.toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Guided Mode will not start the auto-runner')

    wrapper.unmount()
  })
})
