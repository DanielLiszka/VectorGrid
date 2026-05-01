
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
      save: vi.fn(),
      restore: vi.fn(),
      translate: vi.fn(),
      scale: vi.fn(),
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

function setElementRect(element: Element, width = 512, height = 512): void {
  Object.defineProperty(element, 'getBoundingClientRect', {
    configurable: true,
    value: () => ({
      left: 0,
      top: 0,
      width,
      height,
      right: width,
      bottom: height,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    }),
  })
}

function dispatchPointerLikeEvent(
  element: Element,
  type: string,
  init: MouseEventInit & { deltaY?: number; pointerId?: number } = {},
): void {
  const event = type === 'wheel'
    ? new WheelEvent(type, {
        bubbles: true,
        cancelable: true,
        clientX: init.clientX ?? 0,
        clientY: init.clientY ?? 0,
        deltaY: init.deltaY ?? 0,
      })
    : new MouseEvent(type, {
        bubbles: true,
        cancelable: true,
        button: init.button ?? 0,
        clientX: init.clientX ?? 0,
        clientY: init.clientY ?? 0,
      })
  if (init.pointerId != null) {
    Object.defineProperty(event, 'pointerId', {
      configurable: true,
      value: init.pointerId,
    })
  }
  element.dispatchEvent(event)
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

  it('renders model-owned metadata and emits representative candidate-detail params from clicks', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })
    const overlay = wrapper.get<HTMLCanvasElement>('#heatmapOverlayVue').element
    setElementRect(overlay)

    expect(wrapper.get('#heatmapMetaVue').text()).toContain('Sharpe heatmap')

    await wrapper.get('#heatmapOverlayVue').trigger('mousemove', { clientX: 128, clientY: 128 })
    expect(wrapper.get('#heatmapTooltipVue').attributes('style') || '').toContain('display: block')
    expect(wrapper.get('#heatmapTooltipVue').text()).toContain('Sharpe')

    await wrapper.get('#heatmapOverlayVue').trigger('click', { clientX: 128, clientY: 384 })
    expect(wrapper.emitted('request-candidate-detail')?.[0]?.[0]).toMatchObject({
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
    expect(wrapper.emitted('request-candidate-detail')?.[0]?.[0]).toMatchObject({
      fast_len: 9,
      fast_ma_type: 'ema',
      slow_len: 30,
      slow_ma_type: 'ema',
    })
  })

  it('zooms with the pointer wheel and resets from the viewport controls', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })
    setElementRect(wrapper.get('#heatmapCanvasStackVue').element)
    setElementRect(wrapper.get('#heatmapOverlayVue').element)
    await wrapper.vm.$nextTick()

    dispatchPointerLikeEvent(wrapper.get('#heatmapCanvasStackVue').element, 'wheel', {
      clientX: 256,
      clientY: 256,
      deltaY: -300,
    })
    await wrapper.vm.$nextTick()

    expect(Number(wrapper.get('#heatmapCanvasStackVue').attributes('data-viewport-scale'))).toBeGreaterThan(1)

    await wrapper.get('button[aria-label="Reset heatmap viewport"]').trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.get('#heatmapCanvasStackVue').attributes('data-viewport-scale')).toBe('1.000')
    expect(wrapper.get('button[aria-label="Reset heatmap viewport"]').attributes('disabled')).toBeDefined()
  })

  it('pans a zoomed heatmap and suppresses the drag-release click handoff', async () => {
    mockCanvasContext()
    const wrapper = mount(HeatmapCanvas, {
      props: {
        ariaLabel: 'Optimization heatmap for the selected objective.',
        renderModel: renderModel(),
      },
    })
    const stack = wrapper.get('#heatmapCanvasStackVue')
    const overlay = wrapper.get('#heatmapOverlayVue')
    setElementRect(stack.element)
    setElementRect(overlay.element)
    await wrapper.vm.$nextTick()

    dispatchPointerLikeEvent(stack.element, 'wheel', {
      clientX: 256,
      clientY: 256,
      deltaY: -300,
    })
    await wrapper.vm.$nextTick()
    const offsetBeforePan = Number(stack.attributes('data-viewport-offset-x'))

    dispatchPointerLikeEvent(overlay.element, 'pointerdown', {
      pointerId: 7,
      button: 0,
      clientX: 256,
      clientY: 256,
    })
    dispatchPointerLikeEvent(overlay.element, 'pointermove', {
      pointerId: 7,
      clientX: 296,
      clientY: 256,
    })
    dispatchPointerLikeEvent(overlay.element, 'pointerup', {
      pointerId: 7,
      clientX: 296,
      clientY: 256,
    })
    await wrapper.vm.$nextTick()

    expect(Number(stack.attributes('data-viewport-offset-x'))).toBeGreaterThan(offsetBeforePan)

    dispatchPointerLikeEvent(overlay.element, 'click', {
      clientX: 296,
      clientY: 256,
    })

    expect(wrapper.emitted('request-candidate-detail')).toBeUndefined()
  })
})
