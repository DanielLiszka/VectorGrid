// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import MetaWorkspacePanel from './MetaWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useHistoryStore } from '../stores/historyStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { META_RESULTS_TABLE_STATE_KEY } from '../utils/core/storageKeys'

function installMetaPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="meta"></section>
    <div id="metaVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('MetaWorkspacePanel', () => {
  beforeEach(() => {
    installMetaPanelMountShell()
    localStorage.clear()
  })

  it('persists visible meta result columns and restores defaults', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const historyStore = useHistoryStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflow.selectResultsTab('meta', { persist: false })
    workflow.selectAggregateTab('meta', { persist: false })
    historyStore.savedRunCount = 1
    analysisStore.metaTableColumns = [
      { label: 'Group', alignLeft: true },
      { label: 'Selection', alignLeft: true },
      { label: 'Score', alignLeft: false },
    ]
    analysisStore.metaTableRows = [
      {
        cells: [
          { text: 'Momentum', className: 'left', title: 'Momentum' },
          { text: 'Candidate A', className: 'left', title: 'Candidate A' },
          { text: '0.88', className: '', title: '0.88' },
        ],
      },
    ]
    analysisStore.metaOutputPreview = '{"summary":"ok"}'

    const wrapper = mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const scoreToggle = document.querySelector<HTMLInputElement>('#metaColumnToggle_col_2')
    expect(scoreToggle).not.toBeNull()
    scoreToggle!.checked = false
    scoreToggle!.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()

    const headerText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(headerText).toEqual(['Group', 'Selection'])
    expect(localStorage.getItem(META_RESULTS_TABLE_STATE_KEY)).toContain('"visibleColumnKeys":["col_0","col_1"]')

    wrapper.unmount()

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const remountHeaderText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(remountHeaderText).toEqual(['Group', 'Selection'])

    const restoreButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Restore defaults'),
    )
    restoreButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()

    const restoredHeaderText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(restoredHeaderText).toEqual(['Group', 'Selection', 'Score'])
    expect(localStorage.getItem(META_RESULTS_TABLE_STATE_KEY)).toBeNull()
  })

  it('reuses the shared contextual action menu for saved aggregate artifacts', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const historyStore = useHistoryStore(pinia)
    const analysisStore = useAnalysisStore(pinia)

    workflow.selectResultsTab('meta', { persist: false })
    workflow.selectAggregateTab('saved', { persist: false })
    historyStore.savedRunCount = 1

    analysisStore.metaSavedOptions = [
      { value: 'meta-a', label: 'meta-a', disabled: false },
    ]
    analysisStore.metaSavedSelected = 'meta-a'
    analysisStore.canLoadMetaSaved = true
    analysisStore.canRefreshMetaSaved = true
    analysisStore.canOpenMetaSavedReport = true
    analysisStore.canRevealMetaSaved = true
    analysisStore.canDeleteMetaSaved = true

    analysisStore.stage4SavedOptions = [
      { value: 'stage5-a', label: 'stage5-a', disabled: false },
    ]
    analysisStore.stage4SavedSelected = 'stage5-a'
    analysisStore.canLoadStage4Saved = true
    analysisStore.canRefreshStage4Saved = true
    analysisStore.canRevealStage4Saved = true
    analysisStore.canDeleteStage4Saved = true

    analysisStore.policyEvalSavedOptions = [
      { value: 'policy-a', label: 'policy-a', disabled: false },
    ]
    analysisStore.policyEvalSavedSelected = 'policy-a'
    analysisStore.canLoadPolicyEvalSaved = true
    analysisStore.canRefreshPolicyEvalSaved = true
    analysisStore.canRevealPolicyEvalSaved = true
    analysisStore.canDeletePolicyEvalSaved = true

    const openSavedReportSpy = vi
      .spyOn(analysisStore, 'openSavedReport')
      .mockImplementation(() => {})

    mount(MetaWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const menuTriggers = Array.from(document.querySelectorAll<HTMLButtonElement>('button.contextActionMenuTrigger'))
    expect(menuTriggers).toHaveLength(3)

    const loadButtons = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).filter((button) =>
      String(button.textContent || '').trim() === 'Load',
    )
    expect(loadButtons).toHaveLength(3)

    menuTriggers[0]?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()

    const reportItem = Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]')).find(
      (item) => String(item.textContent || '').trim() === 'Open report',
    )
    expect(reportItem).toBeTruthy()

    reportItem?.click()
    await flushPanel()

    expect(openSavedReportSpy).toHaveBeenCalledTimes(1)
    expect(document.activeElement).toBe(menuTriggers[0])
  })
})
