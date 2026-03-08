// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import TopWorkspacePanel from './TopWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'

function installTopPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="top"></section>
    <div id="topVueShellMount"></div>
  `
}

async function waitForInput(id: string): Promise<HTMLInputElement> {
  for (let i = 0; i < 6; i += 1) {
    await nextTick()
    const found = document.querySelector<HTMLInputElement>(`#${id}`)
    if (found) return found
  }
  throw new Error(`Expected input #${id} to be present.`)
}

describe('TopWorkspacePanel', () => {
  beforeEach(() => {
    localStorage.clear()
    installTopPanelMountShell()
  })

  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('debounces repeated filter input updates into one store write', async () => {
    vi.useFakeTimers()
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })

    const setInputSpy = vi.spyOn(analysisStore, 'setInputField')
    const wrapper = mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    const input = await waitForInput('vueFilterMinPnl')
    input.value = '1'
    input.dispatchEvent(new Event('input', { bubbles: true }))
    input.value = '12'
    input.dispatchEvent(new Event('input', { bubbles: true }))
    input.value = '123'
    input.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    expect(setInputSpy).toHaveBeenCalledTimes(0)

    vi.advanceTimersByTime(129)
    await nextTick()
    expect(setInputSpy).toHaveBeenCalledTimes(0)

    vi.advanceTimersByTime(1)
    await nextTick()
    expect(setInputSpy).toHaveBeenCalledTimes(1)
    expect(setInputSpy).toHaveBeenLastCalledWith('filterMinPnl', '123')

    wrapper.unmount()
  })

  it('flushes queued filter updates on unmount', async () => {
    vi.useFakeTimers()
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })

    const setInputSpy = vi.spyOn(analysisStore, 'setInputField')
    const wrapper = mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    const input = await waitForInput('vueFilterMinSharpe')
    input.value = '2.5'
    input.dispatchEvent(new Event('input', { bubbles: true }))
    await nextTick()
    expect(setInputSpy).toHaveBeenCalledTimes(0)

    wrapper.unmount()
    expect(setInputSpy).toHaveBeenCalledTimes(1)
    expect(setInputSpy).toHaveBeenLastCalledWith('filterMinSharpe', '2.5')
  })
})
