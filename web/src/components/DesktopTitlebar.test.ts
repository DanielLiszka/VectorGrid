// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import DesktopTitlebar from './DesktopTitlebar.vue'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useCommandActionStore } from '../stores/commandActionStore'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useWorkflowStore } from '../stores/workflowStore'

const minimizeMock = vi.fn(async () => undefined)
const toggleMaximizeMock = vi.fn(async () => undefined)
const closeMock = vi.fn(async () => undefined)
const startDraggingMock = vi.fn(async () => undefined)
const isMaximizedMock = vi.fn(async () => false)
const isFocusedMock = vi.fn(async () => true)
const onResizedMock = vi.fn(async () => () => {})
const onFocusChangedMock = vi.fn(async () => () => {})

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    minimize: minimizeMock,
    toggleMaximize: toggleMaximizeMock,
    close: closeMock,
    startDragging: startDraggingMock,
    isMaximized: isMaximizedMock,
    isFocused: isFocusedMock,
    onResized: onResizedMock,
    onFocusChanged: onFocusChangedMock,
  }),
}))

vi.mock('../tauri/invoke', () => ({
  tauriRuntimeAvailable: () => true,
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
    startDraggingMock.mockClear()
    isMaximizedMock.mockClear()
    isFocusedMock.mockClear()
    onResizedMock.mockClear()
    onFocusChangedMock.mockClear()
  })

  it('surfaces current workspace and keeps the titlebar free of dataset and task chips', () => {
    const workflowStore = useWorkflowStore()

    workflowStore.selectWorkflowStep('optimize')

    const wrapper = mount(DesktopTitlebar)

    expect(wrapper.text()).toContain('VectorGrid')
    expect(wrapper.text()).toContain('Workflow · Strategy Setup')
    expect(wrapper.text()).not.toContain('Dataset')
    expect(wrapper.findAll('.titlebarTool')).toHaveLength(3)
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

  it('uses native window controls when available', async () => {
    const wrapper = mount(DesktopTitlebar)
    const buttons = wrapper.findAll('.windowControl')

    await buttons[0]?.trigger('click')
    await buttons[1]?.trigger('click')
    await buttons[2]?.trigger('click')

    expect(minimizeMock).toHaveBeenCalledTimes(1)
    expect(toggleMaximizeMock).toHaveBeenCalled()
    expect(closeMock).toHaveBeenCalledTimes(1)
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
})
