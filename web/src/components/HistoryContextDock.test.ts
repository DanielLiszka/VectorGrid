import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import HistoryContextDock from './HistoryContextDock.vue'
import { useHistoryStore } from '../stores/historyStore'
import { useWorkflowStore } from '../stores/workflowStore'

describe('HistoryContextDock', () => {
  it('shows library summary state and routes dock actions through the history store', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)

    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('library')

    const historyStore = useHistoryStore(pinia)
    historyStore.applyLegacySnapshot({
      sessionRunCount: 2,
      sessionSelectedLabel: 'Latest session run',
      savedRunCount: 4,
      savedSelectedValue: 'run_1',
      savedSelectedLabel: 'Saved run 1',
      compareRunCount: 1,
      savedRunsMeta: '4 saved runs',
      savedRunDetailTitle: 'Saved run detail',
      savedRunDetailMeta: 'Ready to reopen',
      statusLine: 'Ready.',
    })

    const applySessionSpy = vi.spyOn(historyStore, 'applySession').mockImplementation(() => {})
    const refreshSpy = vi.spyOn(historyStore, 'refreshSavedRuns').mockImplementation(() => {})
    const loadSpy = vi.spyOn(historyStore, 'loadSavedRun').mockImplementation(() => {})
    const applySavedSpy = vi.spyOn(historyStore, 'applySavedRun').mockImplementation(() => {})
    const clearSpy = vi.spyOn(historyStore, 'clearSession').mockImplementation(() => {})
    const openRunsDirSpy = vi.spyOn(historyStore, 'openRunsDir').mockImplementation(() => {})

    const wrapper = mount(HistoryContextDock, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()

    expect(wrapper.text()).toContain('History Context')
    expect(wrapper.text()).toContain('Latest session run')
    expect(wrapper.text()).toContain('Saved run 1')

    const buttons = wrapper.findAll('button')
    for (const button of buttons) await button.trigger('click')

    expect(applySessionSpy).toHaveBeenCalledTimes(1)
    expect(refreshSpy).toHaveBeenCalledTimes(1)
    expect(loadSpy).toHaveBeenCalledWith('run_1')
    expect(applySavedSpy).toHaveBeenCalledWith('run_1')
    expect(clearSpy).toHaveBeenCalledTimes(1)
    expect(openRunsDirSpy).toHaveBeenCalledTimes(1)
  })
})
