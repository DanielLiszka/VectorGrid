
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import HeatmapLegend from './HeatmapLegend.vue'

describe('HeatmapLegend', () => {
  it('keeps the legacy canvas and label ids used by the heatmap renderer', () => {
    const wrapper = mount(HeatmapLegend, {
      props: { note: 'Sequential plateau scale' },
    })

    expect(wrapper.attributes('role')).toBe('group')
    expect(wrapper.attributes('aria-label')).toBe(
      'Legend. Scale values are not available yet. Sequential plateau scale.',
    )
    expect(wrapper.find('#heatmapLegendVue').exists()).toBe(true)
    expect(wrapper.find('#heatmapLegendMinVue').exists()).toBe(true)
    expect(wrapper.find('#heatmapLegendMidVue').exists()).toBe(true)
    expect(wrapper.find('#heatmapLegendMaxVue').exists()).toBe(true)
    expect(wrapper.text()).toContain('Sequential plateau scale')
  })

  it('shows semantic anchors for perceptual sequential and diverging scales', async () => {
    const wrapper = mount(HeatmapLegend)

    ;(wrapper.vm as unknown as {
      paint: (
        range: { min: number; max: number; label: string },
        palette: {
          kind: 'diverging'
          negative: string
          zero: string
          positive: string
        },
        theme: 'light' | 'dark',
      ) => void
    }).paint(
      { min: -2, max: 6, label: 'asymmetric' },
      {
        kind: 'diverging',
        negative: '#B6405B',
        zero: '#F4F2EC',
        positive: '#006A73',
      },
      'light',
    )
    await wrapper.vm.$nextTick()

    expect(wrapper.attributes('data-palette-kind')).toBe('diverging')
    expect(wrapper.text()).toContain('Negative')
    expect(wrapper.text()).toContain('Neutral · 0')
    expect(wrapper.text()).toContain('Positive')
    expect(wrapper.find('#heatmapLegendMidVue').attributes('style')).toContain('left: 25%')
    expect(wrapper.find('#heatmapLegendMidVue').attributes('data-legend-anchor')).toBe('zero')
    expect(wrapper.attributes('aria-label')).toBe(
      'Legend. Negative -2; Neutral · 0; Positive 6.',
    )
    expect(wrapper.find('.heatmapLegendSemantics').attributes('aria-hidden')).toBe('true')
  })

  it('describes a one-sided domain without claiming that zero is visible', async () => {
    const wrapper = mount(HeatmapLegend, {
      props: { label: 'Objective legend' },
    })

    ;(wrapper.vm as unknown as {
      paint: (
        range: { min: number; max: number; label: string },
        palette: {
          kind: 'diverging'
          negative: string
          zero: string
          positive: string
        },
        theme: 'light' | 'dark',
      ) => void
    }).paint(
      { min: 2, max: 6, label: 'positive-only' },
      {
        kind: 'diverging',
        negative: '#B6405B',
        zero: '#F4F2EC',
        positive: '#006A73',
      },
      'light',
    )
    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('Lower positive')
    expect(wrapper.text()).not.toContain('Neutral · 0')
    expect(wrapper.find('#heatmapLegendMidVue').text()).toBe('4')
    expect(wrapper.find('#heatmapLegendMidVue').attributes('data-legend-anchor')).toBe('midpoint')
    expect(wrapper.attributes('aria-label')).toBe(
      'Objective legend. Lower positive 2; Positive 4; Higher positive 6.',
    )
  })
})
