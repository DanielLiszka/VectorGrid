// @vitest-environment jsdom
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
    const exportButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((candidate) =>
      String(candidate.textContent || '').includes('Export PNG'),
    )
    expect(exportButton?.disabled).toBe(false)
    exportButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()
    expect(exportSpy).toHaveBeenCalledTimes(1)
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
})
