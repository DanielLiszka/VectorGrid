import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import HeatmapWorkspacePanel from './HeatmapWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { refreshHeatmapFromWorkflowRun, restoreHeatmapVisible } from '../services/analysisService'

vi.mock('../services/analysisService', () => ({
  refreshHeatmapFromWorkflowRun: vi.fn(),
  registerHeatmapCanvasController: vi.fn(),
  runHeatmapRepresentativeCandidateDetail: vi.fn(),
  restoreHeatmapVisible: vi.fn(() => false),
}))

const refreshHeatmapFromWorkflowRunMock = vi.mocked(refreshHeatmapFromWorkflowRun)
const restoreHeatmapVisibleMock = vi.mocked(restoreHeatmapVisible)

function installHeatmapPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="heatmap"></section>
    <div id="heatmapVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('HeatmapWorkspacePanel', () => {
  beforeEach(() => {
    installHeatmapPanelMountShell()
    localStorage.clear()
    vi.clearAllMocks()
    restoreHeatmapVisibleMock.mockReturnValue(false)
  })

  it('prefers restoring a visible heatmap over rebuilding it from the workflow run', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    restoreHeatmapVisibleMock.mockReturnValue(true)

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(restoreHeatmapVisibleMock).toHaveBeenCalled()
    expect(refreshHeatmapFromWorkflowRunMock).not.toHaveBeenCalled()
  })

  it('falls back to a workflow refresh and keeps export routing inside the analysis store', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    analysisStore.canHeatmapExportPng = true
    const exportSpy = vi.spyOn(analysisStore, 'heatmapExportPng').mockImplementation(() => undefined)

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(refreshHeatmapFromWorkflowRunMock).toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Heatmap bins > 0')
    expect(document.body.textContent || '').toContain('Current view')
    expect(document.body.textContent || '').toContain('Canvas')
    expect(document.body.textContent || '').toContain('Awaiting run')
    expect(document.body.textContent || '').toContain('Export')
    expect(document.body.textContent || '').toContain('Ready')
    const exportButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((candidate) =>
      String(candidate.textContent || '').includes('Export PNG'),
    )
    expect(exportButton?.disabled).toBe(false)
    exportButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()
    expect(exportSpy).toHaveBeenCalledTimes(1)
  })

  it('does not trigger a redundant heatmap rebuild just because the analysis timestamp changes', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(refreshHeatmapFromWorkflowRunMock).toHaveBeenCalledTimes(1)

    analysisStore.updatedAtUnixMs = 1_700_000_000_000
    await flushPanel()

    expect(refreshHeatmapFromWorkflowRunMock).toHaveBeenCalledTimes(1)
  })

  it('keeps an explicit Review Top results handoff', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const button = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((candidate) =>
      String(candidate.textContent || '').includes('Review Top results'),
    )
    button?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()

    expect(workflowStore.resultsTab).toBe('top')
  })

  it('routes display-mode changes through the analysis store selector action and updates the live copy contract', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    analysisStore.heatmapMode = 'objective'
    analysisStore.heatmapModeOptions = [
      { value: 'objective', label: 'Objective score', disabled: false },
      { value: 'plateau', label: 'Derived plateau', disabled: false },
    ]
    const setSelectFieldSpy = vi.spyOn(analysisStore, 'setSelectField')

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const select = document.querySelector<HTMLSelectElement>('#vueHeatmapMode')
    expect(select).not.toBeNull()
    select!.value = 'plateau'
    select!.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()

    expect(setSelectFieldSpy).toHaveBeenCalledWith('heatmapMode', 'plateau')
    expect(document.body.textContent || '').toContain('supportive 8-neighbor agreement')
    expect(document.body.textContent || '').toContain('representative parameter pair')
    expect(document.querySelector('#heatmapCanvasStackVue')?.getAttribute('aria-label') || '').toContain('Derived plateau heatmap')
  })

  it('explains the plateau score as a derived robustness view', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    analysisStore.heatmapMode = 'plateau'

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.body.textContent || '').toContain('supportive 8-neighbor agreement')
    expect(document.body.textContent || '').toContain('bin-consistency')
    expect(document.body.textContent || '').toContain('representative parameter pair')
    expect(document.body.textContent || '').toContain('exact retained-candidate path')
  })
})
