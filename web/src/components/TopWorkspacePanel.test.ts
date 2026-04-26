import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { computed, nextTick, ref } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import TopWorkspacePanel from './TopWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { TOP_RESULTS_TABLE_STATE_KEY } from '../utils/core/storageKeys'

const virtualTableState = vi.hoisted(() => ({
  visibleIndexes: null as number[] | null,
  topSpacerPx: 0,
  bottomSpacerPx: 0,
  scrollToTop: vi.fn(),
}))

vi.mock('../composables/useVirtualTableRows', () => ({
  useVirtualTableRows: (rows: { value: unknown[] }) => ({
    scrollElementRef: ref(null),
    virtualRows: computed(() => {
      const source = rows.value
      const indexes = virtualTableState.visibleIndexes ?? source.map((_, index) => index)
      return indexes
        .map((index) => ({ index, row: source[index] }))
        .filter((entry) => entry.row !== undefined)
    }),
    topSpacerPx: computed(() => virtualTableState.topSpacerPx),
    bottomSpacerPx: computed(() => virtualTableState.bottomSpacerPx),
    scrollToTop: virtualTableState.scrollToTop,
  }),
}))

function installTopPanelMountShell(): void {
  document.body.innerHTML = `
    <section id="shellResultsColumn" class="resultsColumn">
      <div id="results_panel_top" class="panelHost" data-vue-panel="top">
        <div id="topVueShellMount"></div>
      </div>
    </section>
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
    virtualTableState.visibleIndexes = null
    virtualTableState.topSpacerPx = 0
    virtualTableState.bottomSpacerPx = 0
    virtualTableState.scrollToTop.mockReset()
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
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 1

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
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 1

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

  it('supports arrow-key row navigation across ranked results', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 3
    analysisStore.topColumns = [
      { key: 'name', label: 'Name', sort: '', alignLeft: true },
    ]
    analysisStore.topTableRows = [
      {
        selected: true,
        cells: [{ text: 'row 1', className: 'left', title: 'row 1' }],
      },
      {
        selected: false,
        cells: [{ text: 'row 2', className: 'left', title: 'row 2' }],
      },
      {
        selected: false,
        cells: [{ text: 'row 3', className: 'left', title: 'row 3' }],
      },
    ]

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const rows = Array.from(document.querySelectorAll<HTMLElement>('tr[data-top-row="1"]'))
    expect(rows).toHaveLength(3)

    rows[0]?.focus()
    rows[0]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }))
    expect(document.activeElement).toBe(rows[1])

    rows[1]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true }))
    expect(document.activeElement).toBe(rows[2])

    rows[2]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', bubbles: true }))
    expect(document.activeElement).toBe(rows[0])
  })

  it('renders only the virtual window rows while preserving spacer and selection cues', async () => {
    virtualTableState.visibleIndexes = [1, 3]
    virtualTableState.topSpacerPx = 56
    virtualTableState.bottomSpacerPx = 84

    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 4
    analysisStore.topColumns = [{ key: 'name', label: 'Name', sort: '', alignLeft: true }]
    analysisStore.topTableRows = [
      { selected: false, cells: [{ text: 'row 1', className: 'left', title: 'row 1' }] },
      { selected: false, cells: [{ text: 'row 2', className: 'left', title: 'row 2' }] },
      { selected: false, cells: [{ text: 'row 3', className: 'left', title: 'row 3' }] },
      { selected: true, cells: [{ text: 'row 4', className: 'left', title: 'row 4' }] },
    ]

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const spacerRows = Array.from(document.querySelectorAll<HTMLTableRowElement>('tr.spacerRow'))
    expect(spacerRows).toHaveLength(2)
    expect(spacerRows[0]?.querySelector('td')?.getAttribute('style')).toContain('56px')
    expect(spacerRows[1]?.querySelector('td')?.getAttribute('style')).toContain('84px')

    const tableWrap = document.querySelector<HTMLElement>('[aria-label="Top optimization results table"]')
    expect(tableWrap?.classList.contains('virtualTableViewport')).toBe(true)

    const renderedRows = Array.from(document.querySelectorAll<HTMLTableRowElement>('tbody tr:not(.spacerRow)'))
    expect(renderedRows).toHaveLength(2)
    expect(renderedRows[0]?.textContent).toContain('row 2')
    expect(renderedRows[1]?.textContent).toContain('row 4')
    expect(renderedRows[0]?.textContent).not.toContain('row 1')
    expect(renderedRows[1]?.getAttribute('aria-selected')).toBe('true')

    const pinButtons = Array.from(document.querySelectorAll<HTMLButtonElement>('button.pinCellBtn'))
    expect(pinButtons.map((button) => button.getAttribute('aria-label'))).toEqual([
      'Pin candidate 2',
      'Pin candidate 4',
    ])
  })

  it('announces the selected ranked candidate through a live status region', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 2
    analysisStore.topColumns = [
      { key: 'name', label: 'Name', sort: '', alignLeft: true },
      { key: 'score', label: 'Score', sort: '', alignLeft: false },
    ]
    analysisStore.topTableRows = [
      {
        selected: false,
        cells: [
          { text: 'row 1', className: 'left', title: 'row 1' },
          { text: '1.2', className: '', title: '1.2' },
        ],
      },
      {
        selected: true,
        cells: [
          { text: 'row 2', className: 'left', title: 'row 2' },
          { text: '3.4', className: '', title: '3.4' },
        ],
      },
    ]

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const selectionStatus = document.querySelector<HTMLElement>('#topSelectionStatus')
    expect(selectionStatus?.textContent).toContain('Selected candidate 2 of 2')
    expect(selectionStatus?.textContent).toContain('row 2')
    expect(selectionStatus?.textContent).toContain('3.4')
  })

  it('opens drilldown when a ranked row is selected from the table', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 2
    analysisStore.topColumns = [{ key: 'name', label: 'Name', sort: '', alignLeft: true }]
    analysisStore.topTableRows = [
      { selected: false, cells: [{ text: 'row 1', className: 'left', title: 'row 1' }] },
      { selected: false, cells: [{ text: 'row 2', className: 'left', title: 'row 2' }] },
    ]
    const selectRowSpy = vi.spyOn(analysisStore, 'selectTopRow').mockImplementation(() => undefined)
    const resultsSpy = vi.spyOn(workflowStore, 'selectResultsTab')

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const rows = Array.from(document.querySelectorAll<HTMLTableRowElement>('tr[data-top-row="1"]'))
    rows[1]?.dispatchEvent(new MouseEvent('click', { bubbles: true }))

    expect(resultsSpy).toHaveBeenCalledWith('drill')
    expect(selectRowSpy).toHaveBeenCalledWith(1)
  })

  it('pins shortlist candidates into the compare tray', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 2
    analysisStore.topColumns = [{ key: 'name', label: 'Name', sort: '', alignLeft: true }]
    analysisStore.topTableRows = [
      { selected: true, cells: [{ text: 'row 1', className: 'left', title: 'row 1' }] },
      { selected: false, cells: [{ text: 'row 2', className: 'left', title: 'row 2' }] },
    ]
    const pinSelectedSpy = vi.spyOn(analysisStore, 'pinSelectedTopRow').mockImplementation(() => undefined)

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const pinButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Pin current candidate'),
    )
    pinButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()

    expect(pinSelectedSpy).toHaveBeenCalled()
  })

  it('renders compare actions for pinned candidates', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 1
    analysisStore.topColumns = [{ key: 'name', label: 'Name', sort: '', alignLeft: true }]
    analysisStore.topTableRows = [
      { selected: true, cells: [{ text: 'row 1', className: 'left', title: 'row 1' }] },
    ]
    analysisStore.pinnedTopCandidates = [
      {
        id: 'candidate_a',
        rank: 1,
        title: 'Rank #1 · row 1',
        summary: 'Return 12% · Sharpe 1.8',
        detail: 'Strategy DoubleMa · Objective Sharpe',
        badges: ['Validation stale', 'Filtered shortlist'],
        staleReason: 'Hidden by the current shortlist filters.',
      },
    ]
    const replaceSpy = vi.spyOn(analysisStore, 'replacePinnedTopCandidate').mockImplementation(() => undefined)
    const removeSpy = vi.spyOn(analysisStore, 'unpinTopCandidate').mockImplementation(() => undefined)

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Pinned candidates')
    expect(text).toContain('Filtered shortlist')
    expect(text).toContain('Hidden by the current shortlist filters.')
    expect(document.querySelectorAll('img.actionIcon').length).toBeGreaterThanOrEqual(2)
    expect(document.querySelector('img.chipIcon')).not.toBeNull()

    const replaceButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Replace with current'),
    )
    const removeButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Remove'),
    )

    replaceButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    removeButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()
    expect(replaceSpy).toHaveBeenCalledWith('candidate_a')
    expect(removeSpy).toHaveBeenCalledWith('candidate_a')
  })

  it('renders stale cues from deterministic store state', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 1
    analysisStore.topColumns = [{ key: 'name', label: 'Name', sort: '', alignLeft: true }]
    analysisStore.topTableRows = [
      { selected: true, cells: [{ text: 'row 1', className: 'left', title: 'row 1' }] },
    ]
    analysisStore.topStaleCues = [
      'The prior selection fell out of the current shortlist filters.',
      'Hidden by the current shortlist filters.',
    ]

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('The prior selection fell out of the current shortlist filters.')
    expect(text).toContain('Hidden by the current shortlist filters.')
  })

  it('dispatches pin actions from shortlist row shortcuts', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 2
    analysisStore.topColumns = [{ key: 'name', label: 'Name', sort: '', alignLeft: true }]
    analysisStore.topTableRows = [
      { selected: true, cells: [{ text: 'row 1', className: 'left', title: 'row 1' }] },
      { selected: false, cells: [{ text: 'row 2', className: 'left', title: 'row 2' }] },
    ]
    const pinSpy = vi.spyOn(analysisStore, 'pinTopRow').mockImplementation(() => undefined)

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const rows = Array.from(document.querySelectorAll<HTMLElement>('tr[data-top-row="1"]'))
    rows[1]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'p', bubbles: true }))
    expect(pinSpy).toHaveBeenCalledWith(1)
  })

  it('persists visible top-result columns without disturbing the selection summary', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 2
    analysisStore.topColumns = [
      { key: 'rank', label: '#', sort: '', alignLeft: true },
      { key: 'name', label: 'Name', sort: '', alignLeft: true },
      { key: 'pnl', label: 'PnL', sort: '', alignLeft: false },
    ]
    analysisStore.topTableRows = [
      {
        rowId: 'candidate_a',
        selected: false,
        cells: [
          { text: '1', className: 'left', title: '1' },
          { text: 'row 1', className: 'left', title: 'row 1' },
          { text: '1.2', className: '', title: '1.2' },
        ],
      },
      {
        rowId: 'candidate_b',
        selected: true,
        cells: [
          { text: '2', className: 'left', title: '2' },
          { text: 'row 2', className: 'left', title: 'row 2' },
          { text: '3.4', className: '', title: '3.4' },
        ],
      },
    ]

    const wrapper = mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const pnlToggle = document.querySelector<HTMLInputElement>('#topColumnToggle_pnl')
    expect(pnlToggle).not.toBeNull()
    pnlToggle!.checked = false
    pnlToggle!.dispatchEvent(new Event('change', { bubbles: true }))
    await nextTick()
    await nextTick()

    const headerText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(headerText).toEqual(['#', 'Name'])
    const selectionStatus = document.querySelector<HTMLElement>('#topSelectionStatus')
    expect(selectionStatus?.textContent).toContain('Selected candidate 2 of 2')
    expect(selectionStatus?.textContent).toContain('row 2')
    expect(selectionStatus?.textContent).toContain('3.4')
    expect(localStorage.getItem(TOP_RESULTS_TABLE_STATE_KEY)).toContain('"visibleColumnKeys":["rank","name"]')

    wrapper.unmount()

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const remountHeaderText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(remountHeaderText).toEqual(['#', 'Name'])

    const restoreButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Restore defaults'),
    )
    restoreButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()
    await nextTick()

    const restoredHeaderText = Array.from(document.querySelectorAll('th')).map((cell) =>
      String(cell.textContent || '').trim(),
    )
    expect(restoredHeaderText).toEqual(['#', 'Name', 'PnL'])
    expect(localStorage.getItem(TOP_RESULTS_TABLE_STATE_KEY)).toBeNull()
  })

  it('keeps the row pin affordance on the first visible cell when earlier columns are hidden', async () => {
    localStorage.setItem(
      TOP_RESULTS_TABLE_STATE_KEY,
      JSON.stringify({
        visibleColumnKeys: ['name', 'score'],
        knownColumnKeys: ['rank', 'name', 'score'],
      }),
    )
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 1
    analysisStore.topColumns = [
      { key: 'rank', label: '#', sort: '', alignLeft: true },
      { key: 'name', label: 'Name', sort: '', alignLeft: true },
      { key: 'score', label: 'Score', sort: '', alignLeft: false },
    ]
    analysisStore.topTableRows = [
      {
        rowId: 'candidate_a',
        selected: true,
        cells: [
          { text: '1', className: 'left', title: '1' },
          { text: 'row 1', className: 'left', title: 'row 1' },
          { text: '98', className: '', title: '98' },
        ],
      },
    ]
    const pinSpy = vi.spyOn(analysisStore, 'pinTopRow').mockImplementation(() => undefined)

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const firstRow = document.querySelector<HTMLElement>('tr[data-top-row="1"]')
    const firstCell = firstRow?.querySelector<HTMLTableCellElement>('td')
    const pinButton = firstCell?.querySelector<HTMLButtonElement>('button.pinCellBtn')
    expect(firstCell?.textContent).toContain('row 1')
    expect(pinButton?.textContent).toContain('Pin')

    pinButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    expect(pinSpy).toHaveBeenCalledWith(0)
  })

  it('applies built-in top table presets without misaligning row cells', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 1
    analysisStore.topColumns = [
      { key: 'rank', label: '#', sort: '', alignLeft: true },
      { key: 'name', label: 'Name', sort: '', alignLeft: true },
      { key: 'pnl', label: 'PnL', sort: '', alignLeft: false },
      { key: 'sharpe', label: 'Sharpe', sort: '', alignLeft: false },
      { key: 'max_dd', label: 'MaxDD', sort: '', alignLeft: false },
      { key: 'trades', label: 'Trades', sort: '', alignLeft: false },
      { key: 'exposure', label: 'Expo', sort: '', alignLeft: false },
    ]
    analysisStore.topTableRows = [
      {
        rowId: 'candidate_a',
        selected: true,
        cells: [
          { text: '1', className: 'left', title: '1' },
          { text: 'row 1', className: 'left', title: 'row 1' },
          { text: '100', className: '', title: '100' },
          { text: '2.1', className: '', title: '2.1' },
          { text: '-5', className: '', title: '-5' },
          { text: '8', className: '', title: '8' },
          { text: '0.44', className: '', title: '0.44' },
        ],
      },
    ]

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    document.querySelector<HTMLButtonElement>('[data-qa="top.table.view.risk"]')?.click()
    await nextTick()
    await nextTick()

    expect(Array.from(document.querySelectorAll('th')).map((cell) => String(cell.textContent || '').trim())).toEqual([
      '#',
      'PnL',
      'Sharpe',
      'MaxDD',
      'Trades',
      'Expo',
    ])
    expect(Array.from(document.querySelectorAll('tbody td')).map((cell) => String(cell.textContent || '').trim())).toEqual([
      '1Pin',
      '100',
      '2.1',
      '-5',
      '8',
      '0.44',
    ])
    expect(localStorage.getItem(TOP_RESULTS_TABLE_STATE_KEY)).toContain('"columnOrderKeys":["rank","pnl","sharpe","max_dd","trades","exposure","name"]')
  })

  it('routes current-candidate actions into drilldown and overview while keeping compare below the ranked table', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.topRows = 1
    analysisStore.topColumns = [{ key: 'name', label: 'Name', sort: '', alignLeft: true }]
    analysisStore.topTableRows = [
      {
        rowId: 'candidate_a',
        selected: true,
        cells: [{ text: 'row 1', className: 'left', title: 'row 1' }],
      },
    ]
    analysisStore.pinnedTopCandidates = [
      {
        id: 'candidate_a',
        rank: 1,
        title: 'Rank #1 · row 1',
        summary: 'Return 12% · Sharpe 1.8',
        detail: 'Strategy DoubleMa · Objective Sharpe',
        badges: ['Current leader'],
        staleReason: '',
      },
    ]

    const resultsSpy = vi.spyOn(workflowStore, 'selectResultsTab')

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const tableWrap = document.querySelector('.tableWrap')
    const compareTray = document.querySelector('.compareTray')
    expect(tableWrap).not.toBeNull()
    expect(compareTray).not.toBeNull()
    if (!tableWrap || !compareTray) {
      throw new Error('Expected Top table and compare tray to be rendered.')
    }
    expect(
      Boolean(tableWrap.compareDocumentPosition(compareTray) & Node.DOCUMENT_POSITION_FOLLOWING),
    ).toBe(true)

    const openDrilldownButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Open Drilldown'),
    )
    const reviewOverviewButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Review Overview'),
    )

    openDrilldownButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    reviewOverviewButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))

    expect(resultsSpy).toHaveBeenCalledWith('drill')
    expect(resultsSpy).toHaveBeenCalledWith('overview')
  })

  it('surfaces the winner-only empty state when the run disables top-k output', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.bestTitle = 'Rank #1 · Winner'
    analysisStore.topRows = 0
    analysisStore.topFilterMeta = 'Top k disabled by current output settings.'

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('This run returned only the best winner.')
    expect(text).toContain('Increase Advanced > Top K above 0 and rerun optimization')
    expect(text).toContain('Top k disabled by current output settings.')
  })

  it('offers a clear-filter recovery path when active filters empty the shortlist', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectResultsTab('top', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.bestTitle = 'Rank #1 · Winner'
    analysisStore.topRows = 0
    analysisStore.filterMinPnl = '1.5'
    analysisStore.canClearTopFilters = true
    analysisStore.topSelectionGuardrail = 'The prior selection fell out of the filtered shortlist.'
    const clearSpy = vi.spyOn(analysisStore, 'clearTopFilters').mockImplementation(() => undefined)

    mount(TopWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })

    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('No rows match the current filters.')
    expect(text).toContain('Broaden or clear the active filters to repopulate the ranking table.')
    expect(text).toContain('The prior selection fell out of the filtered shortlist.')

    const clearButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((button) =>
      String(button.textContent || '').includes('Clear filters'),
    )
    clearButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    expect(clearSpy).toHaveBeenCalledTimes(1)
  })
})
