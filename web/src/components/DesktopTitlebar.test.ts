import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DesktopTitlebar from './DesktopTitlebar.vue'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'
import { useDataStore } from '../stores/dataStore'
import { useExecutionStore } from '../stores/executionStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useOptionsExpressionStore } from '../stores/optionsExpressionStore'
import { useTaskStore } from '../stores/taskStore'
import { useWorkflowArtifactsStore } from '../stores/workflowArtifactsStore'
import { useWorkflowStore } from '../stores/workflowStore'

const minimizeMock = vi.fn(async () => undefined)
const maximizeMock = vi.fn(async () => undefined)
const unmaximizeMock = vi.fn(async () => undefined)
const outerSizeMock = vi.fn(async () => ({ width: 3840, height: 2160 }))
const currentMonitorMock = vi.fn(async () => ({ size: { width: 3840, height: 2160 } }))
const closeMock = vi.fn(async () => undefined)
const destroyMock = vi.fn(async () => undefined)
const startDraggingMock = vi.fn(async () => undefined)
const isMaximizedMock = vi.fn(async () => false)
const isFocusedMock = vi.fn(async () => true)
const onResizedMock = vi.fn(async () => () => {})
const onFocusChangedMock = vi.fn(async () => () => {})
const onCloseRequestedMock = vi.fn(async () => () => {})
const invokeCommandMock = vi.fn(async (_command: string, _args?: unknown) => undefined)

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    minimize: minimizeMock,
    maximize: maximizeMock,
    unmaximize: unmaximizeMock,
    outerSize: outerSizeMock,
    close: closeMock,
    destroy: destroyMock,
    startDragging: startDraggingMock,
    isMaximized: isMaximizedMock,
    isFocused: isFocusedMock,
    onResized: onResizedMock,
    onFocusChanged: onFocusChangedMock,
    onCloseRequested: onCloseRequestedMock,
  }),
  currentMonitor: () => currentMonitorMock(),
}))

vi.mock('../tauri/invoke', () => ({
  tauriRuntimeAvailable: () => true,
  invokeCommand: (command: string, args?: unknown) => invokeCommandMock(command, args),
}))

