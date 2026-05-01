
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
  WorkbenchShell: { template: '<div data-stub="WorkbenchShell" />' },
  AdvancedWorkspacePanel: { template: '<div data-stub="AdvancedWorkspacePanel" />' },
  CommandPaletteOverlay: { template: '<div data-stub="CommandPaletteOverlay" />' },
  DataWorkspacePanel: { template: '<div data-stub="DataWorkspacePanel" />' },
  DebugWorkspacePanel: { template: '<div data-stub="DebugWorkspacePanel" />' },
  CandidateDetailWorkspacePanel: { template: '<div data-stub="CandidateDetailWorkspacePanel" />' },
  ExecutionWorkspacePanel: { template: '<div data-stub="ExecutionWorkspacePanel" />' },
  HeatmapWorkspacePanel: { template: '<div data-stub="HeatmapWorkspacePanel" />' },
  HistoryWorkspacePanel: { template: '<div data-stub="HistoryWorkspacePanel" />' },
  MetaWorkspacePanel: { template: '<div data-stub="MetaWorkspacePanel" />' },
  MlSignalShellPanel: { template: '<div data-stub="MlSignalShellPanel" />' },
  OptimizeWorkspacePanel: { template: '<div data-stub="OptimizeWorkspacePanel" />' },
  OverviewWorkspacePanel: { template: '<div data-stub="OverviewWorkspacePanel" />' },
  ParetoWorkspacePanel: { template: '<div data-stub="ParetoWorkspacePanel" />' },
  SettingsWorkspacePanel: { template: '<div data-stub="SettingsWorkspacePanel" />' },
  StrategyWorkspacePanel: { template: '<div data-stub="StrategyWorkspacePanel" />' },
  TaskCenterOverlay: { template: '<div data-stub="TaskCenterOverlay" />' },
  TopWorkspacePanel: { template: '<div data-stub="TopWorkspacePanel" />' },
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
    sessionStorage.clear()
    window.history.replaceState({}, '', '/')
    document.body.innerHTML = ''
  })

  it('mounts the advanced root shell surfaces without startup overlay chrome', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const settingsStore = useSettingsStore(pinia)
    settingsStore.setUiMode('advanced')

    mount(App, {
      global: {
        plugins: [pinia],
        stubs: APP_STUBS,
      },
      attachTo: document.body,
    })
    await flushApp()

    expect(document.querySelector('[data-stub="DesktopTitlebar"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="WorkbenchShell"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="CommandPaletteOverlay"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="GlobalShortcutBridge"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="AppRuntimeBanner"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="TaskCenterOverlay"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="ActionStatusBar"]')).not.toBeNull()
  })

  it('defaults first-run users into the advanced shell', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const wrapper = mount(App, {
      global: {
        plugins: [pinia],
        stubs: APP_STUBS,
      },
      attachTo: document.body,
    })
    await flushApp()

    expect(document.querySelector('[data-stub="WorkbenchShell"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="ActionStatusBar"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="DataWorkspacePanel"]')).not.toBeNull()

    wrapper.unmount()
  })

  it('ignores legacy guided mode requests and keeps the advanced shell mounted', async () => {
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

    expect(document.querySelector('[data-stub="WorkbenchShell"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="ActionStatusBar"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="DataWorkspacePanel"]')).not.toBeNull()
    expect(document.querySelector('[data-stub="OptimizeWorkspacePanel"]')).toBeNull()

    wrapper.unmount()
  })

  it('keeps lazily mounted workflow panels mounted once they have been visited', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    settingsStore.setUiMode('advanced')

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
    const settingsStore = useSettingsStore(pinia)
    settingsStore.setUiMode('advanced')

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
    const settingsStore = useSettingsStore(pinia)
    settingsStore.setUiMode('advanced')

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
