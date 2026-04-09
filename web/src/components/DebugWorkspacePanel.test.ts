// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import DebugWorkspacePanel from './DebugWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

function installDebugMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="debug">
      <div id="debugVueShellMount"></div>
    </section>
  `
}

describe('DebugWorkspacePanel', () => {
  beforeEach(() => {
    installDebugMountShell()
    localStorage.clear()
  })

  it('renders raw analysis output in the debug workspace posture', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('debug', { persist: false })
    analysisStore.debugOutputPreview = JSON.stringify({ selected: 'candidate_a', pnl: 1.23 }, null, 2)
    analysisStore.statusLine = 'Debug payload refreshed.'

    mount(DebugWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('Debug JSON')
    expect(text).toContain('candidate_a')
    expect(text).toContain('Debug payload refreshed.')
  })

  it('keeps the debug panel hidden outside the debug results posture', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('overview', { persist: false })

    mount(DebugWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.querySelector<HTMLElement>('.debugPanelVueRoot')?.style.display).toBe('none')
  })
})
