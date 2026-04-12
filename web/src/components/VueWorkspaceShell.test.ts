// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import VueWorkspaceShell from './VueWorkspaceShell.vue'
import { useTaskCenterVisibility } from '../composables/useTaskCenterVisibility'
import { useSettingsStore } from '../stores/settingsStore'
import { useWorkflowStore } from '../stores/workflowStore'

async function flushUi(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('VueWorkspaceShell', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
    window.innerWidth = 1440
    useTaskCenterVisibility().value = false
  })

  it('shows workspace, mode, and workflow-step navigation with local tabs for the active step', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('optimize', { persist: false })

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(document.querySelector('#workflowTabBar')).toBeNull()
    expect(document.querySelector('#workspaceBar')?.getAttribute('role')).toBe('tablist')
    expect(document.querySelector('#modeBar')?.getAttribute('role')).toBe('tablist')
    expect(document.querySelector('#workflowStepBar')?.getAttribute('role')).toBe('tablist')

    const controlsLabels = Array.from(document.querySelectorAll('#controlsTabBar .tabBtn')).map((el) => el.textContent?.trim())
    const resultsLabels = Array.from(document.querySelectorAll('#resultsTabBar .tabBtn')).map((el) => el.textContent?.trim())

    expect(controlsLabels).toEqual(['Optimize', 'Strategy', 'Advanced'])
    expect(resultsLabels).toEqual(['Overview', 'Top', 'Pareto', 'Heatmap'])

    wrapper.unmount()
  })

  it('switches shell posture for MlSignal mode and hides conventional results chrome', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectMode('ml-signal', { persist: false })
    workflowStore.selectWorkflowStep('validate', { persist: false })

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    const stepLabels = Array.from(document.querySelectorAll('#workflowStepBar .tabBtn')).map((el) => el.textContent?.trim())
    expect(stepLabels).toEqual([
      '1 Data Setup',
      '2 Score Cache',
      '3 Policy Validation',
      '4 Policy Sweep',
      '5 Artifacts',
    ])
    expect(document.querySelector('#controlsTabBar')).toBeNull()
    expect((document.querySelector('#shellResultsColumn') as HTMLElement | null)?.style.display).toBe('none')
    expect(document.body.textContent || '').toContain('ML Signal')
    expect(document.body.textContent || '').toContain('Stage')

    wrapper.unmount()
  })

  it('keeps shared library context truthful when MlSignal mode is active outside the workflow workspace', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectMode('ml-signal', { persist: false })
    workflowStore.selectWorkspace('library')

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    const text = document.body.textContent || ''
    expect(text).toContain('Workspace')
    expect(text).toContain('Run Library')
    expect(text).toContain('Mode')
    expect(text).toContain('ML Signal')
    expect(text).not.toContain('WorkflowML Signal')
    expect(text).not.toContain('StageData Setup')

    wrapper.unmount()
  })

  it('promotes live into its own shell surface with the current support envelope visible', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('optimize', { persist: false })
    workflowStore.selectControlsTab('strategy', { persist: false })
    workflowStore.selectResultsTab('top', { persist: false })
    workflowStore.selectWorkspace('live')

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    const text = document.body.textContent || ''
    expect(text).toContain('Live')
    expect(text).toContain('Current live support envelope')
    expect(text).toContain('DoubleMa · CompositeCpu')
    expect(text).toContain('One symbol · one runner · one stream')
    expect((document.querySelector('#liveShellGrid') as HTMLElement | null)?.style.display).not.toBe('none')
    expect((document.querySelector('#settingsShellGrid') as HTMLElement | null)?.style.display).toBe('none')

    wrapper.unmount()
  })

  it('keeps five default context cards and routes task review through the shell launcher', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    expect(wrapper.text()).not.toContain('Guided default workflow with power tools on demand.')
    expect(document.querySelectorAll('.contextStrip > .contextCard')).toHaveLength(5)
    expect(document.querySelector('.contextOverflowSummary')?.textContent).toContain('Rows')
    expect(document.querySelector('.contextOverflowSummary')?.textContent).toContain('Loaded ID')
    expect(document.querySelector('img.shellPostureIcon')).not.toBeNull()
    expect(document.body.textContent || '').toContain('Build posture')

    await wrapper.get('.taskCenterLauncher').trigger('click')
    await flushUi()
    await flushUi()

    expect(useTaskCenterVisibility().value).toBe(true)

    wrapper.unmount()
  })

  it('hides the controls column when the persisted controls-collapsed setting is enabled', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const settingsStore = useSettingsStore(pinia)
    workflowStore.selectWorkflowStep('optimize', { persist: false })
    settingsStore.setCheckboxField('settingsControlsCollapsed', true)

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect((document.querySelector('.controlsColumn') as HTMLElement | null)?.style.display).toBe('none')
    expect(document.querySelector('#resultsTabBar')).not.toBeNull()

    wrapper.unmount()
  })

  it('mounts the finalizer workflow step on the meta surface', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('finalize', { persist: false })

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    const stepLabels = Array.from(document.querySelectorAll('#workflowStepBar .tabBtn')).map((el) => el.textContent?.trim())
    expect(stepLabels).toContain('5 Finalizer')
    expect(document.querySelector('#controlsTabBar')).toBeNull()
    expect(document.querySelector('#resultsTabBar')).toBeNull()
    expect(document.querySelector('[data-vue-panel=\"meta\"]')).not.toBeNull()

    wrapper.unmount()
  })

  it('moves workspace navigation from the currently focused button', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    const libraryButton = document.querySelector('#workspaceBar [data-workspace="library"]') as HTMLButtonElement | null
    const aggregateButton = document.querySelector('#workspaceBar [data-workspace="aggregate"]') as HTMLButtonElement | null
    const settingsButton = document.querySelector('#workspaceBar [data-workspace="settings"]') as HTMLButtonElement | null
    const workflowButton = document.querySelector('#workspaceBar [data-workspace="workflow"]') as HTMLButtonElement | null
    const shellMainPanel = document.querySelector('#shellMainPanel') as HTMLElement | null

    expect(libraryButton).not.toBeNull()
    expect(aggregateButton).not.toBeNull()
    expect(settingsButton).not.toBeNull()
    expect(workflowButton).not.toBeNull()
    expect(shellMainPanel?.getAttribute('role')).toBe('tabpanel')
    expect(workflowButton?.getAttribute('role')).toBe('tab')
    expect(workflowButton?.getAttribute('aria-controls')).toBe('shellMainPanel')
    expect(workflowButton?.getAttribute('aria-selected')).toBe('true')
    expect(workflowButton?.getAttribute('tabindex')).toBe('0')
    expect(libraryButton?.getAttribute('tabindex')).toBe('-1')

    libraryButton?.focus()
    libraryButton?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }))
    await flushUi()
    expect(aggregateButton?.getAttribute('aria-selected')).toBe('true')
    expect(aggregateButton?.getAttribute('tabindex')).toBe('0')
    expect(libraryButton?.getAttribute('tabindex')).toBe('-1')
    expect(document.activeElement).toBe(aggregateButton)

    settingsButton?.focus()
    settingsButton?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', bubbles: true }))
    await flushUi()
    expect(workflowButton?.getAttribute('aria-selected')).toBe('true')
    expect(document.activeElement).toBe(workflowButton)
    expect(shellMainPanel?.getAttribute('aria-labelledby')).toBe('workflow_step_tab_data')
    expect(document.querySelector('#shellSelectionStatus')?.textContent).toContain('Workspace Workflow.')

    workflowButton?.focus()
    workflowButton?.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true }))
    await flushUi()
    expect(settingsButton?.getAttribute('aria-selected')).toBe('true')
    expect(shellMainPanel?.getAttribute('aria-labelledby')).toBe('workspace_tab_settings')
    expect(document.querySelector('#shellSelectionStatus')?.textContent).toContain('Workspace Settings.')
    expect(document.activeElement).toBe(settingsButton)

    wrapper.unmount()
  })

  it('moves workflow-step navigation from the currently focused button', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('data', { persist: false })

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    const validateButton = document.querySelector('#workflowStepBar [data-workflow-step="validate"]') as HTMLButtonElement | null
    const analyzeButton = document.querySelector('#workflowStepBar [data-workflow-step="analyze"]') as HTMLButtonElement | null
    const finalizeButton = document.querySelector('#workflowStepBar [data-workflow-step="finalize"]') as HTMLButtonElement | null
    const dataButton = document.querySelector('#workflowStepBar [data-workflow-step="data"]') as HTMLButtonElement | null

    expect(validateButton).not.toBeNull()
    expect(analyzeButton).not.toBeNull()
    expect(finalizeButton).not.toBeNull()
    expect(dataButton).not.toBeNull()
    expect(dataButton?.getAttribute('role')).toBe('tab')
    expect(dataButton?.getAttribute('aria-controls')).toBe('shellMainPanel')
    expect(dataButton?.getAttribute('aria-selected')).toBe('true')
    expect(dataButton?.getAttribute('tabindex')).toBe('0')

    validateButton?.focus()
    validateButton?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }))
    await flushUi()
    expect(analyzeButton?.getAttribute('aria-selected')).toBe('true')
    expect(document.activeElement).toBe(analyzeButton)
    expect(document.querySelector('#shellMainPanel')?.getAttribute('aria-labelledby')).toBe('workflow_step_tab_analyze')

    finalizeButton?.focus()
    finalizeButton?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', bubbles: true }))
    await flushUi()
    expect(dataButton?.getAttribute('aria-selected')).toBe('true')
    expect(document.activeElement).toBe(dataButton)

    dataButton?.focus()
    dataButton?.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true }))
    await flushUi()
    expect(finalizeButton?.getAttribute('aria-selected')).toBe('true')
    expect(document.querySelector('#shellSelectionStatus')?.textContent).toContain('Workflow step 5 Finalizer.')
    expect(document.activeElement).toBe(finalizeButton)

    wrapper.unmount()
  })

  it('keeps controls and results keyboard navigation aligned with the focused tab', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('optimize', { persist: false })

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    const strategyButton = document.querySelector('#controlsTabBar [data-tab="strategy"]') as HTMLButtonElement | null
    const advancedButton = document.querySelector('#controlsTabBar [data-tab="advanced"]') as HTMLButtonElement | null
    const topButton = document.querySelector('#resultsTabBar [data-tab="top"]') as HTMLButtonElement | null
    const heatmapButton = document.querySelector('#resultsTabBar [data-tab="heatmap"]') as HTMLButtonElement | null

    expect(strategyButton).not.toBeNull()
    expect(advancedButton).not.toBeNull()
    expect(topButton).not.toBeNull()
    expect(heatmapButton).not.toBeNull()

    strategyButton?.focus()
    strategyButton?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }))
    await flushUi()
    expect(advancedButton?.getAttribute('aria-selected')).toBe('true')
    expect(document.activeElement).toBe(advancedButton)

    topButton?.focus()
    topButton?.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true }))
    await flushUi()
    expect(heatmapButton?.getAttribute('aria-selected')).toBe('true')
    expect(document.activeElement).toBe(heatmapButton)

    wrapper.unmount()
  })

  it('clamps split-divider keyboard moves to the supported min and max bounds', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('optimize', { persist: false })
    workflowStore.setSplitFraction(0.34, { persist: false })

    const wrapper = mount(VueWorkspaceShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    const divider = document.querySelector('.workspaceDivider') as HTMLButtonElement | null
    expect(divider).not.toBeNull()
    expect(divider?.getAttribute('role')).toBe('separator')
    const minPercent = Number(divider?.getAttribute('aria-valuemin') || '0')
    const maxPercent = Number(divider?.getAttribute('aria-valuemax') || '100')

    divider?.focus()
    divider?.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true }))
    await flushUi()
    expect(workflowStore.splitFraction).toBeCloseTo(maxPercent / 100, 5)
    expect(divider?.getAttribute('aria-valuenow')).toBe(String(maxPercent))

    divider?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', bubbles: true }))
    await flushUi()
    expect(workflowStore.splitFraction).toBeCloseTo(minPercent / 100, 5)
    expect(divider?.getAttribute('aria-valuenow')).toBe(String(minPercent))

    wrapper.unmount()
  })
})
