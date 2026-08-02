import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { nextTick } from 'vue'
import MetricDistributionChart from './MetricDistributionChart.vue'

const columns = [
  { key: 'pnl', label: 'PnL' },
  { key: 'max_dd', label: 'Max DD' },
]

describe('MetricDistributionChart', () => {
  it('renders an accessible preferred-metric histogram with summary stats and scaled bins', async () => {
    const wrapper = mount(MetricDistributionChart, {
      props: {
        title: 'Pareto metric distribution',
        preferredKeys: ['pnl'],
        columns,
        rows: [
          { cells: [{ text: '10', title: '10' }, { text: '2', title: '2' }] },
          { cells: [{ text: '11', title: '11' }, { text: '3', title: '3' }] },
          { cells: [{ text: '20', title: '20' }, { text: '5', title: '5' }] },
          { cells: [{ text: '40', title: '40' }, { text: '8', title: '8' }] },
        ],
      },
    })

    await nextTick()

    expect(wrapper.text()).toContain('Visual summary')
    expect(wrapper.text()).toContain('Pareto metric distribution')
    expect(wrapper.text()).toContain('PnL: n=4 min=10.0 median=15.5 max=40.0')

    const histogram = wrapper.get('.metricDistributionHistogram')
    expect(histogram.attributes('role')).toBe('img')
    expect(histogram.attributes('aria-label')).toContain(
      'PnL: n=4 min=10.0 median=15.5 max=40.0. Histogram bins:',
    )
    expect(histogram.attributes('aria-label')).toContain('candidates')
    expect(histogram.attributes('style')).toContain('--metric-bin-count: 10')
    expect(wrapper.find('canvas').exists()).toBe(false)

    const bars = wrapper.findAll('.metricDistributionBar')
    expect(bars).toHaveLength(10)
    expect(
      bars.reduce((sum, bar) => sum + Number(bar.attributes('data-bin-count') || 0), 0),
    ).toBe(4)
    expect(bars.filter((bar) => bar.classes().includes('hasValue'))).toHaveLength(3)
    expect(bars.filter((bar) => bar.classes().includes('isPeak'))).toHaveLength(1)
    expect(wrapper.findAll('.metricDistributionXAxis span')).toHaveLength(3)
  })

  it('updates the native histogram and accessible description when the metric changes', async () => {
    const wrapper = mount(MetricDistributionChart, {
      props: {
        title: 'Pareto metric distribution',
        preferredKeys: ['pnl'],
        columns,
        rows: [
          { cells: [{ text: '10' }, { text: '2' }] },
          { cells: [{ text: '20' }, { text: '5' }] },
          { cells: [{ text: '40' }, { text: '8' }] },
        ],
      },
    })

    await wrapper.get('select').setValue('max_dd')
    await nextTick()

    expect(wrapper.get('select').element.value).toBe('max_dd')
    expect(wrapper.get('.metricDistributionStats').text()).toBe(
      'Max DD: n=3 min=2 median=5 max=8',
    )
    expect(wrapper.get('.metricDistributionHistogram').attributes('aria-label')).toContain(
      'Max DD: n=3 min=2 median=5 max=8. Histogram bins:',
    )
    expect(
      wrapper.findAll('.metricDistributionBar')
        .reduce((sum, bar) => sum + Number(bar.attributes('data-bin-count') || 0), 0),
    ).toBe(3)
  })

  it('renders a single centered bin for a constant metric', async () => {
    const wrapper = mount(MetricDistributionChart, {
      props: {
        title: 'Constant metric',
        columns: [{ key: 'pnl', label: 'PnL' }],
        rows: [
          { cells: [{ text: '5' }] },
          { cells: [{ text: '5' }] },
        ],
      },
    })

    await nextTick()

    const bars = wrapper.findAll('.metricDistributionBar')
    expect(bars).toHaveLength(1)
    expect(bars[0]?.attributes('data-bin-label')).toBe('5')
    expect(bars[0]?.attributes('data-bin-count')).toBe('2')
    expect(bars[0]?.attributes('style')).toContain('height: 100%')
    expect(bars[0]?.classes()).toContain('isPeak')
    expect(wrapper.get('.metricDistributionXAxis').classes()).toContain('isSingle')
    expect(wrapper.get('.metricDistributionXAxis').text()).toBe('5')
    expect(wrapper.get('.metricDistributionHistogram').attributes('aria-label')).toContain(
      '5: 2 candidates',
    )
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
    expect(wrapper.find('[role="img"]').exists()).toBe(false)
  })
})
