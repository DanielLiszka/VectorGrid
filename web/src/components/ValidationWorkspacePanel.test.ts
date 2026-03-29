// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import ValidationWorkspacePanel from './ValidationWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { VALIDATION_RESULTS_TABLE_STATE_KEY } from '../utils/core/storageKeys'

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

  it('keeps the primary validation action inline in the panel', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    workflow.selectResultsTab('validate', { persist: false })

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.body.textContent || '').toContain('Run validation')
    const runButtons = Array.from(document.querySelectorAll('button')).filter(
      (button) => button.textContent?.trim() === 'Run validation',
    )
    expect(runButtons).toHaveLength(1)

    wrapper.unmount()
  })

  it('persists visible validation result columns and restores defaults', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const dataStore = useDataStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const validationStore = useValidationStore(pinia)
    workflow.selectResultsTab('validate', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 1
    validationStore.tableRowCount = 1
    validationStore.tableVisible = true
    validationStore.validationColumns = [
      { label: 'Route', alignLeft: true },
      { label: 'Warning', alignLeft: true },
      { label: 'Score', alignLeft: false },
    ]
    validationStore.validationRows = [
      {
        cells: [
          { text: 'GpuKernel', className: 'left', title: 'GpuKernel' },
          { text: 'Stable', className: 'left', title: 'Stable' },
          { text: '0.92', className: '', title: '0.92' },
        ],
      },
    ]

    const wrapper = mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const scoreToggle = document.querySelector<HTMLInputElement>('#validationColumnToggle_col_2')
    expect(scoreToggle).not.toBeNull()
    scoreToggle!.checked = false
    scoreToggle!.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()

    const headerText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(headerText).toEqual(['Route', 'Warning'])
    expect(localStorage.getItem(VALIDATION_RESULTS_TABLE_STATE_KEY)).toContain('"visibleColumnKeys":["col_0","col_1"]')

    wrapper.unmount()

    mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const remountHeaderText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(remountHeaderText).toEqual(['Route', 'Warning'])

    const restoreButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Restore defaults'),
    )
    restoreButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()

    const restoredHeaderText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(restoredHeaderText).toEqual(['Route', 'Warning', 'Score'])
    expect(localStorage.getItem(VALIDATION_RESULTS_TABLE_STATE_KEY)).toBeNull()
  })
})
