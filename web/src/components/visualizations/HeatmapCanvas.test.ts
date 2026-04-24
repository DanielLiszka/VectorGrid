// @vitest-environment jsdom
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import HeatmapCanvas from './HeatmapCanvas.vue'
import type { HeatmapCanvasRenderModel } from '../../visualizations/heatmapCanvasModel'

function renderModel(): HeatmapCanvasRenderModel {
  return {
    signature: 'heatmap:test',
    meta: 'Sharpe heatmap',
    statusLine: 'Heatmap ready.',
    model: {
      binsFast: 2,
      binsSlow: 2,
      fastMin: 5,
      fastMax: 9,
      slowMin: 20,
      slowMax: 30,
      values: [0.1, 0.2, -0.1, 0.05],
      meanValues: null,
      sampleCounts: null,
    },
    displayMode: 'objective',
    displayValues: [0.1, 0.2, -0.1, 0.05],
    displayLabel: 'Sharpe',
    displayLegendNote: 'Zero-centered',
    request: {
      fastMaType: 'ema',
      slowMaType: 'ema',
      fastMaParams: null,
      slowMaParams: null,
      scoreLabel: 'Sharpe',
    },
    markers: [{ col: 0, drawRow: 1, label: '1' }],
    range: { min: -0.2, max: 0.2, label: 'zero-centered' },
    palette: {
      kind: 'diverging',
      negative: '#b91c1c',
      zero: '#2563eb',
      positive: '#15803d',
    },
    plateauDiagnostics: null,
  }
}

function mockCanvasContext() {
  return vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(
    () => ({
      clearRect: vi.fn(),
      setTransform: vi.fn(),
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      beginPath: vi.fn(),
      arc: vi.fn(),
      fill: vi.fn(),
      stroke: vi.fn(),
      fillText: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      drawImage: vi.fn(),
      createLinearGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
    }) as never,
  )
}

describe('HeatmapCanvas', () => {
  it('owns the heatmap DOM contract and empty state', () => {
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        selectedSummary: 'Selected cell: fast 12, slow 48.',
      },
    })

    expect(wrapper.get('#heatmapEmptyHintVue').text()).toContain('Heatmap bins')
    expect(wrapper.get('#heatmapWrapVue').attributes('style')).toContain('display: none')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('aria-label')).toContain('Optimization heatmap')
    expect(wrapper.get('#heatmapCanvasStackVue').attributes('tabindex')).toBe('0')
    expect(wrapper.find('#heatmapCanvasVue').exists()).toBe(true)
    expect(wrapper.find('#heatmapOverlayVue').exists()).toBe(true)
    expect(wrapper.find('#heatmapTooltipVue').exists()).toBe(true)
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('fast 12')
  })

  it('emits an explicit Top results handoff from the visualization host', async () => {
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
      },
    })

    await wrapper.get('button').trigger('click')

    expect(wrapper.emitted('review-top-results')).toHaveLength(1)
  })

  it('renders model-owned metadata and emits representative drilldown params from clicks', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })
    const overlay = wrapper.get<HTMLCanvasElement>('#heatmapOverlayVue').element
    Object.defineProperty(overlay, 'getBoundingClientRect', {
      configurable: true,
      value: () => ({
        left: 0,
        top: 0,
        width: 512,
        height: 512,
        right: 512,
        bottom: 512,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }),
    })

    expect(wrapper.get('#heatmapMetaVue').text()).toContain('Sharpe heatmap')

    await wrapper.get('#heatmapOverlayVue').trigger('mousemove', { clientX: 128, clientY: 128 })
    expect(wrapper.get('#heatmapTooltipVue').attributes('style') || '').toContain('display: block')
    expect(wrapper.get('#heatmapTooltipVue').text()).toContain('Sharpe')

    await wrapper.get('#heatmapOverlayVue').trigger('click', { clientX: 128, clientY: 384 })
    expect(wrapper.emitted('request-drilldown')?.[0]?.[0]).toMatchObject({
      fast_len: 5,
      fast_ma_type: 'ema',
      slow_len: 20,
      slow_ma_type: 'ema',
    })
  })

  it('supports keyboard cell selection with a persistent selected-cell summary', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'Home' })
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('EMA 5')
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('EMA 30')

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'ArrowRight' })
    expect(wrapper.get('#heatmapSelectedCellSummaryVue').text()).toContain('EMA 9')

    await wrapper.get('#heatmapCanvasStackVue').trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('request-drilldown')?.[0]?.[0]).toMatchObject({
      fast_len: 9,
      fast_ma_type: 'ema',
      slow_len: 30,
      slow_ma_type: 'ema',
    })
  })
})
