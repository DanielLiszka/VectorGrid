
import { enableAutoUnmount, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ParetoScatter from './ParetoScatter.vue'
import {
  PARETO_TRUTH_NOTE,
  type ParetoRenderModel,
} from '../../visualizations/paretoModel'

enableAutoUnmount(afterEach)

const model: ParetoRenderModel = {
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
  meta: '2 points from the retained shortlist - x=max_dd - y=pnl',
  truthNote: PARETO_TRUTH_NOTE,
  representativeSummary: 'x-extreme #2 - y-extreme #2',
  confidenceNote: 'Validation has not run.',
  status: 'ready',
}
model.frontier = [...model.points].sort((left, right) => left.x - right.x)

function mockCanvasContext(): {
  fillStyles: string[]
  strokeStyles: string[]
} {
  let fillStyle = ''
  let strokeStyle = ''
  const fillStyles: string[] = []
  const strokeStyles: string[] = []
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
    stroke: vi.fn(() => {
      strokeStyles.push(strokeStyle)
    }),
    arc: vi.fn(),
    fill: vi.fn(() => {
      fillStyles.push(fillStyle)
    }),
    font: '',
    get fillStyle() {
      return fillStyle
    },
    set fillStyle(value: string) {
      fillStyle = value
    },
    get strokeStyle() {
      return strokeStyle
    },
    set strokeStyle(value: string) {
      strokeStyle = value
    },
    lineWidth: 1,
  }
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(context as unknown as CanvasRenderingContext2D)
  return { fillStyles, strokeStyles }
}

