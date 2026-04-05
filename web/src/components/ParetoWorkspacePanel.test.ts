// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ParetoWorkspacePanel from './ParetoWorkspacePanel.vue'
import { useWorkflowStore } from '../stores/workflowStore'

vi.mock('../services/analysisService', () => ({
  refreshParetoFromWorkflowRun: vi.fn(),
}))

function installParetoPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="pareto"></section>
    <div id="paretoVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('ParetoWorkspacePanel', () => {
  beforeEach(() => {
    installParetoPanelMountShell()
    localStorage.clear()
  })

  it('keeps an explicit Review Top results handoff', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('pareto', { persist: false })

    mount(ParetoWorkspacePanel, {
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
