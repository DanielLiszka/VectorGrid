// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ValidationWorkspacePanel from './ValidationWorkspacePanel.vue'
import { bindRuntimePinia } from '../stores/runtimePinia'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'

const virtualizerState = vi.hoisted(() => ({
  virtualItems: [
    { index: 1, start: 28, end: 56 },
    { index: 2, start: 56, end: 84 },
  ] as Array<{ index: number; start: number; end: number }>,
  totalSize: 112,
  scrollCalls: [] as number[],
}))

vi.mock('@tanstack/vue-virtual', () => ({
  useVirtualizer: () => ({
    value: {
      getVirtualItems: () => virtualizerState.virtualItems,
      getTotalSize: () => virtualizerState.totalSize,
      scrollToOffset: (offset: number) => {
        virtualizerState.scrollCalls.push(offset)
      },
      measure: vi.fn(),
    },
  }),
}))

vi.mock('@vueuse/core', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@vueuse/core')>()
  return {
    ...actual,
    useResizeObserver: () => ({
      stop: vi.fn(),
    }),
  }
})

function installValidationVirtualizationMountShell(): void {
  document.body.innerHTML = `
    <section id="shellResultsColumn" class="resultsColumn">
      <div id="results_panel_validate" class="panelHost" data-vue-panel="validate">
        <div id="validationVueShellMount"></div>
      </div>
    </section>
  `
}

describe('ValidationWorkspacePanel virtualization', () => {
  beforeEach(() => {
    localStorage.clear()
    installValidationVirtualizationMountShell()
    virtualizerState.scrollCalls = []
  })

  it('renders the current validation result window inside the owned table viewport', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    bindRuntimePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const dataStore = useDataStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const validationStore = useValidationStore(pinia)

    workflowStore.selectResultsTab('validate', { persist: false })
    dataStore.dataId = 'dataset_validation_virtualized'
    dataStore.datasetLabel = 'AAPL 1m'
    analysisStore.topRows = 4
    validationStore.tableVisible = true
    validationStore.tableRowCount = 4
    validationStore.validationColumns = [
      { label: 'Route', alignLeft: true },
      { label: 'Verdict', alignLeft: true },
      { label: 'Sharpe', alignLeft: false },
    ]
    validationStore.validationRows = [
      {
        cells: [
          { text: 'route 1', className: 'left', title: 'route 1' },
          { text: 'reject', className: 'left', title: 'reject' },
          { text: '0.70', className: 'mono', title: '0.70' },
        ],
      },
      {
        cells: [
          { text: 'route 2', className: 'left', title: 'route 2' },
          { text: 'watch', className: 'left', title: 'watch' },
          { text: '0.92', className: 'mono', title: '0.92' },
        ],
      },
      {
        cells: [
          { text: 'route 3', className: 'left', title: 'route 3' },
          { text: 'pass', className: 'left', title: 'pass' },
          { text: '1.08', className: 'mono', title: '1.08' },
        ],
      },
      {
        cells: [
          { text: 'route 4', className: 'left', title: 'route 4' },
          { text: 'hold', className: 'left', title: 'hold' },
          { text: '0.64', className: 'mono', title: '0.64' },
        ],
      },
    ]

    mount(ValidationWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const scrollHost = document.querySelector<HTMLElement>('[aria-label="Validation results table"]')
    expect(scrollHost?.classList.contains('virtualTableViewport')).toBe(true)

    const spacerRows = Array.from(document.querySelectorAll<HTMLTableRowElement>('.validationTableVue tbody tr.spacerRow'))
    expect(spacerRows).toHaveLength(2)
    expect(spacerRows.map((row) => row.querySelector('td')?.getAttribute('style'))).toEqual([
      'height: 28px;',
      'height: 28px;',
    ])

    const table = document.querySelector('.validationTableVue')
    expect(table?.textContent || '').toContain('route 2')
    expect(table?.textContent || '').toContain('route 3')
    expect(table?.textContent || '').not.toContain('route 1')
    expect(table?.textContent || '').not.toContain('route 4')
    expect(virtualizerState.scrollCalls).toEqual([0])
  })
})
