// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DesktopTitlebar from './DesktopTitlebar.vue'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'
import { useExecutionStore } from '../stores/executionStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useTaskStore } from '../stores/taskStore'
import { useWorkflowStore } from '../stores/workflowStore'

const minimizeMock = vi.fn(async () => undefined)
const toggleMaximizeMock = vi.fn(async () => undefined)
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
    toggleMaximize: toggleMaximizeMock,
    close: closeMock,
    destroy: destroyMock,
    startDragging: startDraggingMock,
    isMaximized: isMaximizedMock,
    isFocused: isFocusedMock,
    onResized: onResizedMock,
    onFocusChanged: onFocusChangedMock,
    onCloseRequested: onCloseRequestedMock,
  }),
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

    minimizeMock.mockClear()
    toggleMaximizeMock.mockClear()
    closeMock.mockClear()
    destroyMock.mockClear()
    startDraggingMock.mockClear()
    isMaximizedMock.mockClear()
    isFocusedMock.mockClear()
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
    const workflowStore = useWorkflowStore()
    const optimizeStore = useOptimizeStore()
    workflowStore.selectMode('ml-signal', { persist: false })
    optimizeStore.$patch({
      mlSignalDatasetExportPath: 'C:/data/ml_dataset.csv',
      mlSignalScoreInputPath: 'C:/data/ml_scores.csv',
      mlSignalScoreCachePath: 'C:/data/ml_score_cache.json',
      runDisabled: false,
      validateDisabled: false,
    })

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.get('.titlebarPrimary')

    workflowStore.selectWorkflowStep('data', { persist: false })
    await nextTick()
    expect(primary.attributes('aria-label')).toBe('Export dataset')

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
    const workflowStore = useWorkflowStore()

    const openPaletteSpy = vi.spyOn(commandPaletteStore, 'openPalette')
    const runSpy = vi.spyOn(commandActionStore, 'runOptimization')
    const settingsSpy = vi.spyOn(workflowStore, 'selectWorkspace')

    const wrapper = mount(DesktopTitlebar)

    await wrapper.get('button[aria-label="Open command palette"]').trigger('click')
    await wrapper.get('button[aria-label="Open settings workspace"]').trigger('click')
    await wrapper.get('.titlebarPrimary').trigger('click')

    expect(openPaletteSpy).toHaveBeenCalledTimes(1)
    expect(settingsSpy).toHaveBeenCalledWith('settings')
    expect(runSpy).toHaveBeenCalledTimes(1)
  })

  it('routes the conventional primary action to validation on the Validate step', async () => {
    const commandActionStore = useCommandActionStore()
    const optimizeStore = useOptimizeStore()
    const workflowStore = useWorkflowStore()
    workflowStore.selectWorkflowStep('validate', { persist: false })
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

  it('uses guided labels and routes guided primary actions through existing commands', async () => {
    const commandActionStore = useCommandActionStore()
    const settingsStore = useSettingsStore()
    const optimizeStore = useOptimizeStore()
    settingsStore.setUiMode('guided')
    settingsStore.setGuidedStep('validate')
    optimizeStore.$patch({ validateDisabled: false, runDisabled: false })
    const validateSpy = vi.spyOn(commandActionStore, 'validateOptimization')

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.get('.titlebarPrimary')

    expect(wrapper.text()).toContain('Guided · Validate')
    expect(primary.attributes('aria-label')).toBe('Run validation')
    await primary.trigger('click')

    expect(validateSpy).toHaveBeenCalledTimes(1)
  })

  it('opens settings from guided mode by switching to the advanced settings workspace', async () => {
    const settingsStore = useSettingsStore()
    const workflowStore = useWorkflowStore()
    settingsStore.setUiMode('guided')
    const settingsSpy = vi.spyOn(workflowStore, 'selectWorkspace')

    const wrapper = mount(DesktopTitlebar)
    await wrapper.get('button[aria-label="Open settings workspace"]').trigger('click')

    expect(settingsStore.uiMode).toBe('advanced')
    expect(settingsSpy).toHaveBeenCalledWith('settings')
  })

  it('does not let the guided titlebar bypass data or strategy choices', async () => {
    const commandActionStore = useCommandActionStore()
    const settingsStore = useSettingsStore()
    const optimizeStore = useOptimizeStore()
    settingsStore.setUiMode('guided')
    optimizeStore.$patch({ runDisabled: false, strategyKind: 'DoubleMa' })
    const loadSpy = vi.spyOn(commandActionStore, 'loadData')
    const runSpy = vi.spyOn(commandActionStore, 'runOptimization')

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.get('.titlebarPrimary')

    settingsStore.setGuidedStep('data')
    await nextTick()
    expect(primary.attributes('aria-label')).toBe('Choose data')
    expect((primary.element as HTMLButtonElement).disabled).toBe(true)
    await primary.trigger('click')

    settingsStore.setGuidedStep('strategy')
    await nextTick()
    expect(primary.attributes('aria-label')).toBe('Pick strategy')
    expect((primary.element as HTMLButtonElement).disabled).toBe(true)
    await primary.trigger('click')

    expect(loadSpy).not.toHaveBeenCalled()
    expect(runSpy).not.toHaveBeenCalled()
  })

  it('runs guided optimization only from the optimize step with a supported strategy', async () => {
    const commandActionStore = useCommandActionStore()
    const settingsStore = useSettingsStore()
    const optimizeStore = useOptimizeStore()
    settingsStore.setUiMode('guided')
    settingsStore.setGuidedStep('optimize')
    optimizeStore.$patch({ runDisabled: false, strategyKind: 'MlSignal' })
    const runSpy = vi.spyOn(commandActionStore, 'runOptimization')

    const wrapper = mount(DesktopTitlebar)
    const primary = wrapper.get('.titlebarPrimary')

    expect((primary.element as HTMLButtonElement).disabled).toBe(true)
    await primary.trigger('click')
    expect(runSpy).not.toHaveBeenCalled()

    optimizeStore.$patch({ strategyKind: 'DoubleMa' })
    await nextTick()
    expect((primary.element as HTMLButtonElement).disabled).toBe(false)
    await primary.trigger('click')
    expect(runSpy).toHaveBeenCalledTimes(1)
  })

  it('uses native window controls when available', async () => {
    const wrapper = mount(DesktopTitlebar)
    const buttons = wrapper.findAll('.windowControl')

    await buttons[0]?.trigger('click')
    await buttons[1]?.trigger('click')
    await buttons[2]?.trigger('click')

    expect(minimizeMock).toHaveBeenCalledTimes(1)
    expect(toggleMaximizeMock).toHaveBeenCalled()
    expect(invokeCommandMock).toHaveBeenCalledWith('exit_application', undefined)
    expect(destroyMock).not.toHaveBeenCalled()
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
