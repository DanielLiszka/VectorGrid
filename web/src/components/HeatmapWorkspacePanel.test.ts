// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import HeatmapWorkspacePanel from './HeatmapWorkspacePanel.vue'
import { useWorkflowStore } from '../stores/workflowStore'

vi.mock('../services/analysisService', () => ({
  refreshHeatmapFromWorkflowRun: vi.fn(),
  restoreHeatmapVisible: vi.fn(() => false),
}))

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
