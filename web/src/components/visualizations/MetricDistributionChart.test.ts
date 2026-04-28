import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import MetricDistributionChart from './MetricDistributionChart.vue'

const chartMocks = vi.hoisted(() => {
  const chart = {
    setOption: vi.fn(),
    resize: vi.fn(),
    dispose: vi.fn(),
  }
  return {
    chart,
    init: vi.fn(() => chart),
    use: vi.fn(),
  }
})

vi.mock('echarts/core', () => ({
  init: chartMocks.init,
  use: chartMocks.use,
}))

vi.mock('echarts/core.js', () => ({
  init: chartMocks.init,
  use: chartMocks.use,
}))

vi.mock('echarts/charts', () => ({
  BarChart: { type: 'bar' },
}))

vi.mock('echarts/charts.js', () => ({
  BarChart: { type: 'bar' },
}))

vi.mock('echarts/components', () => ({
  AriaComponent: { type: 'aria' },
  GridComponent: { type: 'grid' },
  TooltipComponent: { type: 'tooltip' },
}))

vi.mock('echarts/components.js', () => ({
  AriaComponent: { type: 'aria' },
  GridComponent: { type: 'grid' },
  TooltipComponent: { type: 'tooltip' },
}))

vi.mock('echarts/renderers', () => ({
  CanvasRenderer: { type: 'canvas' },
}))

vi.mock('echarts/renderers.js', () => ({
  CanvasRenderer: { type: 'canvas' },
}))

vi.mock('echarts/lib/chart/bar', () => ({}))
vi.mock('echarts/lib/chart/bar.js', () => ({}))
vi.mock('echarts/lib/component/aria', () => ({}))
vi.mock('echarts/lib/component/aria.js', () => ({}))
vi.mock('echarts/lib/component/grid', () => ({}))
vi.mock('echarts/lib/component/grid.js', () => ({}))
vi.mock('echarts/lib/component/tooltip', () => ({}))
vi.mock('echarts/lib/component/tooltip.js', () => ({}))

class ResizeObserverMock {
  observe = vi.fn()
  disconnect = vi.fn()
}

describe('MetricDistributionChart', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    chartMocks.init.mockImplementation(() => chartMocks.chart)
    ;(globalThis as typeof globalThis & { ResizeObserver?: typeof ResizeObserver }).ResizeObserver =
      ResizeObserverMock as unknown as typeof ResizeObserver
    vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(480)
    vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(160)
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(function getContextMock(
      this: HTMLCanvasElement,
    ) {
      return {
        canvas: this,
        setTransform: vi.fn(),
        resetTransform: vi.fn(),
        clearRect: vi.fn(),
        fillRect: vi.fn(),
        strokeRect: vi.fn(),
        fillText: vi.fn(),
        strokeText: vi.fn(),
        measureText: vi.fn(() => ({ width: 24 })),
        save: vi.fn(),
        restore: vi.fn(),
        translate: vi.fn(),
        rotate: vi.fn(),
        scale: vi.fn(),
        transform: vi.fn(),
        beginPath: vi.fn(),
        closePath: vi.fn(),
        moveTo: vi.fn(),
        lineTo: vi.fn(),
        bezierCurveTo: vi.fn(),
        quadraticCurveTo: vi.fn(),
        arc: vi.fn(),
        rect: vi.fn(),
        clip: vi.fn(),
        fill: vi.fn(),
        stroke: vi.fn(),
        setLineDash: vi.fn(),
        drawImage: vi.fn(),
        createPattern: vi.fn(() => ({})),
        createLinearGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
        font: '',
        fillStyle: '',
        strokeStyle: '',
        lineWidth: 1,
        textAlign: 'left',
        textBaseline: 'alphabetic',
        globalAlpha: 1,
      } as unknown as CanvasRenderingContext2D
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
    delete (globalThis as typeof globalThis & { ResizeObserver?: typeof ResizeObserver }).ResizeObserver
  })

  it('renders a preferred metric histogram with summary stats', async () => {
    const wrapper = mount(MetricDistributionChart, {
      props: {
        title: 'Pareto metric distribution',
        preferredKeys: ['pnl'],
        columns: [
          { key: 'pnl', label: 'PnL' },
          { key: 'max_dd', label: 'Max DD' },
        ],
        rows: [
          { cells: [{ text: '10', title: '10' }, { text: '2', title: '2' }] },
          { cells: [{ text: '20', title: '20' }, { text: '5', title: '5' }] },
          { cells: [{ text: '40', title: '40' }, { text: '8', title: '8' }] },
        ],
      },
      attachTo: document.body,
    })

    await nextTick()
    await vi.waitFor(() => expect(chartMocks.init).toHaveBeenCalledTimes(1))

    expect(wrapper.text()).toContain('Visual summary')
    expect(wrapper.text()).toContain('Pareto metric distribution')
    expect(wrapper.text()).toContain('PnL: n=3 min=10.0 median=20.0 max=40.0')
    expect(chartMocks.chart.setOption).toHaveBeenCalled()
    expect(chartMocks.chart.setOption.mock.calls[0]?.[0]).toMatchObject({
      series: [
        {
          type: 'bar',
          name: 'PnL',
        },
      ],
    })

    wrapper.unmount()
  })

  it('keeps a DOM histogram fallback available for resilient rendering', async () => {
    const wrapper = mount(MetricDistributionChart, {
      props: {
        title: 'Pareto metric distribution',
        preferredKeys: ['pnl'],
        columns: [
          { key: 'pnl', label: 'PnL' },
          { key: 'max_dd', label: 'Max DD' },
        ],
        rows: [
          { cells: [{ text: '10', title: '10' }, { text: '2', title: '2' }] },
          { cells: [{ text: '20', title: '20' }, { text: '5', title: '5' }] },
          { cells: [{ text: '40', title: '40' }, { text: '8', title: '8' }] },
        ],
      },
      attachTo: document.body,
    })

    await nextTick()
    await vi.waitFor(() => expect(chartMocks.init).toHaveBeenCalled())
    await nextTick()
    await vi.waitFor(() => expect(wrapper.find('.metricDistributionFallback').exists()).toBe(true))

    expect(wrapper.text()).toContain('PnL: n=3 min=10.0 median=20.0 max=40.0')
    expect(wrapper.find('.metricDistributionFallback').attributes('aria-label')).toContain('Histogram fallback')
    expect(wrapper.findAll('.metricDistributionFallbackRow').length).toBeGreaterThan(0)

    wrapper.unmount()
  })

  it('does not mount chart chrome when no numeric metric is available', () => {
    const wrapper = mount(MetricDistributionChart, {
      props: {
        title: 'No numeric metrics',
        columns: [{ key: 'gate', label: 'Gate' }],
        rows: [{ cells: [{ text: 'pass' }] }],
      },
    })

    expect(wrapper.html()).toBe('<!--v-if-->')
    expect(chartMocks.init).not.toHaveBeenCalled()
  })
})
