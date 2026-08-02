
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import PriceChartPanel from './PriceChartPanel.vue'
import { lightweightChartLegendColor } from '../../visualizations/chartTheme'
import { namedChartSeriesColor } from '../../visualizations/lightweightChartRuntime'

function normalizedBackground(value: string): string {
  const element = document.createElement('span')
  element.style.background = value
  return element.style.background
}

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
        stateKind: 'ready',
        renderReady: true,
        viewLabel: '0.0..80.0',
        navigation: true,
        pointInspection: true,
        qaPrefix: 'candidateChart',
        frameAriaLabel: 'Backtest price and trade chart.',
        hostAriaLabel: 'Backtest price chart',
        describedBy: 'drillKeyboardHint',
      },
    })

    expect(wrapper.get('.chartTitle').text()).toContain('Price + trades')
    expect(wrapper.get('#drillTvChartVue').attributes('role')).toBe('group')
    expect(wrapper.get('#drillTvChartVue').attributes('aria-roledescription')).toBe('price chart')
    expect(wrapper.get('#drillTvChartVue').attributes('aria-label')).toBe('Backtest price chart')
    expect(wrapper.get('#drillTvChartVue').attributes('tabindex')).toBe('0')
    expect(wrapper.get('#drillTvChartVue').attributes('aria-keyshortcuts')).toContain('ArrowLeft')
    expect(wrapper.get('.tvChartFrame').attributes('role')).toBe('group')
    expect(wrapper.get('.tvChartFrame').attributes('aria-describedby')).toContain('drillTvChartMetaVueDescription')
    expect(wrapper.get('#drillTvChartVue').attributes('aria-describedby')).toContain('drillTvChartMetaVueDescription')
    expect(wrapper.get('#drillTvChartVue').attributes('aria-describedby')).toContain('drillKeyboardHint')
    expect(wrapper.get('.tvChartFrame').attributes('data-chart-ready')).toBe('true')
    expect(wrapper.get('.tvChartFrame').attributes('aria-busy')).toBe('false')
    expect(wrapper.get('.chartStateBadge').attributes('data-chart-state')).toBe('ready')
    expect(wrapper.get('.chartEvidenceChips').text()).toContain('MA overlays')
    expect(wrapper.get('.chartLegendOverlay').text()).toContain('Fast MA')
    expect(wrapper.get('.chartLegendOverlay').text()).toContain('Trades')
    expect(wrapper.get('.tvChartFrame').attributes('data-chart-view')).toBe('0.0..80.0')
    expect(wrapper.get('[data-qa-action="candidateChart.panLeft"]').attributes('aria-label')).toBe('Pan chart left')
    expect(wrapper.get('[data-qa-action="candidateChart.panLeft"]').attributes('disabled')).toBeUndefined()
    await wrapper.get('[data-qa-action="candidateChart.panLeft"]').trigger('click')
    await wrapper.get('[data-qa-action="candidateChart.fitContent"]').trigger('click')
    await wrapper.get('[data-qa-action="candidateChart.panRight"]').trigger('click')
    expect(wrapper.emitted('pan-left')).toHaveLength(1)
    expect(wrapper.emitted('fit-content')).toHaveLength(1)
    expect(wrapper.emitted('pan-right')).toHaveLength(1)
    expect(wrapper.get('#drillTvChartMetaVue').text()).toContain('OHLC')
    expect(wrapper.get('#drillTvChartMetaVue').attributes('aria-live')).toBe('off')
    expect(wrapper.get('.chartInteractionHint').text()).toContain('Home/End jump')
    await wrapper.setProps({
      metaText: 'candles=4 · inspect[keyboard]: 2025-04-12 17:20:00 ET (UTC-04:00) · O 100 · H 101 · L 99 · C 100.5 · Fast MA 100.2',
    })
    expect(wrapper.get('#drillTvChartMetaVueDescription').text()).toContain('O 100 · H 101')
    expect(wrapper.get('.chartPointInspector').attributes('data-inspector-source')).toBe('keyboard')
    expect(wrapper.get('.chartPointInspector').text()).toContain('Keyboard point')
    expect(wrapper.get('.chartPointInspector').text()).toContain('2025-04-12 17:20:00 ET')
    expect(wrapper.get('.chartPointInspector').text()).toContain('Fast MA 100.2')
    expect(wrapper.get('#drillTvChartMetaVue').text()).toContain('candles=4')
    expect(wrapper.get('#drillTvChartMetaVue').text()).not.toContain('O 100')
    expect(wrapper.get('#drillTvChartMetaVueLifecycleStatus').text()).toBe('Chart ready')
    expect(wrapper.get('#drillTvChartMetaVueLifecycleStatus').text()).not.toContain('O=100')
    const accessibleDescription = wrapper.get('#drillTvChartMetaVueDescription').text()
    expect(accessibleDescription).toContain('4 price bars')
    expect(accessibleDescription).toContain('Fast MA 4')
    expect(accessibleDescription).toContain('Visible logical range 0.0..80.0')
    wrapper.unmount()
  })

  it('publishes the host element when mounted and null when unmounted', () => {
    const wrapper = mount(PriceChartPanel, {
      props: {
        chartId: 'execTvChartVue',
        metaId: 'execTvChartMetaVue',
        title: 'Execution chart',
        metaText: 'Loaded.',
        navigation: true,
        renderReady: true,
        stateKind: 'ready',
      },
    })

    const readyEvents = wrapper.emitted('host-ready')
    expect(readyEvents?.[0]?.[0]).toBeInstanceOf(HTMLElement)
    expect(wrapper.get('#execTvChartVue').attributes('tabindex')).toBeUndefined()
    expect(wrapper.get('#execTvChartVue').attributes('aria-keyshortcuts')).toBeUndefined()
    expect(wrapper.find('.chartInteractionHint').exists()).toBe(false)

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
    wrapper.unmount()
  })

  it('announces loading and error states while keeping navigation disabled', async () => {
    const wrapper = mount(PriceChartPanel, {
      props: {
        chartId: 'statefulChart',
        metaId: 'statefulChartMeta',
        title: 'Execution chart',
        metaText: 'Loading execution chart for AAPL.',
        stateLabel: 'Loading chart',
        stateKind: 'loading',
        renderReady: true,
        navigation: true,
        qaPrefix: 'executionChart',
      },
    })

    expect(wrapper.get('.tvChartFrame').attributes('aria-busy')).toBe('true')
    expect(wrapper.get('.chartStateBadge').attributes('data-chart-state')).toBe('loading')
    expect(wrapper.get('#statefulChartMeta').attributes('aria-live')).toBe('off')
    expect(wrapper.get('#statefulChartMetaLifecycleStatus').attributes('role')).toBe('status')
    expect(wrapper.get('[data-qa-action="executionChart.fitContent"]').attributes('disabled')).toBeDefined()

    await wrapper.setProps({
      metaText: 'Execution chart unavailable: price bars could not be loaded for AAPL.',
      stateLabel: 'Chart unavailable',
      stateKind: 'error',
      renderReady: true,
    })

    expect(wrapper.get('.tvChartFrame').attributes('aria-busy')).toBe('false')
    expect(wrapper.get('.tvChartFrame').attributes('data-chart-ready')).toBe('false')
    expect(wrapper.get('.chartStateBadge').attributes('data-chart-state')).toBe('error')
    expect(wrapper.get('#statefulChartMeta').attributes('aria-live')).toBe('off')
    expect(wrapper.get('#statefulChartMetaLifecycleStatus').attributes('role')).toBe('alert')
    expect(wrapper.get('#statefulChartMetaLifecycleStatus').attributes('aria-live')).toBe('assertive')
    expect(wrapper.get('[data-qa-action="executionChart.fitContent"]').attributes('disabled')).toBeDefined()
    expect(wrapper.text()).not.toContain('Chart ready')
    wrapper.unmount()
  })

  it('resolves palette-key swatches reactively and stops after unmount', async () => {
    document.documentElement.dataset.theme = 'dark'
    const wrapper = mount(PriceChartPanel, {
      props: {
        chartId: 'paletteChart',
        metaId: 'paletteMeta',
        title: 'Palette chart',
        metaText: 'Ready.',
        legendItems: [
          { label: 'Fast MA', paletteKey: 'fast' as const },
          { label: 'Candles', paletteKey: 'candles' as const },
          { label: 'Orders', paletteKey: 'orders' as const },
        ],
      },
    })
    const fastSwatch = wrapper.get('[data-chart-series="fast"]').element as HTMLElement
    const ordersSwatch = wrapper.get('[data-chart-series="orders"]').element as HTMLElement

    expect(fastSwatch.style.background).toBe(normalizedBackground(
      lightweightChartLegendColor('dark', 'fast'),
    ))
    expect(ordersSwatch.style.background).toBe(normalizedBackground(
      lightweightChartLegendColor('dark', 'orders'),
    ))

    document.documentElement.dataset.theme = 'light'
    await vi.waitFor(() => {
      expect(fastSwatch.style.background).toBe(normalizedBackground(
        lightweightChartLegendColor('light', 'fast'),
      ))
    })
    expect(ordersSwatch.style.background).toBe(normalizedBackground(
      lightweightChartLegendColor('light', 'orders'),
    ))

    wrapper.unmount()
    const colorAfterUnmount = fastSwatch.style.background
    document.documentElement.dataset.theme = 'dark'
    await Promise.resolve()
    expect(fastSwatch.style.background).toBe(colorAfterUnmount)
    delete document.documentElement.dataset.theme
  })

  it('renders native-series metadata, pane-aware height, and theme-reactive named colors', async () => {
    document.documentElement.dataset.theme = 'dark'
    const wrapper = mount(PriceChartPanel, {
      props: {
        chartId: 'nativeChart',
        metaId: 'nativeChartMeta',
        title: 'Price + strategy indicators + trades',
        metaText: 'Ready.',
        paneCount: 3,
        nativeStrategy: 'Vemr',
        nativeSeriesVisible: 3,
        nativeSeriesTotal: 4,
        legendItems: [
          { label: 'Bollinger upper', color: 'named-series:0', value: '3' },
          { label: 'Volume Z-score', color: 'named-histogram:1', value: '2' },
        ],
      },
    })

    const frame = wrapper.get('.tvChartFrame')
    const host = wrapper.get('#nativeChart')
    expect(frame.attributes('data-native-strategy')).toBe('Vemr')
    expect(frame.attributes('data-native-series-visible')).toBe('3')
    expect(frame.attributes('data-native-series-total')).toBe('4')
    expect(frame.attributes('data-native-pane-count')).toBe('3')
    expect(host.attributes('data-chart-pane-count')).toBe('3')
    expect(host.attributes('style')).toContain('clamp(480px, 60vh, 680px)')

    const firstSwatch = wrapper.get('[data-chart-series="named-series:0"]').element as HTMLElement
    const secondSwatch = wrapper.get('[data-chart-series="named-histogram:1"]').element as HTMLElement
    expect(firstSwatch.style.background).toBe(normalizedBackground(
      namedChartSeriesColor('dark', 0),
    ))
    expect(secondSwatch.style.background).toBe(normalizedBackground(
      lightweightChartLegendColor('dark', 'halftrend'),
    ))

    document.documentElement.dataset.theme = 'light'
    await vi.waitFor(() => {
      expect(firstSwatch.style.background).toBe(normalizedBackground(
        namedChartSeriesColor('light', 0),
      ))
    })
    expect(secondSwatch.style.background).toBe(normalizedBackground(
      lightweightChartLegendColor('light', 'halftrend'),
    ))

    wrapper.unmount()
    delete document.documentElement.dataset.theme
  })
})
