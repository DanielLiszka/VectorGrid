// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import ValidationWorkspacePanel from './ValidationWorkspacePanel.vue'
import { useWorkflowStore } from '../stores/workflowStore'

function installValidationPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="validate"></section>
    <div id="validationVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('ValidationWorkspacePanel', () => {
  beforeEach(() => {
    installValidationPanelMountShell()
    localStorage.clear()
  })

  it('does not duplicate the primary run action inline', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    workflow.selectResultsTab('validate', { persist: false })

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.body.textContent || '').toContain('Run validation from the sticky footer')
    const runButtons = Array.from(document.querySelectorAll('button')).filter(
      (button) => button.textContent?.trim() === 'Run validation',
    )
    expect(runButtons).toHaveLength(0)

    wrapper.unmount()
  })
})
