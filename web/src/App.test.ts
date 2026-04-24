// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import App from './App.vue'
import { useSettingsStore } from './stores/settingsStore'
import { useWorkflowStore } from './stores/workflowStore'

const APP_STUBS = {
  AppRuntimeBanner: { template: '<div data-stub="AppRuntimeBanner" />' },
  ActionStatusBar: { template: '<div data-stub="ActionStatusBar" />' },
  DesktopTitlebar: { template: '<div data-stub="DesktopTitlebar" />' },
  GlobalShortcutBridge: { template: '<div data-stub="GlobalShortcutBridge" />' },
  VueWorkspaceShell: { template: '<div data-stub="VueWorkspaceShell" />' },
  AdvancedWorkspacePanel: { template: '<div data-stub="AdvancedWorkspacePanel" />' },
  AnalysisContextDock: { template: '<div data-stub="AnalysisContextDock" />' },
  CommandPaletteOverlay: { template: '<div data-stub="CommandPaletteOverlay" />' },
  DataWorkspacePanel: { template: '<div data-stub="DataWorkspacePanel" />' },
  DebugWorkspacePanel: { template: '<div data-stub="DebugWorkspacePanel" />' },
  DrillWorkspacePanel: { template: '<div data-stub="DrillWorkspacePanel" />' },
  ExecutionWorkspacePanel: { template: '<div data-stub="ExecutionWorkspacePanel" />' },
  HeatmapWorkspacePanel: { template: '<div data-stub="HeatmapWorkspacePanel" />' },
  GuidedWorkspacePanel: { template: '<div data-stub="GuidedWorkspacePanel" />' },
  HistoryWorkspacePanel: { template: '<div data-stub="HistoryWorkspacePanel" />' },
  HistoryContextDock: { template: '<div data-stub="HistoryContextDock" />' },
  MetaWorkspacePanel: { template: '<div data-stub="MetaWorkspacePanel" />' },
  MlSignalShellPanel: { template: '<div data-stub="MlSignalShellPanel" />' },
  OptimizeContextDock: { template: '<div data-stub="OptimizeContextDock" />' },
  OptimizeWorkspacePanel: { template: '<div data-stub="OptimizeWorkspacePanel" />' },
  OverviewWorkspacePanel: { template: '<div data-stub="OverviewWorkspacePanel" />' },
  ParetoWorkspacePanel: { template: '<div data-stub="ParetoWorkspacePanel" />' },
  SettingsWorkspacePanel: { template: '<div data-stub="SettingsWorkspacePanel" />' },
  StrategyWorkspacePanel: { template: '<div data-stub="StrategyWorkspacePanel" />' },
  TaskCenterOverlay: { template: '<div data-stub="TaskCenterOverlay" />' },
  TopWorkspacePanel: { template: '<div data-stub="TopWorkspacePanel" />' },
  ValidationContextDock: { template: '<div data-stub="ValidationContextDock" />' },
  ValidationWorkspacePanel: { template: '<div data-stub="ValidationWorkspacePanel" />' },
  WorkflowQuickSwitch: { template: '<div data-stub="WorkflowQuickSwitch" />' },
}

async function flushApp(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('App root shell', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
  })

  it('mounts the root shell surfaces without the startup overlay', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    mount(App, {
      global: {
        plugins: [pinia],
        stubs: APP_STUBS,
      },
      attachTo: document.body,
    })
    await flushApp()

    expect(document.querySelector('[data-stub="DesktopTitlebar"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="VueWorkspaceShell"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="CommandPaletteOverlay"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="GlobalShortcutBridge"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="AppRuntimeBanner"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="TaskCenterOverlay"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="ActionStatusBar"]')).not.toBeNull()
  })

  it('keeps guided mode focused by hiding advanced workflow panels and the advanced footer', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const settingsStore = useSettingsStore(pinia)
    settingsStore.setUiMode('guided')

    const wrapper = mount(App, {
      global: {
        plugins: [pinia],
        stubs: APP_STUBS,
      },
      attachTo: document.body,
    })
    await flushApp()

    expect(document.querySelector('[data-stub="GuidedWorkspacePanel"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="ActionStatusBar"]')).toBeNull()
    expect(document.querySelector('[data-stub="DataWorkspacePanel"]')).toBeNull()
    expect(document.querySelector('[data-stub="OptimizeWorkspacePanel"]')).toBeNull()

    wrapper.unmount()
  })

  it('keeps lazily mounted workflow panels mounted once they have been visited', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)

    const wrapper = mount(App, {
      global: {
        plugins: [pinia],
        stubs: APP_STUBS,
      },
      attachTo: document.body,
    })
    await flushApp()

    expect(document.querySelector('[data-stub="DataWorkspacePanel"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="OptimizeWorkspacePanel"]')).toBeNull()

    workflowStore.selectWorkflowStep('optimize', { persist: false })
    await flushApp()
    expect(document.querySelector('[data-stub="OptimizeWorkspacePanel"]')).not.toBeNull()

    workflowStore.selectWorkflowStep('data', { persist: false })
    await flushApp()
    expect(document.querySelector('[data-stub="OptimizeWorkspacePanel"]')).not.toBeNull()

    wrapper.unmount()
  })

  it('mounts execution under the live workspace instead of the settings workspace', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)

    mount(App, {
      global: {
        plugins: [pinia],
        stubs: APP_STUBS,
      },
      attachTo: document.body,
    })
    await flushApp()

    expect(document.querySelector('[data-stub="ExecutionWorkspacePanel"]')).toBeNull()

    workflowStore.selectWorkspace('settings', undefined, { persist: false })
    await flushApp()
    expect(document.querySelector('[data-stub="ExecutionWorkspacePanel"]')).toBeNull()
    expect(document.querySelector('[data-stub="SettingsWorkspacePanel"]')).not.toBeNull()

    workflowStore.selectWorkspace('live', undefined, { persist: false })
    await flushApp()
    expect(document.querySelector('[data-stub="ExecutionWorkspacePanel"]')).not.toBeNull()
  })

  it('keeps the execution panel mounted after the live workspace has been visited', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)

    mount(App, {
      global: {
        plugins: [pinia],
        stubs: APP_STUBS,
      },
      attachTo: document.body,
    })
    await flushApp()

    expect(document.querySelector('[data-stub="ExecutionWorkspacePanel"]')).toBeNull()

    workflowStore.selectWorkspace('live', undefined, { persist: false })
    await flushApp()
    expect(document.querySelector('[data-stub="ExecutionWorkspacePanel"]')).not.toBeNull()

    workflowStore.selectWorkspace('workflow', undefined, { persist: false })
    await flushApp()
    expect(document.querySelector('[data-stub="ExecutionWorkspacePanel"]')).not.toBeNull()
  })
})