describe('DesktopTitlebar', () => {
  beforeEach(() => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    localStorage.clear()
    localStorage.setItem('vta_ui_mode_v1', 'advanced')

    minimizeMock.mockClear()
    maximizeMock.mockClear()
    unmaximizeMock.mockClear()
    outerSizeMock.mockReset().mockResolvedValue({ width: 3840, height: 2160 })
    currentMonitorMock.mockReset().mockResolvedValue({ size: { width: 3840, height: 2160 } })
    closeMock.mockClear()
    destroyMock.mockClear()
    startDraggingMock.mockClear()
    isMaximizedMock.mockReset().mockResolvedValue(false)
    isFocusedMock.mockReset().mockResolvedValue(true)
    onResizedMock.mockClear()
    onFocusChangedMock.mockClear()
    onCloseRequestedMock.mockClear()
    invokeCommandMock.mockClear()
  })

  it('surfaces current workspace and keeps the titlebar free of dataset and task chips', () => {
    const workflowStore = useWorkflowStore()

    workflowStore.selectWorkflowStep('optimize')

    const wrapper = mount(DesktopTitlebar)

    expect(wrapper.text()).toContain('VectorGrid')
    expect(wrapper.text()).toContain('Research · Strategy Setup')
    expect(wrapper.text()).not.toContain('Dataset')
    expect(wrapper.findAll('.titlebarTool')).toHaveLength(3)
  })

  it('associates disabled primary actions with an explicit blocking reason', () => {
    const optimizeStore = useOptimizeStore()
    optimizeStore.runDisabled = true

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.get('.titlebarPrimary')

    expect((primary.element as HTMLButtonElement).disabled).toBe(true)
    expect(primary.attributes('aria-describedby')).toBe('desktopPrimaryActionBlockedReason')
    expect(primary.attributes('title')).toContain('Load a dataset')
    expect(wrapper.get('#desktopPrimaryActionBlockedReason').text()).toContain('Load a dataset')
  })

  it('uses mode-aware labels and action text for MlSignal shell mode', () => {
    const workflowStore = useWorkflowStore()
    workflowStore.selectMode('ml-signal', { persist: false })
    workflowStore.selectWorkflowStep('validate', { persist: false })

    const wrapper = mount(DesktopTitlebar)

    expect(wrapper.text()).toContain('Research · Policy Validation')
    expect(wrapper.get('.titlebarPrimary').attributes('aria-label')).toBe('Run ML validation')
    expect(wrapper.get('.titlebarPrimary').text()).toContain('Validate')
  })

  it('keeps MlSignal shared primary actions truthful across workflow steps', async () => {
    const dataStore = useDataStore()
    const workflowStore = useWorkflowStore()
    const optimizeStore = useOptimizeStore()
    workflowStore.selectMode('ml-signal', { persist: false })
    dataStore.dataId = 'data-1'
    optimizeStore.$patch({
      mlSignalDatasetExportPath: 'C:/data/ml_dataset.csv',
      mlSignalModelBundleId: 'bundle-a',
      mlSignalScoreInputPath: '',
      mlSignalScoreCachePath: 'C:/data/ml_score_cache.json',
      mlSignalScoreSource: 'native',
      runDisabled: false,
      validateDisabled: false,
    })

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.get('.titlebarPrimary')

    workflowStore.selectWorkflowStep('data', { persist: false })
    await nextTick()
    expect(primary.attributes('aria-label')).toBe('Export dataset')

    workflowStore.selectWorkflowStep('optimize', { persist: false })
    await nextTick()
    expect(primary.attributes('aria-label')).toBe('Build score cache')
    expect((primary.element as HTMLButtonElement).disabled).toBe(false)

    workflowStore.selectWorkflowStep('analyze', { persist: false })
    await nextTick()
    expect(primary.attributes('aria-label')).toBe('Run ML policy sweep')

    workflowStore.selectWorkflowStep('finalize', { persist: false })
    await nextTick()
    expect(primary.attributes('aria-label')).toBe('Artifacts ready')
    expect((primary.element as HTMLButtonElement).disabled).toBe(true)
  })

  it('routes shell actions through the existing stores', async () => {
    const commandPaletteStore = useCommandPaletteStore()
    const commandActionStore = useCommandActionStore()
    const dataStore = useDataStore()
    const workflowStore = useWorkflowStore()
    dataStore.dataId = 'data-1'

    const openPaletteSpy = vi.spyOn(commandPaletteStore, 'openPalette')
    const runSpy = vi.spyOn(commandActionStore, 'runOptimization')
    const settingsSpy = vi.spyOn(workflowStore, 'selectWorkspace')

    const wrapper = mount(DesktopTitlebar)

    await wrapper.get('button[aria-label="Open command palette"]').trigger('click')
    await wrapper.get('.titlebarPrimary').trigger('click')
    await wrapper.get('button[aria-label="Open settings workspace"]').trigger('click')
    await wrapper.get('.titlebarPrimary').trigger('click')

    expect(openPaletteSpy).toHaveBeenCalledTimes(1)
    expect(settingsSpy).toHaveBeenCalledWith('settings')
    expect(settingsSpy).toHaveBeenCalledWith('workflow')
    expect(runSpy).toHaveBeenCalledTimes(1)
  })

  it('uses Options stage labels and routes expression search to the stage action', async () => {
    const dataStore = useDataStore()
    const optionsStore = useOptionsExpressionStore()
    const workflowStore = useWorkflowStore()
    dataStore.dataId = 'data-options'
    optionsStore.selectedDatasetId = 'options-dataset'
    workflowStore.selectMode('options', { persist: false })
    workflowStore.selectWorkflowStep('validate', { persist: false })
    const stageAction = document.createElement('button')
    stageAction.dataset.qaAction = 'options.expression.run'
    const stageActionSpy = vi.fn()
    stageAction.addEventListener('click', stageActionSpy)
    document.body.append(stageAction)

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.get('.titlebarPrimary')
    expect(primary.attributes('aria-label')).toBe('Run expression search')
    expect((primary.element as HTMLButtonElement).disabled).toBe(false)
    await primary.trigger('click')
    expect(stageActionSpy).toHaveBeenCalledTimes(1)

    workflowStore.selectWorkflowStep('analyze', { persist: false })
    await nextTick()
    expect(primary.attributes('aria-label')).toBe('Run options validation')
    expect((primary.element as HTMLButtonElement).disabled).toBe(true)
    wrapper.unmount()
    stageAction.remove()
  })

  it('uses a workspace action instead of leaking the last workflow action outside Research', async () => {
    const workflowStore = useWorkflowStore()
    workflowStore.selectMode('ml-signal', { persist: false })
    workflowStore.selectWorkflowStep('data', { persist: false })
    workflowStore.selectWorkspace('live')

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.get('.titlebarPrimary')

    expect(primary.attributes('aria-label')).toBe('Open research')
    expect(primary.text()).toContain('Research')

    await primary.trigger('click')

    expect(workflowStore.workspace).toBe('workflow')
  })

  it('routes the conventional primary action to validation on the Validate step', async () => {
    const commandActionStore = useCommandActionStore()
    const dataStore = useDataStore()
    const optimizeStore = useOptimizeStore()
    const workflowArtifactsStore = useWorkflowArtifactsStore()
    const workflowStore = useWorkflowStore()
    workflowStore.selectWorkflowStep('validate', { persist: false })
    dataStore.dataId = 'data-1'
    workflowArtifactsStore.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-07-18T12:00:00Z',
        req: { data_id: 'data-1', strategy_kind: 'DoubleMa' },
        res: { top: [{}] },
      },
    })
    optimizeStore.$patch({ validateDisabled: false, runDisabled: false })
    const validateSpy = vi.spyOn(commandActionStore, 'validateOptimization')
    const runSpy = vi.spyOn(commandActionStore, 'runOptimization')

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.get('.titlebarPrimary')

    expect(primary.attributes('aria-label')).toBe('Run validation')
    expect(primary.text()).toContain('Validate')
    await primary.trigger('click')

    expect(validateSpy).toHaveBeenCalledTimes(1)
    expect(runSpy).not.toHaveBeenCalled()
  })

  it('blocks conventional validation when retained evidence belongs to another dataset path', async () => {
    const commandActionStore = useCommandActionStore()
    const dataStore = useDataStore()
    const optimizeStore = useOptimizeStore()
    const workflowArtifactsStore = useWorkflowArtifactsStore()
    const workflowStore = useWorkflowStore()
    workflowStore.selectWorkflowStep('validate', { persist: false })
    dataStore.dataId = 'data-1'
    dataStore.baseCsvPath = 'C:/samples/vectorgrid_synthetic_ohlcv.csv'
    workflowArtifactsStore.applyVueRuntimePayload({
      currentRun: {
        ts: '2026-07-18T12:00:00Z',
        req: { data_id: 'data-1', strategy_kind: 'DoubleMa' },
        dataset_meta: { csv_path: 'C:/market/AAPL.csv' },
        res: { top: [{}] },
      },
    })
    optimizeStore.$patch({ validateDisabled: false, runDisabled: false })
    const validateSpy = vi.spyOn(commandActionStore, 'validateOptimization')

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.get('.titlebarPrimary')

    expect(primary.attributes()).toHaveProperty('disabled')
    expect(primary.attributes('title')).toContain('compatible optimization')
    await primary.trigger('click')
    expect(validateSpy).not.toHaveBeenCalled()
  })

  it('uses native window controls when available', async () => {
    const wrapper = mount(DesktopTitlebar)
    const buttons = wrapper.findAll('.windowControl')

    await buttons[0]?.trigger('click')
    await buttons[1]?.trigger('click')
    await buttons[2]?.trigger('click')

    expect(minimizeMock).toHaveBeenCalledTimes(1)
    expect(maximizeMock).toHaveBeenCalled()
    expect(unmaximizeMock).not.toHaveBeenCalled()
    expect(invokeCommandMock).toHaveBeenCalledWith('exit_application', undefined)
    expect(destroyMock).not.toHaveBeenCalled()
  })

  it('automatically reverts a maximize result that spans beyond the current monitor', async () => {
    outerSizeMock.mockResolvedValue({ width: 7680, height: 4272 })
    const wrapper = mount(DesktopTitlebar)
    await flushPromises()
    isMaximizedMock.mockResolvedValueOnce(true).mockResolvedValueOnce(false)

    await wrapper.get('button[aria-label="Maximize window"]').trigger('click')
    await flushPromises()

    expect(maximizeMock).toHaveBeenCalledTimes(1)
    expect(unmaximizeMock).toHaveBeenCalledTimes(1)
    expect(wrapper.get('[role="status"]').text()).toContain('exceeded the current monitor')
    expect(wrapper.get('button[aria-label="Maximize window"]').attributes('title')).toContain('reverted')
  })

  it('repairs unsafe persisted maximized geometry during startup', async () => {
    isMaximizedMock.mockResolvedValueOnce(true).mockResolvedValueOnce(false)
    outerSizeMock.mockResolvedValue({ width: 7680, height: 4272 })

    const wrapper = mount(DesktopTitlebar)
    await flushPromises()

    expect(maximizeMock).not.toHaveBeenCalled()
    expect(unmaximizeMock).toHaveBeenCalledTimes(1)
    expect(wrapper.get('[role="status"]').text()).toContain('exceeded the current monitor')
  })

  it('guards close when the live runner is active', async () => {
    const executionStore = useExecutionStore()
    executionStore.runnerActive = true

    const wrapper = mount(DesktopTitlebar, {
      attachTo: document.body,
    })

    await wrapper.get('button[aria-label="Close window"]').trigger('click')
    await nextTick()

    expect(closeMock).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('Exit VectorGrid now?')
    expect(wrapper.text()).toContain('The live runner is active.')
    expect(wrapper.text()).toContain('Close exits VectorGrid instead of minimizing it.')

    await wrapper.get('.confirmationDialogActions button:last-child').trigger('click')
    await nextTick()

    expect(invokeCommandMock).toHaveBeenCalledWith('exit_application', undefined)
    expect(destroyMock).not.toHaveBeenCalled()
  })

  it('guards native close requests while tracked work is active', async () => {
    const taskStore = useTaskStore()
    taskStore.$patch({
      activeById: {
        task_1: {
          id: 'task_1',
          command: 'run_double_ma_optimization',
          label: 'Optimization',
          status: 'running',
          startedAt: 1_700_000_000_000,
        },
      },
      activeOrder: ['task_1'],
    })

    const wrapper = mount(DesktopTitlebar, {
      attachTo: document.body,
    })
    await vi.waitFor(() => {
      expect(onCloseRequestedMock).toHaveBeenCalledTimes(1)
    })
    const closeRequestCalls = onCloseRequestedMock.mock.calls as unknown[][]
    const firstCloseRequestCall = closeRequestCalls.length > 0 ? (closeRequestCalls[0] as unknown[]) : undefined
    const handler = firstCloseRequestCall?.[0] as ((event: { preventDefault: () => void }) => Promise<void>) | undefined
    const preventDefault = vi.fn()

    expect(handler).toBeTruthy()
    await handler?.({ preventDefault })
    await nextTick()

    expect(preventDefault).toHaveBeenCalledTimes(1)
    expect(closeMock).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('Optimization is still active.')
  })

  it('does not guard native close requests for short non-progress tasks without a live runner', async () => {
    const taskStore = useTaskStore()
    taskStore.$patch({
      activeById: {
        task_short: {
          id: 'task_short',
          command: 'execute_strategy_signal_once',
          label: 'Manual execution',
          status: 'running',
          startedAt: 1_700_000_000_000,
        },
      },
      activeOrder: ['task_short'],
    })

    const wrapper = mount(DesktopTitlebar, {
      attachTo: document.body,
    })
    await vi.waitFor(() => {
      expect(onCloseRequestedMock).toHaveBeenCalledTimes(1)
    })
    const closeRequestCalls = onCloseRequestedMock.mock.calls as unknown[][]
    const firstCloseRequestCall = closeRequestCalls.length > 0 ? (closeRequestCalls[0] as unknown[]) : undefined
    const handler = firstCloseRequestCall?.[0] as ((event: { preventDefault: () => void }) => Promise<void>) | undefined
    const preventDefault = vi.fn()

    expect(handler).toBeTruthy()
    await handler?.({ preventDefault })
    await nextTick()

    expect(preventDefault).not.toHaveBeenCalled()
    expect(wrapper.text()).not.toContain('Exit VectorGrid now?')
  })

  it('starts dragging from the titlebar drag surfaces', async () => {
    const wrapper = mount(DesktopTitlebar)

    await wrapper.find('.titlebarIdentity').trigger('mousedown', { button: 0 })

    expect(startDraggingMock).toHaveBeenCalledTimes(1)
  })

  it('switches the primary action to stop while a run is cancellable', async () => {
    const optimizeStore = useOptimizeStore()
    const commandActionStore = useCommandActionStore()
    optimizeStore.$patch({ canCancel: true, runDisabled: false })
    const cancelSpy = vi.spyOn(commandActionStore, 'cancelOptimization')

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.find('.titlebarPrimary')

    expect(primary.text()).toContain('Stop')
    await primary.trigger('click')

    expect(cancelSpy).toHaveBeenCalledTimes(1)
  })

  it('keeps icon-only titlebar controls accessible through stable aria labels', () => {
    const wrapper = mount(DesktopTitlebar)

    const labels = wrapper.findAll('button').map((button) => button.attributes('aria-label')).filter(Boolean)
    expect(labels).toContain('Open command palette')
    expect(labels).toContain('Open settings workspace')
    expect(labels).toContain('Run optimization')
    expect(labels).toContain('Minimize window')
    expect(labels).toContain('Maximize window')
    expect(labels).toContain('Close window')
  })

  it('switches the native window control label to restore when the window is maximized', async () => {
    isMaximizedMock.mockResolvedValue(true)

    const wrapper = mount(DesktopTitlebar)
    await nextTick()
    await nextTick()

    const labels = wrapper.findAll('.windowControl').map((button) => button.attributes('aria-label')).filter(Boolean)
    expect(labels).toContain('Restore window')
    expect(labels).not.toContain('Maximize window')
  })
})
