// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import DrillWorkspacePanel from './DrillWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'

function installDrillPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="drill">
      <div id="drillVueShellMount"></div>
    </section>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('DrillWorkspacePanel', () => {
  beforeEach(() => {
    installDrillPanelMountShell()
    localStorage.clear()
  })

  it('rerenders on visible mount and keeps the drill evidence surface visible to contributors', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    analysisStore.statusLine = 'Drilldown ready.'
    const rerenderSpy = vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)

    mount(DrillWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(rerenderSpy).toHaveBeenCalled()
    expect(text).toContain('Price + trades')
    expect(text).toContain('Stability')
    expect(text).toContain('Trades')
    expect(text).toContain('If the charts stay empty')
    expect(text).toContain('Drilldown ready.')
    expect(document.querySelector('#tradeTableBodyVue')).not.toBeNull()
    expect(document.querySelector('#stabilityTableBodyVue')).not.toBeNull()
  })

  it('keeps an explicit Review Top results handoff', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('drill', { persist: false })
    vi.spyOn(analysisStore, 'rerenderDrill').mockImplementation(() => undefined)

    mount(DrillWorkspacePanel, {
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
    expect(document.querySelector("[data-vue-panel='drill']")?.getAttribute('data-vue-drill')).toBe('1')
  })
})
