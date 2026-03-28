// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { computed, nextTick, ref } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import TopWorkspacePanel from './TopWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useWorkflowStore } from '../stores/workflowStore'

vi.mock('../composables/useVirtualTableRows', () => ({
  useVirtualTableRows: (rows: { value: unknown[] }) => ({
    scrollElementRef: ref(null),
    virtualRows: computed(() => rows.value.map((row, index) => ({ index, row }))),
    topSpacerPx: computed(() => 0),
    bottomSpacerPx: computed(() => 0),
    scrollToTop: vi.fn(),
  }),
}))

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
        badges: ['Current leader', 'Filtered shortlist'],
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
})
