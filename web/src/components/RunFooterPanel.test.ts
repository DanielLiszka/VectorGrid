// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import RunFooterPanel from './RunFooterPanel.vue'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useWorkflowStore } from '../stores/workflowStore'

function installFooterMountShell(): void {
  document.body.innerHTML = `
    <footer id="runFooterShell">
      <div id="runFooterVueShellMount"></div>
    </footer>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('RunFooterPanel', () => {
  beforeEach(() => {
    installFooterMountShell()
    localStorage.clear()
  })

  it('keeps the legacy component name as an ActionStatusBar wrapper', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    useWorkflowStore(pinia).selectWorkflowStep('optimize', { persist: false })
    useOptimizeStore(pinia).runDisabled = false

    const wrapper = mount(RunFooterPanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.querySelector('[aria-label="Action status bar"]')).not.toBeNull()
    expect(document.body.textContent || '').toContain('Execution readiness')
    expect(
      Array.from(document.querySelectorAll('button')).some(
        (button) => button.textContent?.trim() === 'Run optimization',
      ),
    ).toBe(true)

    wrapper.unmount()
  })
})
