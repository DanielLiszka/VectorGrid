
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppMetricStrip from './AppMetricStrip.vue'

describe('AppMetricStrip', () => {
  it('renders metric labels and clipped values with title fallback', () => {
    const wrapper = mount(AppMetricStrip, {
      props: {
        ariaLabel: 'Dataset metrics',
        items: [
          { key: 'rows', label: 'Rows', value: '500' },
          { key: 'range', label: 'Range', value: '2023-01 -> 2024-01', tone: 'accent', trend: '+12%' },
        ],
      },
    })

    expect(wrapper.element.tagName).toBe('DL')
    expect(wrapper.attributes('aria-label')).toBe('Dataset metrics')
    expect(wrapper.text()).toContain('Rows')
    expect(wrapper.text()).toContain('500')
    expect(wrapper.text()).toContain('+12%')
    expect(wrapper.find('.appMetricItem-accent').exists()).toBe(true)
    expect(wrapper.findAll('dd')[1]?.attributes('title')).toBe('2023-01 -> 2024-01')
  })
})
