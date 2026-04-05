// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import AdvancedWorkspacePanel from './AdvancedWorkspacePanel.vue'
import { useAdvancedStore } from '../stores/advancedStore'
import { useOptimizeStore } from '../stores/optimizeStore'
import { useStrategyStore } from '../stores/strategyStore'
import { useWorkflowStore } from '../stores/workflowStore'

function installAdvancedPanelMountShell(): void {
  document.body.innerHTML = `
    <section data-vue-panel="advanced"></section>
    <div id="advancedVueShellMount"></div>
  `
}

async function flushPanelTimers(): Promise<void> {
  await nextTick()
  await vi.runAllTimersAsync()
  await nextTick()
}

describe('AdvancedWorkspacePanel', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    installAdvancedPanelMountShell()
    localStorage.clear()
  })

  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('shows heatmap and export controls for Double MA', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DoubleMa'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueHeatmapBins')).not.toBeNull()
    expect(document.querySelector('#vueExportAllCsvPath')).not.toBeNull()

    wrapper.unmount()
  })

  it('hides output-shaping controls that Composite MO does not use', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'CompositeMo'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueHeatmapBins')).toBeNull()
    expect(document.querySelector('#vueTopK')).toBeNull()
    expect(document.querySelector('#vueIncludeAll')).toBeNull()
    expect(document.querySelector('#vueExportAllCsvPath')).toBeNull()
    expect(document.body.textContent || '').toContain('Composite MO returns Pareto points from its own search-space and prescan settings')

    wrapper.unmount()
  })

  it('keeps Top K but hides heatmap for VCC', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'Vcc'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueHeatmapBins')).toBeNull()
    expect(document.querySelector('#vueTopK')).not.toBeNull()
    expect(document.querySelector('#vueIncludeAll')).not.toBeNull()

    wrapper.unmount()
  })

  it('shows standalone unified-GPU shortlist controls on requested GpuKernel routes without exact-kernel-only extras', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const advanced = useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'Vemr'
    optimize.backend = 'GpuKernel'
    advanced.includeAll = 'true'
    advanced.extraShortlistObjectives = 'Pnl,Omega'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueShortlistConfidenceMode')).not.toBeNull()
    expect(document.querySelector('#vueShortlistUnionMode')).not.toBeNull()
    expect(document.querySelector('#vueShortlistSliceMode')).toBeNull()
    expect(document.body.textContent || '').toContain('standalone unified-GPU')
    expect(document.body.textContent || '').toContain('host-side')

    wrapper.unmount()
  })

  it('shows shortlist confidence control for Double MA', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DoubleMa'
    optimize.backend = 'GpuKernel'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueShortlistConfidenceMode')).not.toBeNull()
    expect(document.body.textContent || '').toContain('PSR / DSR rerank')

    wrapper.unmount()
  })

  it('hides exact-kernel-only controls for Double MA when the requested backend is not GpuKernel', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DoubleMa'
    optimize.backend = 'CpuOnly'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueIncludeTradeMetrics')).toBeNull()
    expect(document.querySelector('#vueExtraShortlistObjectives')).toBeNull()
    expect(document.querySelector('#vueShortlistConfidenceMode')).toBeNull()
    expect(document.querySelector('#vueShortlistUnionMode')).toBeNull()
    expect(document.querySelector('#vueShortlistSliceMode')).toBeNull()

    wrapper.unmount()
  })

  it('shows exact-kernel-only controls for CompositeTrend when ADX is off on the exact route', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const strategy = useStrategyStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'CompositeTrend'
    optimize.backend = 'GpuKernel'
    strategy.trendUseAdx = false

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueMetricContract')).not.toBeNull()
    expect(document.querySelector('#vueExtraShortlistObjectives')).not.toBeNull()
    expect(document.querySelector('#vueShortlistConfidenceMode')).not.toBeNull()

    wrapper.unmount()
  })

  it('keeps metric-contract controls visible but hides exact shortlist controls for CompositeTrend when ADX promotes the route to hybrid', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const strategy = useStrategyStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'CompositeTrend'
    optimize.backend = 'GpuKernel'
    strategy.trendUseAdx = true

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueMetricContract')).not.toBeNull()
    expect(document.querySelector('#vueExtraShortlistObjectives')).toBeNull()
    expect(document.querySelector('#vueShortlistConfidenceMode')).toBeNull()
    expect(document.querySelector('#vueShortlistUnionMode')).toBeNull()
    expect(document.querySelector('#vueShortlistSliceMode')).toBeNull()
    expect(document.body.textContent || '').toContain('Explicit Full17 and Trade22 metric-contract requests are available')
    expect(document.body.textContent || '').toContain('advanced exact-kernel shortlist/score controls remain disabled')

    wrapper.unmount()
  })

  it('shows the current dynamic exact-route controls for DynamicDoubleMa on the requested GpuKernel route', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DynamicDoubleMa'
    optimize.backend = 'GpuKernel'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueExtraShortlistObjectives')).not.toBeNull()
    expect(document.querySelector('#vueShortlistConfidenceMode')).not.toBeNull()
    expect(document.querySelector('#vueMetricContract')).not.toBeNull()
    expect(document.querySelector('#vueShortlistUnionMode')).not.toBeNull()
    expect(document.querySelector('#vueShortlistSliceMode')).not.toBeNull()
    expect(document.querySelector('#vueShortlistSliceBenchmark')).not.toBeNull()
    expect(document.querySelector('#vueShortlistNeighborhoodRadius')).not.toBeNull()
    expect(document.body.textContent || '').toContain('exact-compatible DynamicDoubleMa GpuKernel(dynamic_strategy_v1) shapes')

    wrapper.unmount()
  })

  it('shows shortlist confidence control for DynamicDoubleMa on the requested GpuKernel route', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DynamicDoubleMa'
    optimize.backend = 'GpuKernel'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueShortlistConfidenceMode')).not.toBeNull()
    expect(document.body.textContent || '').toContain('DynamicDoubleMa shortlist')

    wrapper.unmount()
  })

  it('shows shortlist union control for Double MA', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const advanced = useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DoubleMa'
    optimize.backend = 'GpuKernel'
    advanced.extraShortlistObjectives = 'Pnl,Omega'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueShortlistUnionMode')).not.toBeNull()
    expect(document.body.textContent || '').toContain('union shortlist')

    wrapper.unmount()
  })

  it('shows the explicit metric contract control for Double MA on the GpuKernel route and reflects the stored value', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const advanced = useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DoubleMa'
    optimize.backend = 'GpuKernel'
    advanced.metricContract = 'full_17'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    const select = document.querySelector('#vueMetricContract') as HTMLSelectElement | null
    expect(select).not.toBeNull()
    expect(select?.value).toBe('full_17')
    expect(document.body.textContent || '').toContain('Full 17-row metrics')

    wrapper.unmount()
  })

  it('shows the explicit metric contract control for CompositeCpu on its fixed route', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const advanced = useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'CompositeCpu'
    advanced.metricContract = 'trade_22'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    const select = document.querySelector('#vueMetricContract') as HTMLSelectElement | null
    expect(select).not.toBeNull()
    expect(select?.value).toBe('trade_22')
    expect(document.body.textContent || '').toContain('Forces the 22-metric trade-summary contract on the current audited route')

    wrapper.unmount()
  })

  it('shows hurdle tau control for Double MA and reflects the stored value', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const advanced = useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DoubleMa'
    advanced.hurdleTau = '0.01'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    const input = document.querySelector('#vueHurdleTau') as HTMLInputElement | null
    expect(input).not.toBeNull()
    expect(input?.value).toBe('0.01')
    expect(document.body.textContent || '').toContain('Hurdle tau')

    wrapper.unmount()
  })

  it('shows shortlist union control for DynamicDoubleMa when extra shortlist heads are requested on the GpuKernel route', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const advanced = useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DynamicDoubleMa'
    optimize.backend = 'GpuKernel'
    advanced.extraShortlistObjectives = 'Pnl,Omega'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueShortlistUnionMode')).not.toBeNull()
    expect(document.querySelector('#vueShortlistConfidenceMode')).not.toBeNull()
    expect(document.querySelector('#vueShortlistSliceMode')).not.toBeNull()
    expect(document.body.textContent || '').toContain('DynamicDoubleMa union shortlist')

    wrapper.unmount()
  })

  it('shows shortlist slice controls for Double MA', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const advanced = useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DoubleMa'
    optimize.backend = 'GpuKernel'
    advanced.extraShortlistObjectives = 'Pnl,Omega'
    advanced.shortlistSliceMode = 'halves'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueShortlistSliceMode')).not.toBeNull()
    expect(document.querySelector('#vueShortlistSliceBenchmark')).not.toBeNull()
    expect(document.querySelector('#vueShortlistNeighborhoodRadius')).not.toBeNull()
    expect(document.querySelector('#vueShortlistNeighborhoodStep')).not.toBeNull()
    expect(document.body.textContent || '').toContain('Shortlist slice mode')

    wrapper.unmount()
  })

  it('keeps shortlist slice controls visible with a stale DynamicDoubleMa value so they can be cleared', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflow = useWorkflowStore(pinia)
    const optimize = useOptimizeStore(pinia)
    const advanced = useAdvancedStore(pinia)
    workflow.selectControlsTab('advanced', { persist: false })
    optimize.strategyKind = 'DynamicDoubleMa'
    advanced.shortlistSliceMode = 'halves'

    const wrapper = mount(AdvancedWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await flushPanelTimers()

    expect(document.querySelector('#vueShortlistSliceMode')).not.toBeNull()
    expect(document.body.textContent || '').toContain('The current route does not support shortlist slice mode')

    wrapper.unmount()
  })
})
