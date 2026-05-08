
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ParetoScatter from './ParetoScatter.vue'
import type { ParetoRenderModel } from '../../visualizations/paretoModel'

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
  meta: '2 points - x=max_dd - y=pnl',
  truthNote: 'Pareto points show tradeoffs.',
  representativeSummary: 'x-extreme #2 - y-extreme #2',
  confidenceNote: 'Validation has not run.',
  status: 'ready',
}
model.frontier = [...model.points].sort((left, right) => left.x - right.x)

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

describe('ParetoScatter', () => {
  beforeEach(() => {
    vi.restoreAllMocks()
    mockCanvasContext()
  })

  it('renders the legacy canvas ids from a serializable model', () => {
    const wrapper = mount(ParetoScatter, {
      props: { model },
    })

    expect(wrapper.find('#paretoWrapVue').exists()).toBe(true)
    expect(wrapper.find('#paretoMetaVue').text()).toContain('2 points')
    expect(wrapper.find('#paretoCanvasVue').exists()).toBe(true)
    expect(wrapper.text()).toContain('#1 - max_dd=0.2 - pnl=0.05')
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
})
