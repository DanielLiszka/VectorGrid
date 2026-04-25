import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ParetoWorkspacePanel from './ParetoWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useDataStore } from '../stores/dataStore'
import { useWorkflowStore } from '../stores/workflowStore'
import type { ParetoRenderModel } from '../visualizations/paretoModel'

const analysisServiceMocks = vi.hoisted(() => ({
  refreshParetoFromWorkflowRun: vi.fn(),
  drillParetoPoint: vi.fn(),
}))

vi.mock('../services/analysisService', () => ({
  refreshParetoFromWorkflowRun: analysisServiceMocks.refreshParetoFromWorkflowRun,
  drillParetoPoint: analysisServiceMocks.drillParetoPoint,
}))

const paretoModel: ParetoRenderModel = {
  xKey: 'max_dd',
  yKey: 'pnl',
  points: [
    {
      index: 0,
      rowId: 'row-1',
      rank: 1,
      x: 0.2,
      y: 0.05,
      frontier: true,
      summary: '#1 - max_dd=0.2 - pnl=0.05',
    },
    {
      index: 1,
      rowId: 'row-2',
      rank: 2,
      x: 0.1,
      y: 0.12,
      frontier: true,
      summary: '#2 - max_dd=0.1 - pnl=0.12',
    },
  ],
  frontier: [],
  xRange: { min: 0.08, max: 0.22 },
  yRange: { min: 0.03, max: 0.14 },
  meta: '2 points - x=max_dd - y=pnl',
  truthNote: 'Pareto points show tradeoffs, not proof of robustness.',
  representativeSummary: 'x-extreme #2 - y-extreme #2',
  confidenceNote: 'Validation has not run. Frontier membership alone does not approve a candidate.',
  status: 'ready',
}
paretoModel.frontier = [...paretoModel.points].sort((left, right) => left.x - right.x)

function installParetoPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="pareto"></section>
    <div id="paretoVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

function mockCanvasContext(): void {
  const context = {
    setTransform: vi.fn(),
    clearRect: vi.fn(),
    fillRect: vi.fn(),
    strokeRect: vi.fn(),
    fillText: vi.fn(),
    save: vi.fn(),
    translate: vi.fn(),
    rotate: vi.fn(),
    restore: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    arc: vi.fn(),
    fill: vi.fn(),
    font: '',
    fillStyle: '',
    strokeStyle: '',
    lineWidth: 1,
  }
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(context as unknown as CanvasRenderingContext2D)
}

describe('ParetoWorkspacePanel', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    analysisServiceMocks.refreshParetoFromWorkflowRun.mockReset()
    analysisServiceMocks.drillParetoPoint.mockReset()
    mockCanvasContext()
    installParetoPanelMountShell()
    localStorage.clear()
  })

  it('distinguishes no-data and no-run empty states before frontier data exists', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('pareto', { persist: false })

    const wrapper = mount(ParetoWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.body.textContent || '').toContain('Load data before opening the Pareto view.')

    dataStore.dataId = 'dataset_1'
    await flushPanel()

    expect(document.body.textContent || '').toContain('Run an optimization to generate Pareto points.')
    wrapper.unmount()
  })

  it('keeps visible axis and meta cues when Pareto output is ready', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('pareto', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.bestTitle = 'Rank #1 · Winner'
    analysisStore.paretoX = 'max_dd'
    analysisStore.paretoY = 'pnl'
    analysisStore.paretoXOptions = [{ value: 'max_dd', label: 'MaxDD', disabled: false }]
    analysisStore.paretoYOptions = [{ value: 'pnl', label: 'PnL', disabled: false }]
    analysisStore.paretoMeta = '12 points • x=max_dd • y=pnl'
    analysisStore.paretoTruthNote = 'Pareto points show tradeoffs, not proof of robustness.'
    analysisStore.paretoRepresentativeSummary = 'x-extreme #2 • y-extreme #5 • selected #3'
    analysisStore.paretoConfidenceNote = 'Validation has not run. Frontier membership alone does not approve a candidate.'

    mount(ParetoWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const text = document.body.textContent || ''
    expect(text).toContain('X axis')
    expect(text).toContain('Y axis')
    expect(text).toContain('12 points • x=max_dd • y=pnl')
    expect(text).toContain('Pareto points show tradeoffs, not proof of robustness.')
    expect(text).toContain('Representative points: x-extreme #2 • y-extreme #5 • selected #3')
    expect(text).toContain('Validation has not run. Frontier membership alone does not approve a candidate.')
    expect(text).toContain('Use the Top results table as the keyboard-friendly way')
  })

  it('renders model-backed Pareto points and routes keyboard activation', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const analysisStore = useAnalysisStore(pinia)
    const dataStore = useDataStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('pareto', { persist: false })
    dataStore.dataId = 'dataset_1'
    analysisStore.bestTitle = 'Rank #1 · Winner'
    analysisStore.paretoXOptions = [{ value: 'max_dd', label: 'MaxDD', disabled: false }]
    analysisStore.paretoYOptions = [{ value: 'pnl', label: 'PnL', disabled: false }]
    analysisStore.paretoMeta = paretoModel.meta
    analysisStore.paretoTruthNote = paretoModel.truthNote
    analysisStore.paretoRepresentativeSummary = paretoModel.representativeSummary
    analysisStore.paretoConfidenceNote = paretoModel.confidenceNote
    analysisStore.paretoModel = paretoModel

    mount(ParetoWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.querySelector('#paretoWrapVue')).not.toBeNull()
    expect(document.body.textContent || '').toContain('#1 - max_dd=0.2 - pnl=0.05')

    const chartSurface = document.querySelector('.paretoChartSurface')
    expect(chartSurface).not.toBeNull()
    chartSurface?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }))
    chartSurface?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await flushPanel()

    await vi.waitFor(() => {
      expect(analysisServiceMocks.drillParetoPoint).toHaveBeenCalledWith('row-2')
    })
  })

  it('keeps an explicit Review Top results handoff', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('pareto', { persist: false })

    mount(ParetoWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const button = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((candidate) =>
      String(candidate.textContent || '').includes('Review Top results'),
    )
    button?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()

    expect(workflowStore.resultsTab).toBe('top')
  })
})
