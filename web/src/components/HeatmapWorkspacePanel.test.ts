import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import HeatmapWorkspacePanel from './HeatmapWorkspacePanel.vue'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { refreshHeatmapFromWorkflowRun, restoreHeatmapVisible } from '../services/analysisService'
import type { HeatmapCanvasRenderModel } from '../visualizations/heatmapCanvasModel'

vi.mock('../services/analysisService', () => ({
  refreshHeatmapFromWorkflowRun: vi.fn(),
  registerHeatmapCanvasController: vi.fn(),
  runHeatmapRepresentativeCandidateDetail: vi.fn(),
  restoreHeatmapVisible: vi.fn(() => false),
}))

const refreshHeatmapFromWorkflowRunMock = vi.mocked(refreshHeatmapFromWorkflowRun)
const restoreHeatmapVisibleMock = vi.mocked(restoreHeatmapVisible)

function installHeatmapPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="heatmap"></section>
    <div id="heatmapVueShellMount"></div>
  `
}

async function flushPanel(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('HeatmapWorkspacePanel', () => {
  beforeEach(() => {
    installHeatmapPanelMountShell()
    localStorage.clear()
    vi.clearAllMocks()
    restoreHeatmapVisibleMock.mockReturnValue(false)
  })

  it('prefers restoring a visible heatmap over rebuilding it from the workflow run', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    restoreHeatmapVisibleMock.mockReturnValue(true)

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(restoreHeatmapVisibleMock).toHaveBeenCalled()
    expect(refreshHeatmapFromWorkflowRunMock).not.toHaveBeenCalled()
  })

  it('falls back to a workflow refresh and keeps export routing inside the analysis store', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    analysisStore.canHeatmapExportPng = true
    const exportSpy = vi.spyOn(analysisStore, 'heatmapExportPng').mockImplementation(() => undefined)

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(refreshHeatmapFromWorkflowRunMock).toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('Heatmap bins > 0')
    expect(document.body.textContent || '').toContain('Current view')
    expect(document.body.textContent || '').toContain('Canvas')
    expect(document.body.textContent || '').toContain('Awaiting run')
    expect(document.body.textContent || '').toContain('Export')
    expect(document.body.textContent || '').toContain('Ready')
    const exportButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((candidate) =>
      String(candidate.textContent || '').includes('Export PNG'),
    )
    expect(exportButton?.disabled).toBe(false)
    exportButton?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await flushPanel()
    expect(exportSpy).toHaveBeenCalledTimes(1)
  })

  it('does not trigger a redundant heatmap rebuild just because the analysis timestamp changes', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(refreshHeatmapFromWorkflowRunMock).toHaveBeenCalledTimes(1)

    analysisStore.updatedAtUnixMs = 1_700_000_000_000
    await flushPanel()

    expect(refreshHeatmapFromWorkflowRunMock).toHaveBeenCalledTimes(1)
  })

  it('keeps an explicit Review Top results handoff', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })

    mount(HeatmapWorkspacePanel, {
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

  it('routes display-mode changes through the analysis store selector action and updates the live copy contract', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    analysisStore.heatmapMode = 'objective'
    analysisStore.heatmapModeOptions = [
      { value: 'objective', label: 'Objective score', disabled: false },
      { value: 'plateau', label: 'Derived plateau', disabled: false },
    ]
    analysisStore.heatmapViewMode = '2d'
    analysisStore.heatmapViewModeOptions = [
      { value: '2d', label: '2D heatmap', disabled: false },
      { value: '3d', label: '3D slices', disabled: false },
    ]
    analysisStore.canHeatmapExportPng = true
    const setSelectFieldSpy = vi.spyOn(analysisStore, 'setSelectField')

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const select = document.querySelector<HTMLSelectElement>('#vueHeatmapMode')
    expect(select).not.toBeNull()
    select!.value = 'plateau'
    select!.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()

    expect(setSelectFieldSpy).toHaveBeenCalledWith('heatmapMode', 'plateau')
    expect(document.body.textContent || '').toContain('supportive 8-neighbor agreement')
    expect(document.body.textContent || '').toContain('representative parameter pair')
    expect(document.querySelector('#heatmapCanvasStackVue')?.getAttribute('aria-label') || '').toContain('Derived plateau display mode')

    const viewSelect = document.querySelector<HTMLSelectElement>('#vueHeatmapViewMode')
    expect(viewSelect).not.toBeNull()
    expect(viewSelect!.value).toBe('2d')
    expect(
      viewSelect!.querySelector<HTMLOptionElement>('option[value="3d"]')?.disabled,
    ).toBe(true)

    select!.value = 'objective'
    select!.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()
    expect(
      viewSelect!.querySelector<HTMLOptionElement>('option[value="3d"]')?.disabled,
    ).toBe(false)

    viewSelect!.value = '3d'
    viewSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()

    expect(setSelectFieldSpy).toHaveBeenCalledWith('heatmapViewMode', '3d')
    expect(document.body.textContent || '').toContain('3D slices')
    const exportButton = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((candidate) =>
      String(candidate.textContent || '').includes('Export PNG'),
    )
    expect(exportButton?.disabled).toBe(true)
    expect(exportButton?.title).toContain('renderer is ready')
  })

  it('explains how to enable 3D when the current run has only an aggregate heatmap', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    analysisStore.heatmapMode = 'objective'
    analysisStore.heatmapViewModeOptions = [
      { value: '2d', label: '2D heatmap', disabled: false },
      {
        value: '3d',
        label: '3D slices',
        disabled: true,
      },
    ]

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const viewSelect = document.querySelector<HTMLSelectElement>('#vueHeatmapViewMode')
    const reason = document.querySelector<HTMLElement>('#heatmap3dAvailabilityReasonVue')
    expect(viewSelect?.querySelector<HTMLOptionElement>('option[value="3d"]')?.disabled).toBe(true)
    expect(viewSelect?.getAttribute('aria-describedby')).toBe('heatmap3dAvailabilityReasonVue')
    expect(reason?.textContent || '').toContain('Return all results')
    expect(reason?.textContent || '').toContain('at least two variants')

    analysisStore.heatmapViewModeOptions = [
      { value: '2d', label: '2D heatmap', disabled: false },
      { value: '3d', label: '3D slices', disabled: false },
    ]
    await flushPanel()

    expect(viewSelect?.hasAttribute('aria-describedby')).toBe(false)
    expect(document.querySelector('#heatmap3dAvailabilityReasonVue')).toBeNull()
  })

  it('explains the plateau score as a derived robustness view', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    analysisStore.heatmapMode = 'plateau'

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    expect(document.body.textContent || '').toContain('supportive 8-neighbor agreement')
    expect(document.body.textContent || '').toContain('bin-consistency')
    expect(document.body.textContent || '').toContain('representative parameter pair')
    expect(document.body.textContent || '').toContain('exact retained-candidate path')
  })

  it('forces restored plateau state onto the truthful 2D view', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    analysisStore.heatmapMode = 'plateau'
    analysisStore.heatmapModeOptions = [
      { value: 'objective', label: 'Objective score', disabled: false },
      { value: 'plateau', label: 'Derived plateau', disabled: false },
    ]
    analysisStore.heatmapViewMode = '3d'
    analysisStore.heatmapViewModeOptions = [
      { value: '2d', label: '2D heatmap', disabled: false },
      { value: '3d', label: '3D slices', disabled: false },
    ]

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const viewSelect = document.querySelector<HTMLSelectElement>('#vueHeatmapViewMode')
    expect(viewSelect?.value).toBe('2d')
    expect(analysisStore.heatmapViewMode).toBe('2d')
    expect(
      viewSelect?.querySelector<HTMLOptionElement>('option[value="3d"]')?.disabled,
    ).toBe(true)
    expect(document.body.textContent || '').toContain('2D heatmap · Derived plateau')
    expect(document.querySelector('#heatmapSurface3DHostVue')).toBeNull()

    analysisStore.setSelectField('heatmapMode', 'objective')
    await flushPanel()
    expect(analysisStore.heatmapViewMode).toBe('2d')
    expect(viewSelect?.value).toBe('2d')
  })

  it('falls back an unavailable delta to the aggregate while keeping comparison controls recoverable', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const analysisStore = useAnalysisStore(pinia)
    workflowStore.selectWorkflow('analyze', { persist: false })
    workflowStore.selectResultsTab('heatmap', { persist: false })
    analysisStore.heatmapCanvasModel = {
      signature: 'no-overlap-slices',
      meta: 'fixture',
      statusLine: 'ready',
      model: {
        binsFast: 2,
        binsSlow: 2,
        fastMin: 5,
        fastMax: 10,
        slowMin: 20,
        slowMax: 30,
        values: [0.3, 0.2, 0.1, null],
        meanValues: null,
        sampleCounts: null,
      },
      displayMode: 'objective',
      displayValues: [0.3, 0.2, 0.1, null],
      displayLabel: 'Sharpe',
      displayLegendNote: 'Zero-centered',
      request: {
        fastAxisLabel: 'Fast SMA length',
        slowAxisLabel: 'Slow SMA length',
        fastAxisDescription: 'Fast SMA length used by each grid column.',
        slowAxisDescription: 'Slow SMA length used by each grid row.',
        supports3dSlices: true,
        threeDimensionalUnavailableReason: '',
        fastMaType: 'sma',
        slowMaType: 'sma',
        fastMaParams: null,
        slowMaParams: null,
        scoreLabel: 'Sharpe',
      },
      markers: [],
      range: { min: -0.3, max: 0.3, label: 'zero-centered' },
      palette: {
        kind: 'diverging',
        negative: '#b91c1c',
        zero: '#2563eb',
        positive: '#15803d',
      },
      plateauDiagnostics: null,
      slice3d: {
        axisLabel: 'MA variants',
        scoreLabel: 'Sharpe',
        slices: [
          {
            key: 'slice-a',
            label: 'SMA / SMA',
            shortLabel: 'SMA/SMA',
            values: [0.3, null, null, null],
            sampleCounts: [1, null, null, null],
            cells: [{ params: { fast_len: 5, slow_len: 20 }, metrics: {} }, null, null, null],
            sourceRowCount: 1,
            finiteCells: 1,
            bestScore: 0.3,
          },
          {
            key: 'slice-b',
            label: 'EMA / EMA',
            shortLabel: 'EMA/EMA',
            values: [null, 0.2, null, null],
            sampleCounts: [null, 1, null, null],
            cells: [null, { params: { fast_len: 10, slow_len: 20 }, metrics: {} }, null, null],
            sourceRowCount: 1,
            finiteCells: 1,
            bestScore: 0.2,
          },
          {
            key: 'slice-c',
            label: 'EMA / SMA',
            shortLabel: 'EMA/SMA',
            values: [0.1, null, null, null],
            sampleCounts: [1, null, null, null],
            cells: [{ params: { fast_len: 5, slow_len: 20 }, metrics: {} }, null, null, null],
            sourceRowCount: 1,
            finiteCells: 1,
            bestScore: 0.1,
          },
        ],
        range: { min: -0.3, max: 0.3, label: 'zero-centered' },
        palette: {
          kind: 'diverging',
          negative: '#b91c1c',
          zero: '#2563eb',
          positive: '#15803d',
        },
        sourceRowCount: 3,
        retainedSliceRowCount: 3,
        unavailableReason: '',
      },
    } satisfies HeatmapCanvasRenderModel

    mount(HeatmapWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanel()

    const sourceSelect = document.querySelector<HTMLSelectElement>('#vueHeatmap2dSource')
    expect(sourceSelect).not.toBeNull()
    sourceSelect!.value = 'delta'
    sourceSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()

    expect(sourceSelect!.value).toBe('delta')
    expect(document.body.textContent || '').toContain(
      'do not share any retained heatmap cells',
    )
    expect(document.body.textContent || '').toContain('SourceAggregate')
    expect(document.body.textContent || '').not.toContain('SourceDelta')

    const selectedSelect = document.querySelector<HTMLSelectElement>('#vueHeatmapSliceSelect')
    const baselineSelect = document.querySelector<HTMLSelectElement>('#vueHeatmapBaselineSliceSelect')
    expect(selectedSelect?.value).toBe('slice-a')
    expect(baselineSelect?.value).toBe('slice-b')

    baselineSelect!.value = 'slice-c'
    baselineSelect!.dispatchEvent(new Event('change', { bubbles: true }))
    await flushPanel()

    expect(sourceSelect!.value).toBe('delta')
    expect(document.body.textContent || '').not.toContain(
      'do not share any retained heatmap cells',
    )
    expect(document.body.textContent || '').toContain('SourceDelta')

    analysisStore.heatmapViewMode = '3d'
    analysisStore.heatmapViewModeOptions = [
      { value: '2d', label: '2D heatmap', disabled: false },
      { value: '3d', label: '3D slices', disabled: false },
    ]
    await flushPanel()

    const metricStrip = document.querySelector<HTMLElement>('[aria-label="Heatmap state"]')
    expect(metricStrip?.textContent || '').toContain('SourceSelected slice')
    expect(metricStrip?.textContent || '').not.toContain('Fit')
    expect(metricStrip?.textContent || '').not.toContain('Markers')
  })
})
