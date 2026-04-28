import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppMetricCell from './AppMetricCell.vue'

describe('AppMetricCell', () => {
  it('renders text unchanged while adding a decorative metric bar', () => {
    const wrapper = mount(AppMetricCell, {
      props: {
        text: '123.45',
        align: 'right',
        visual: {
          kind: 'bar',
          tone: 'good',
          fillPct: 72,
          label: 'pnl 123.45',
        },
      },
    })

    expect(wrapper.text()).toBe('123.45')
    expect(wrapper.attributes('data-visual-kind')).toBe('bar')
    expect(wrapper.attributes('aria-label')).toBe('pnl 123.45')
    expect(wrapper.attributes('style')).toContain('--metric-fill: 72%')
    expect(wrapper.find('.metricCellFill').attributes('aria-hidden')).toBe('true')
  })

  it('supports status badge cells and keeps slotted controls interactive', () => {
    const wrapper = mount(AppMetricCell, {
      props: {
        text: 'pass',
        align: 'left',
        visual: {
          kind: 'badge',
          tone: 'good',
          fillPct: 0,
          label: 'gate pass',
        },
      },
      slots: {
        default: '<button type="button">Pin</button>',
      },
    })

    expect(wrapper.classes()).toContain('align-left')
    expect(wrapper.attributes('data-visual-kind')).toBe('badge')
    expect(wrapper.text()).toContain('pass')
    expect(wrapper.find('button').text()).toBe('Pin')
  })
})
