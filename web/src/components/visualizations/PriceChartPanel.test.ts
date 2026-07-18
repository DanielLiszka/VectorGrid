
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PriceChartPanel from './PriceChartPanel.vue'

describe('PriceChartPanel', () => {
  it('renders a shared chart host with stable ids and live metadata', async () => {
    const wrapper = mount(PriceChartPanel, {
      props: {
        chartId: 'drillTvChartVue',
        metaId: 'drillTvChartMetaVue',
        title: 'Price + trades (TradingView Lightweight Charts)',
        metaText: 'Hover for OHLC/MA values.',
        summaryItems: ['4 price bars', '2 trades', 'MA overlays'],
        legendItems: [
          { label: 'Fast MA', color: '#0ea5e9', value: '4' },
          { label: 'Trades', color: '#e11d48', value: '2' },
        ],
        stateLabel: 'Chart ready',
        renderReady: true,
        viewLabel: '0.0..80.0',
        navigation: true,
        qaPrefix: 'candidateChart',
        frameAriaLabel: 'Backtest price and trade chart.',
        hostAriaLabel: 'Backtest price chart',
        describedBy: 'drillKeyboardHint',
      },
    })

    expect(wrapper.get('.chartTitle').text()).toContain('Price + trades')
    expect(wrapper.get('#drillTvChartVue').attributes('role')).toBe('img')
    expect(wrapper.get('#drillTvChartVue').attributes('aria-label')).toBe('Backtest price chart')
    expect(wrapper.get('.tvChartFrame').attributes('role')).toBe('group')
    expect(wrapper.get('.tvChartFrame').attributes('aria-describedby')).toBe('drillKeyboardHint')
    expect(wrapper.get('.tvChartFrame').attributes('data-chart-ready')).toBe('true')
    expect(wrapper.get('.chartStateBadge').attributes('data-chart-state')).toBe('ready')
    expect(wrapper.get('.chartEvidenceChips').text()).toContain('MA overlays')
    expect(wrapper.get('.chartLegendOverlay').text()).toContain('Fast MA')
    expect(wrapper.get('.chartLegendOverlay').text()).toContain('Trades')
    expect(wrapper.get('.tvChartFrame').attributes('data-chart-view')).toBe('0.0..80.0')
    expect(wrapper.get('[data-qa-action="candidateChart.panLeft"]').attributes('aria-label')).toBe('Pan chart left')
    await wrapper.get('[data-qa-action="candidateChart.fitContent"]').trigger('click')
    expect(wrapper.emitted('fit-content')).toHaveLength(1)
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

  it('keeps dense evidence and legends compact on the chart surface', () => {
    const wrapper = mount(PriceChartPanel, {
      props: {
        chartId: 'denseChart',
        metaId: 'denseMeta',
        title: 'Dense chart',
        metaText: 'Ready.',
        summaryItems: [
          '240 price bars',
          '42 trades',
          'MA overlays',
          'HalfTrend regime',
          'OTT',
          'MAC-Z',
        ],
        legendItems: [
          { label: 'Candles', color: '#16a34a', value: '240' },
          { label: 'Fast MA', color: '#0ea5e9', value: '240' },
          { label: 'Slow MA', color: '#f59e0b', value: '240' },
          { label: 'HalfTrend', color: '#22c55e', value: '240' },
          { label: 'OTT', color: '#a855f7', value: '240' },
          { label: 'MAC-Z', color: '#94a3b8', value: '240' },
          { label: 'Entries', color: '#22c55e', value: '21' },
          { label: 'Exits', color: '#f43f5e', value: '21' },
        ],
      },
    })

    expect(wrapper.findAll('.chartEvidenceChip')).toHaveLength(6)
    expect(wrapper.get('.chartEvidenceMore').text()).toBe('+1')
    expect(wrapper.get('.chartEvidenceChips').text()).toContain('OTT')
    expect(wrapper.get('.chartEvidenceChips').text()).not.toContain('HalfTrend')
    expect(wrapper.findAll('.chartLegendItem')).toHaveLength(8)
    expect(wrapper.get('.chartLegendMore').text()).toBe('+1')
    expect(wrapper.get('.chartLegendOverlay').text()).toContain('Entries')
    expect(wrapper.get('.chartLegendOverlay').text()).toContain('Exits')
    expect(wrapper.get('.chartLegendOverlay').text()).not.toContain('Candles')
  })
})
