import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useTaskCenterVisibility } from '../../composables/useTaskCenterVisibility'
import { loadExecutionStrategySupportMatrix } from '../../services/executionService'
import { useHistoryStore } from '../../stores/historyStore'
import { useWorkflowStore } from '../../stores/workflowStore'
import { createTestPinia } from '../../test/pinia'
import { tauriRuntimeAvailable } from '../../tauri/invoke'
import WorkbenchShell from './WorkbenchShell.vue'

vi.mock('../../tauri/invoke', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../tauri/invoke')>()
  return {
    ...actual,
    tauriRuntimeAvailable: vi.fn(() => false),
  }
})

vi.mock('../../services/executionService', async (importOriginal) => {
  const actual =
    await importOriginal<typeof import('../../services/executionService')>()
  return {
    ...actual,
    loadExecutionStrategySupportMatrix: vi.fn(async () => []),
  }
})

const tauriRuntimeAvailableMock = vi.mocked(tauriRuntimeAvailable)
const loadExecutionStrategySupportMatrixMock = vi.mocked(
  loadExecutionStrategySupportMatrix,
)

async function flushUi(): Promise<void> {
  await nextTick()
  await Promise.resolve()
  await nextTick()
}

describe('WorkbenchShell', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  beforeEach(() => {
    localStorage.clear()
    localStorage.setItem('vta_ui_mode_v1', 'advanced')
    document.body.innerHTML = ''
    useTaskCenterVisibility().value = false
    tauriRuntimeAvailableMock.mockReturnValue(false)
    loadExecutionStrategySupportMatrixMock.mockResolvedValue([])
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
    expect(document.querySelector('[data-qa-action="shell.mode.options"]')).not.toBeNull()
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
    expect(document.querySelector('#modeBar')).toBeNull()
    expect(document.querySelector('#workspaceBar [data-workspace="aggregate"]')?.getAttribute('aria-selected')).toBe('true')
    expect(document.querySelector('#results_panel_meta')).not.toBeNull()
    expect(document.body.textContent || '').toContain('Runs')

    await wrapper.get('[data-qa-action="shell.workspace.portfolio"]').trigger('click')
    await flushUi()
    expect(workflowStore.workspace).toBe('portfolio')
    expect(document.querySelector('#workspaceBar [data-workspace="portfolio"]')?.getAttribute('aria-selected')).toBe('true')
    expect(document.querySelector('#portfolioShellGrid')).not.toBeNull()
    expect(document.querySelector('#portfolioShellGrid')?.getAttribute('aria-label')).toBe('Portfolio research workspace shell')
    expect(document.body.textContent || '').toContain('Portfolio')

    await wrapper.get('[data-qa-action="shell.workspace.settings"]').trigger('click')
    await flushUi()
    expect(workflowStore.workspace).toBe('settings')
    expect(document.querySelector('#settingsShellGrid')?.getAttribute('aria-label')).toBe('Settings workspace shell')

    await wrapper.get('[data-qa-action="shell.workspace.workflow"]').trigger('click')
    await flushUi()
    expect(workflowStore.workspace).toBe('workflow')
    expect(document.querySelector('#modeBar')).not.toBeNull()
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

  it('opens the advanced drawer without changing the active stage', async () => {
    const pinia = createTestPinia()
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflowStep('validate', { persist: false })

    const wrapper = mount(WorkbenchShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    await wrapper.get('[data-qa-action="shell.advancedDrawer.toggle"]').trigger('click')
    await flushUi()

    const drawer = document.querySelector('#researchAdvancedDrawer') as HTMLElement | null
    expect(drawer).not.toBeNull()
    expect(drawer?.style.display).not.toBe('none')
    expect(drawer?.textContent || '').toContain('Validation Controls')
    expect(drawer?.textContent || '').toContain('Validation foregrounds robustness evidence')
    expect(workflowStore.workflowStep).toBe('validate')
    expect(document.querySelector('[data-qa-action="shell.advancedDrawer.openStrategy"]')).not.toBeNull()
    expect(document.querySelector('[data-qa-action="shell.advancedDrawer.openAdvancedControls"]')).not.toBeNull()

    await wrapper.get('[data-qa-action="shell.advancedDrawer.close"]').trigger('click')
    await flushUi()
    expect(workflowStore.workflowStep).toBe('validate')
    expect(drawer?.style.display).toBe('none')

    await wrapper.get('[data-qa-action="shell.advancedDrawer.toggle"]').trigger('click')
    await wrapper.get('[data-qa-action="shell.advancedDrawer.openStrategy"]').trigger('click')
    await flushUi()

    expect(workflowStore.workflowStep).toBe('optimize')
    expect(workflowStore.controlsTab).toBe('strategy')

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

  it('keeps the inspector available as a compact drawer below 1180px', async () => {
    vi.stubGlobal('matchMedia', vi.fn(() => ({
      matches: true,
      media: '(max-width: 1180px)',
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })))
    const pinia = createTestPinia()
    const wrapper = mount(WorkbenchShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    const launcher = wrapper.get('[data-qa-action="workbench.inspector.compactOpen"]')
    ;(launcher.element as HTMLButtonElement).focus()
    expect(launcher.attributes('aria-expanded')).toBe('false')
    expect(wrapper.get('[data-qa-panel="workbench.inspector"]').attributes('aria-hidden')).toBe('true')

    await launcher.trigger('click')
    await flushUi()
    expect(launcher.attributes('aria-expanded')).toBe('true')
    const inspector = wrapper.get('[data-qa-panel="workbench.inspector"]')
    expect(inspector.attributes('aria-hidden')).toBeUndefined()
    expect(inspector.attributes('role')).toBe('dialog')
    expect(inspector.attributes('aria-modal')).toBe('true')
    expect(document.querySelector('.workbenchContent')?.hasAttribute('inert')).toBe(true)
    expect(document.activeElement).toBe(
      wrapper.get('[data-qa-action="workbench.inspector.toggle"]').element,
    )

    const focusable = Array.from(
      inspector.element.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    )
    const first = focusable[0]
    const last = focusable[focusable.length - 1]
    expect(first).toBeTruthy()
    expect(last).toBeTruthy()
    first?.focus()
    document.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'Tab',
        shiftKey: true,
        bubbles: true,
        cancelable: true,
      }),
    )
    expect(document.activeElement).toBe(last)

    document.dispatchEvent(
      new KeyboardEvent('keydown', {
        key: 'Escape',
        bubbles: true,
        cancelable: true,
      }),
    )
    await flushUi()
    expect(launcher.attributes('aria-expanded')).toBe('false')
    expect(document.activeElement).toBe(launcher.element)

    wrapper.unmount()
  })

  it('keeps live support matrix runtime failures operator-readable', async () => {
    const pinia = createTestPinia()
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('live')
    tauriRuntimeAvailableMock.mockReturnValue(true)
    loadExecutionStrategySupportMatrixMock.mockRejectedValueOnce({
      name: 'TauriRuntimeError',
      message:
        'Desktop runtime unavailable for execution strategy support matrix. Open this action in the Tauri desktop app.',
    })

    const wrapper = mount(WorkbenchShell, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushUi()

    expect(loadExecutionStrategySupportMatrixMock).toHaveBeenCalledTimes(1)
    const supportPane = wrapper.get('[aria-label="Current live support envelope"]')
    expect(supportPane.text()).toContain('Unavailable')
    expect(supportPane.text()).not.toContain('DoubleMa · CompositeCpu')
    const status = wrapper.get('[data-qa-state="workbench.liveSupport.status"]')
    expect(status.text()).toContain('Live support matrix unavailable:')
    expect(status.text()).toContain('Desktop runtime unavailable')
    expect(status.text()).not.toContain('TauriRuntimeError')

    loadExecutionStrategySupportMatrixMock.mockResolvedValueOnce([
      {
        strategyKind: 'DoubleMa',
        supported: true,
        adapter: 'double_ma_execution_adapter',
        supportLevel: 'signal_and_delta',
        notes: 'ready',
      },
    ])
    await wrapper.get('.liveSupportRetry').trigger('click')
    await flushUi()

    expect(loadExecutionStrategySupportMatrixMock).toHaveBeenCalledTimes(2)
    expect(supportPane.text()).toContain('DoubleMa')
    expect(supportPane.text()).toContain('double_ma_execution_adapter')
    expect(supportPane.text()).not.toContain('Retry support check')

    wrapper.unmount()
  })
})
