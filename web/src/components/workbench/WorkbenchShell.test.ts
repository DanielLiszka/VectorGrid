import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useTaskCenterVisibility } from '../../composables/useTaskCenterVisibility'
import { useHistoryStore } from '../../stores/historyStore'
import { useWorkflowStore } from '../../stores/workflowStore'
import { createTestPinia } from '../../test/pinia'
import WorkbenchShell from './WorkbenchShell.vue'

async function flushUi(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('WorkbenchShell', () => {
  beforeEach(() => {
    localStorage.clear()
    localStorage.setItem('vta_ui_mode_v1', 'advanced')
    document.body.innerHTML = ''
    useTaskCenterVisibility().value = false
  })

  it('renders the feature-flag shell with preserved shell selectors and mount targets', async () => {
    const pinia = createTestPinia()
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('optimize', { persist: false })

    const wrapper = mount(WorkbenchShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    expect(document.querySelector('[data-qa-shell="workbench"]')).not.toBeNull()
    expect(document.querySelector('#workspaceBar .railTabList')?.getAttribute('role')).toBe('tablist')
    expect(document.querySelector('#workflowStepBar')?.getAttribute('role')).toBe('tablist')
    expect(document.querySelector('#controlsTabBar')?.getAttribute('role')).toBe('tablist')
    expect(document.querySelector('#resultsTabBar')?.getAttribute('role')).toBe('tablist')
    expect(document.querySelector('#runFooterShell')).not.toBeNull()
    expect(document.querySelector('#runFooterVueShellMount')).not.toBeNull()
    expect(document.querySelector('#optVueShellMount')).not.toBeNull()
    expect(document.querySelector('#topVueShellMount')).not.toBeNull()
    expect(document.querySelector('[data-qa-panel="workbench.researchCockpit"]')).not.toBeNull()
    expect(document.querySelector('[data-qa-state="workbench.cockpit.card.dataset"]')?.textContent).toContain('No dataset')
    expect(document.querySelector('#shellSelectionStatus')?.textContent).toContain('Workspace Research.')

    wrapper.unmount()
  })

  it('switches workspaces through the premium rail without changing internal workspace ids', async () => {
    const pinia = createTestPinia()
    const workflowStore = useWorkflowStore(pinia)

    const wrapper = mount(WorkbenchShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    await wrapper.get('[data-qa-action="shell.workspace.aggregate"]').trigger('click')
    await flushUi()
    expect(workflowStore.workspace).toBe('aggregate')
    expect(document.querySelector('#workspaceBar [data-workspace="aggregate"]')?.getAttribute('aria-selected')).toBe('true')
    expect(document.querySelector('#results_panel_meta')).not.toBeNull()
    expect(document.body.textContent || '').toContain('Runs')

    await wrapper.get('[data-qa-action="shell.workspace.portfolio"]').trigger('click')
    await flushUi()
    expect(workflowStore.workspace).toBe('portfolio')
    expect(document.querySelector('#workspaceBar [data-workspace="portfolio"]')?.getAttribute('aria-selected')).toBe('true')
    expect(document.querySelector('#portfolioShellGrid')).not.toBeNull()
    expect(document.body.textContent || '').toContain('Portfolio')

    await wrapper.get('[data-qa-action="shell.workspace.workflow"]').trigger('click')
    await flushUi()
    expect(workflowStore.workspace).toBe('workflow')
    expect(document.querySelector('#workflowStepBar')).not.toBeNull()

    wrapper.unmount()
  })

  it('keeps the advanced workbench chrome mounted for the default shell', async () => {
    const pinia = createTestPinia()

    const wrapper = mount(WorkbenchShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    expect(document.querySelector('#uiModeBar')).toBeNull()
    expect(document.querySelector('#workspaceBar')).not.toBeNull()
    expect(document.querySelector('#guidedShellGrid')).toBeNull()
    expect(document.querySelector('#runFooterShell')).not.toBeNull()

    wrapper.unmount()
  })

  it('exposes inspector mode switching and collapse state', async () => {
    const pinia = createTestPinia()
    const workflowStore = useWorkflowStore(pinia)
    const historyStore = useHistoryStore(pinia)
    const clearSessionSpy = vi.spyOn(historyStore, 'clearSession').mockImplementation(() => {})
    historyStore.canClearHistory = true
    workflowStore.selectWorkflowStep('validate', { persist: false })

    const wrapper = mount(WorkbenchShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    expect(document.querySelector('[data-qa-state="workbench.inspector.activeMode"]')?.textContent).toContain('Validation')
    expect(document.querySelector('[data-qa-state="workbench.inspector.section.compatibility"]')).not.toBeNull()
    await wrapper.get('[data-qa-action="workbench.inspector.mode.tasks"]').trigger('click')
    await flushUi()
    expect(document.querySelector('[data-qa-state="workbench.inspector.activeMode"]')?.textContent).toContain('Tasks')
    expect(document.querySelector('[data-qa-action="workbench.inspector.tasks.open"]')).not.toBeNull()

    await wrapper.get('[data-qa-action="workbench.inspector.mode.history"]').trigger('click')
    await flushUi()
    expect(document.querySelector('[data-qa-action="workbench.inspector.history.clearSession"]')).not.toBeNull()
    await wrapper.get('[data-qa-action="workbench.inspector.history.clearSession"]').trigger('click')
    expect(clearSessionSpy).toHaveBeenCalledTimes(1)

    await wrapper.get('[data-qa-action="workbench.inspector.toggle"]').trigger('click')
    await flushUi()
    expect(document.querySelector('[data-qa-panel="workbench.inspector"]')?.getAttribute('data-collapsed')).toBe('true')

    wrapper.unmount()
  })
})