function setElementRect(element: Element, width: number, height: number): void {
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

let canvasColors: ReturnType<typeof mockCanvasContext>

describe('ParetoScatter', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    canvasColors = mockCanvasContext()
    document.documentElement.dataset.theme = 'light'
  })

  afterEach(() => {
    delete document.documentElement.dataset.theme
  })

  it('renders the legacy canvas ids from a serializable model', () => {
    const wrapper = mount(ParetoScatter, {
      props: { model },
    })

    expect(wrapper.find('#paretoWrapVue').exists()).toBe(true)
    expect(wrapper.find('#paretoMetaVue').text()).toContain('2 points')
    expect(wrapper.find('#paretoCanvasVue').exists()).toBe(true)
    expect(wrapper.text()).toContain('#1 - max_dd=0.2 - pnl=0.05')
    expect(wrapper.text()).toContain('computed only within the retained shortlist')
    expect(wrapper.get('.paretoChartSurface').attributes('aria-label')).toContain('arrow keys')
    expect(wrapper.get('#paretoCanvasVue').attributes('aria-label')).toContain('not a global search-space frontier')
  })

  it('supports keyboard point navigation and activation', async () => {
    const wrapper = mount(ParetoScatter, {
      props: { model },
    })
    const chartSurface = wrapper.find('.paretoChartSurface')

    expect(chartSurface.attributes('role')).toBe('group')
    await chartSurface.trigger('keydown', { key: 'ArrowRight' })
    expect(wrapper.text()).toContain('#2 - max_dd=0.1 - pnl=0.12')

    await chartSurface.trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('select-point')?.[0]?.[0]).toMatchObject({
      rowId: 'row-2',
      rank: 2,
    })
  })

  it('maps narrow rendered coordinates back to the logical canvas for point hit testing', async () => {
    vi.spyOn(window, 'devicePixelRatio', 'get').mockReturnValue(4)
    const wrapper = mount(ParetoScatter, {
      props: { model },
    })
    const canvas = wrapper.get<HTMLCanvasElement>('#paretoCanvasVue')
    setElementRect(canvas.element, 240, 240)
    await wrapper.vm.$nextTick()

    const logicalWidth = 320
    const logicalHeight = 240
    const plotWidth = logicalWidth - 56 - 18
    const plotHeight = logicalHeight - 18 - 34
    const logicalX = 56 + ((model.points[0]!.x - model.xRange.min) / (model.xRange.max - model.xRange.min)) * plotWidth
    const logicalY = 18 + (
      plotHeight -
      ((model.points[0]!.y - model.yRange.min) / (model.yRange.max - model.yRange.min)) * plotHeight
    )

    await canvas.trigger('click', {
      clientX: logicalX * (240 / logicalWidth),
      clientY: logicalY,
    })

    expect(canvas.attributes('data-pixel-ratio')).toBe('2')
    expect(canvas.attributes('data-logical-width')).toBe('320')
    expect(canvas.element.width).toBe(640)
    expect(wrapper.emitted('select-point')?.[0]?.[0]).toMatchObject({
      rowId: 'row-1',
      rank: 1,
    })
  })

  it('preserves wheel scrolling but consumes single-point navigation keys', async () => {
    const onePointModel: ParetoRenderModel = {
      ...model,
      points: [model.points[0]!],
      frontier: [model.points[0]!],
    }
    const wrapper = mount(ParetoScatter, {
      props: { model: onePointModel },
    })
    const chartSurface = wrapper.get('.paretoChartSurface')
    const wheelEvent = new WheelEvent('wheel', {
      bubbles: true,
      cancelable: true,
      deltaY: 120,
    })
    chartSurface.element.dispatchEvent(wheelEvent)
    const arrowEvent = new KeyboardEvent('keydown', {
      bubbles: true,
      cancelable: true,
      key: 'ArrowDown',
    })
    chartSurface.element.dispatchEvent(arrowEvent)

    expect(wheelEvent.defaultPrevented).toBe(false)
    expect(arrowEvent.defaultPrevented).toBe(true)

    await chartSurface.trigger('keydown', { key: 'Enter' })
    expect(wrapper.emitted('select-point')?.[0]?.[0]).toMatchObject({
      rowId: 'row-1',
      rank: 1,
    })
  })

  it('consumes Home and End at selection boundaries while preserving selection', async () => {
    const wrapper = mount(ParetoScatter, {
      props: { model },
    })
    const chartSurface = wrapper.get('.paretoChartSurface')
    const homeAtStart = new KeyboardEvent('keydown', {
      bubbles: true,
      cancelable: true,
      key: 'Home',
    })
    chartSurface.element.dispatchEvent(homeAtStart)
    const end = new KeyboardEvent('keydown', {
      bubbles: true,
      cancelable: true,
      key: 'End',
    })
    chartSurface.element.dispatchEvent(end)
    await wrapper.vm.$nextTick()

    expect(homeAtStart.defaultPrevented).toBe(true)
    expect(end.defaultPrevented).toBe(true)
    expect(wrapper.text()).toContain('#2 - max_dd=0.1 - pnl=0.12')

    const endAtEnd = new KeyboardEvent('keydown', {
      bubbles: true,
      cancelable: true,
      key: 'End',
    })
    chartSurface.element.dispatchEvent(endAtEnd)
    expect(endAtEnd.defaultPrevented).toBe(true)
    expect(wrapper.text()).toContain('#2 - max_dd=0.1 - pnl=0.12')
  })

  it('redraws with contrast-safe light and dark chart palettes', async () => {
    const wrapper = mount(ParetoScatter, {
      props: { model },
    })
    await wrapper.vm.$nextTick()

    expect(canvasColors.fillStyles).toContain('#047857')
    expect(canvasColors.fillStyles).toContain('#0369a1')
    expect(canvasColors.strokeStyles).toContain('#854d0e')

    document.documentElement.dataset.theme = 'dark'
    await vi.waitFor(() => {
      expect(canvasColors.fillStyles).toContain('#22c55e')
      expect(canvasColors.fillStyles).toContain('#38bdf8')
      expect(canvasColors.strokeStyles).toContain('#fde047')
    })
  })

  it('disconnects its theme observer when unmounted', () => {
    const disconnect = vi.spyOn(MutationObserver.prototype, 'disconnect')
    const wrapper = mount(ParetoScatter, {
      props: { model },
    })

    wrapper.unmount()

    expect(disconnect).toHaveBeenCalled()
  })
})
