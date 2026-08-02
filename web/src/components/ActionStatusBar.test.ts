
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ActionStatusBar from './ActionStatusBar.vue'
import { TASK_CENTER_PANEL_ID } from '../composables/useTaskCenterVisibility'
import { useAnalysisStore } from '../stores/analysisStore'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useDataStore } from '../stores/dataStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useTaskStore } from '../stores/taskStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'

function installFooterMountShell(): void {
  document.body.innerHTML = `
    <footer id="runFooterShell">
      <div id="runFooterVueShellMount"></div>
    </footer>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

function findButton(label: string): HTMLButtonElement | undefined {
  return Array.from(document.querySelectorAll('button')).find(
    (button) => button.textContent?.trim() === label,
  ) as HTMLButtonElement | undefined
}

describe('ActionStatusBar', () => {
  beforeEach(() => {
    installFooterMountShell()
    localStorage.clear()
  })

  it('uses Run optimization as the primary action on the optimize workflow step', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const data = useDataStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const actions = useCommandActionStore(pinia)
    workflow.selectWorkflowStep('optimize', { persist: false })
    data.dataId = 'data-1'
    data.datasetLabel = 'AAPL'
    optimize.runDisabled = false
    const runSpy = vi.spyOn(actions, 'runOptimization').mockImplementation(() => {})

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const primary = findButton('Run optimization')
    expect(primary).toBeDefined()
    expect(primary?.disabled).toBe(false)
    primary?.click()
    expect(runSpy).toHaveBeenCalledTimes(1)

    wrapper.unmount()
  })

  it('uses validation as the contextual primary action on the validation step', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const dataStore = useDataStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const artifacts = useWorkflowArtifactsStore(pinia)
    const actions = useCommandActionStore(pinia)
    workflow.selectWorkflowStep('validate', { persist: false })
    dataStore.dataId = 'AAPL_1D'
    dataStore.datasetLabel = 'AAPL'
    artifacts.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-03-09T15:00:00Z',
        req: { data_id: 'AAPL_1D', strategy_kind: 'DoubleMa' },
        res: { top: [] },
      },
    })
    optimize.validateDisabled = false
    const validateSpy = vi.spyOn(actions, 'validateOptimization').mockImplementation(() => {})

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const primary = findButton('Run validation')
    expect(primary).toBeDefined()
    expect(primary?.disabled).toBe(false)
    primary?.click()
    expect(validateSpy).toHaveBeenCalledTimes(1)
    expect(document.body.textContent || '').toContain('Execution readiness')

    wrapper.unmount()
  })

  it('blocks validation and retained candidate copy after loading a different dataset path', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const dataStore = useDataStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const artifacts = useWorkflowArtifactsStore(pinia)
    workflow.selectWorkflowStep('validate', { persist: false })
    dataStore.dataId = 'data-1'
    dataStore.datasetLabel = 'vectorgrid_synthetic_ohlcv.csv'
    dataStore.baseCsvPath = 'C:/samples/vectorgrid_synthetic_ohlcv.csv'
    analysisStore.topRows = 20
    analysisStore.selectedTopTitle = 'Retained AAPL winner'
    optimize.validateDisabled = false
    artifacts.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-07-18T12:00:00Z',
        req: { data_id: 'data-1', strategy_kind: 'DoubleMa' },
        dataset_meta: { csv_path: 'C:/market/AAPL.csv' },
        res: { top: [{}] },
      },
    })

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const primary = findButton('Run validation')
    expect(primary?.disabled).toBe(true)
    const text = document.body.textContent || ''
    expect(text).toContain('Ready to configure and run optimization.')
    expect(text).not.toContain('Retained AAPL winner')

    wrapper.unmount()
  })

  it('shows a blocked reason when the primary action is unavailable', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const data = useDataStore(pinia)
    const optimize = useOptimizeStore(pinia)
    workflow.selectWorkflowStep('optimize', { persist: false })
    data.dataId = 'data-1'
    data.datasetLabel = 'AAPL'
    optimize.$patch({
      runDisabled: true,
      runReadinessHint: 'Readiness: Load data first.',
    })

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const primary = findButton('Run optimization')
    expect(primary).toBeDefined()
    expect(primary?.disabled).toBe(true)
    expect(primary?.getAttribute('title')).toBe('Readiness: Load data first.')
    const reasonId = primary?.getAttribute('aria-describedby')
    expect(reasonId).toBe('footerPrimaryDisabledReason')
    expect(document.getElementById(reasonId || '')?.textContent).toContain(
      'Readiness: Load data first.',
    )

    wrapper.unmount()
  })

  it('blocks optimization when no dataset is loaded even if the raw run flag is permissive', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    workflow.selectWorkflowStep('optimize', { persist: false })
    optimize.$patch({
      runDisabled: false,
      runReadinessHint: 'Readiness: Load data first.',
    })

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const primary = findButton('Run optimization')
    expect(primary).toBeDefined()
    expect(primary?.disabled).toBe(true)
    expect(primary?.getAttribute('title')).toBe('Load a dataset before running optimization.')

    wrapper.unmount()
  })

  it('keeps the MlSignal primary action aligned with the current workflow step', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const data = useDataStore(pinia)
    workflow.selectMode('ml-signal', { persist: false })
    data.dataId = 'data-1'
    optimize.$patch({
      runDisabled: false,
      validateDisabled: false,
      mlSignalDatasetExportPath: 'C:\\data\\ml_dataset.csv',
      mlSignalModelBundleId: 'bundle-a',
      mlSignalScoreInputPath: '',
      mlSignalScoreCachePath: 'C:\\data\\score_cache.json',
      mlSignalScoreSource: 'native',
    })

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    workflow.selectWorkflowStep('data', { persist: false })
    await flushPanel()
    expect(findButton('Export dataset')).toBeDefined()

    workflow.selectWorkflowStep('optimize', { persist: false })
    await flushPanel()
    const scoreCachePrimary = findButton('Build score cache')
    expect(scoreCachePrimary).toBeDefined()
    expect(scoreCachePrimary?.disabled).toBe(false)

    workflow.selectWorkflowStep('validate', { persist: false })
    await flushPanel()
    expect(findButton('Run ML validation')).toBeDefined()

    workflow.selectWorkflowStep('analyze', { persist: false })
    await flushPanel()
    expect(findButton('Run ML policy sweep')).toBeDefined()

    workflow.selectWorkflowStep('finalize', { persist: false })
    await flushPanel()
    const finalizePrimary = findButton('Artifacts ready')
    expect(finalizePrimary).toBeDefined()
    expect(finalizePrimary?.disabled).toBe(true)

    wrapper.unmount()
  })

  it('routes the single running primary action through cancellation', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const actions = useCommandActionStore(pinia)
    workflow.selectWorkflowStep('optimize', { persist: false })
    optimize.$patch({ canCancel: true, runDisabled: false, progressMeta: 'Optimize 84/100' })
    const cancelSpy = vi.spyOn(actions, 'cancelOptimization').mockImplementation(() => {})

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const primary = findButton('Stop run')
    expect(primary).toBeDefined()
    expect(primary?.disabled).toBe(false)

    primary?.click()
    expect(cancelSpy).toHaveBeenCalledTimes(1)
    expect(document.body.textContent || '').toContain('Optimize 84/100')

    wrapper.unmount()
  })

  it('opens task center from Details instead of expanding a footer details grid', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    useWorkflowStore(pinia).selectWorkflowStep('optimize', { persist: false })

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const details = findButton('Details')
    expect(details).toBeDefined()
    expect(details?.getAttribute('aria-controls')).toBe(TASK_CENTER_PANEL_ID)
    expect(document.querySelector('.summaryGrid')).toBeNull()

    details?.click()
    await flushPanel()

    expect(details?.getAttribute('aria-expanded')).toBe('true')

    wrapper.unmount()
  })

  it('does not render outside the workflow workspace', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    workflow.selectWorkspace('settings')

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.body.textContent || '').not.toContain('Execution readiness')
    expect(findButton('Run optimization')).toBeUndefined()

    wrapper.unmount()
  })

  it('surfaces grouped task posture in the compact task summary', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const taskStore = useTaskStore(pinia)
    workflow.selectWorkflowStep('optimize', { persist: false })
    optimize.progressMeta = 'Optimize 42/100'
    taskStore.$patch({
      activeById: {
        task_running: {
          id: 'task_running',
          command: 'run_double_ma_optimization',
          label: 'Optimization',
          status: 'running',
          startedAt: 1_700_000_000_000,
          message: 'Evaluate (42.0%)',
        },
        task_queued: {
          id: 'task_queued',
          command: 'run_double_ma_validation',
          label: 'Validation',
          status: 'queued',
          startedAt: 1_700_000_000_100,
          message: 'Queued.',
        },
      },
      activeOrder: ['task_running', 'task_queued'],
      history: [
        {
          id: 'task_failed',
          command: 'run_double_ma_validation',
          label: 'Validation',
          status: 'failed',
          startedAt: 1_700_000_000_000,
          endedAt: 1_700_000_001_000,
          error: 'Validation failed.',
        },
      ],
    })

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('1 running')
    expect(text).toContain('1 queued')
    expect(text).toContain('1 failed')
    expect(text).toContain('Attention')

    wrapper.unmount()
  })

  it('keeps the selected post-run candidate visible in the footer summary', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const data = useDataStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflow.selectWorkflowStep('analyze', { persist: false })
    workflow.selectResultsTab('top', { persist: false })
    data.dataId = 'data-1'
    analysisStore.topRows = 4
    analysisStore.selectedTopTitle = 'Rank #2 · SMA 12 / EMA 48'
    analysisStore.selectedTopSummary = 'Selected candidate 2 of 4: Sharpe 1.7.'

    const wrapper = mount(ActionStatusBar, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('Rank #2 · SMA 12 / EMA 48')
    expect(text).toContain('Selected candidate 2 of 4: Sharpe 1.7.')

    wrapper.unmount()
  })
})
