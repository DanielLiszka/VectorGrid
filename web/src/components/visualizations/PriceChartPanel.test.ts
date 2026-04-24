// @vitest-environment jsdom
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PriceChartPanel from './PriceChartPanel.vue'

describe('PriceChartPanel', () => {
  it('renders a shared chart host with stable ids and live metadata', () => {
    const wrapper = mount(PriceChartPanel, {
      props: {
        chartId: 'drillTvChartVue',
        metaId: 'drillTvChartMetaVue',
        title: 'Price + trades (TradingView Lightweight Charts)',
        metaText: 'Hover for OHLC/MA values.',
        frameAriaLabel: 'Backtest price and trade chart.',
        hostAriaLabel: 'Backtest price chart',
        describedBy: 'drillKeyboardHint',
      },
    })

    expect(wrapper.get('.chartTitle').text()).toContain('Price + trades')
    expect(wrapper.get('#drillTvChartVue').attributes('role')).toBe('img')
    expect(wrapper.get('#drillTvChartVue').attributes('aria-label')).toBe('Backtest price chart')
    expect(wrapper.get('.tvChartFrame').attributes('aria-describedby')).toBe('drillKeyboardHint')
    expect(wrapper.get('#drillTvChartMetaVue').text()).toContain('OHLC')
  })

  it('publishes the host element when mounted and null when unmounted', () => {
    const wrapper = mount(PriceChartPanel, {
      props: {
        chartId: 'execTvChartVue',
        metaId: 'execTvChartMetaVue',
        title: 'Execution chart',
        metaText: 'Loaded.',
      },
    })

    const readyEvents = wrapper.emitted('host-ready')
    expect(readyEvents?.[0]?.[0]).toBeInstanceOf(HTMLElement)

    wrapper.unmount()

    const allReadyEvents = wrapper.emitted('host-ready') ?? []
    expect(allReadyEvents[allReadyEvents.length - 1]?.[0]).toBeNull()
  })
})
