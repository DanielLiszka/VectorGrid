
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import HeatmapLegend from './HeatmapLegend.vue'

describe('HeatmapLegend', () => {
  it('keeps the legacy canvas and label ids used by the heatmap renderer', () => {
    const wrapper = mount(HeatmapLegend, {
      props: { note: 'Sequential plateau scale' },
    })

    expect(wrapper.attributes('role')).toBe('group')
    expect(wrapper.attributes('aria-label')).toBe('Legend')
    expect(wrapper.find('#heatmapLegendVue').exists()).toBe(true)
    expect(wrapper.find('#heatmapLegendMinVue').exists()).toBe(true)
    expect(wrapper.find('#heatmapLegendMidVue').exists()).toBe(true)
    expect(wrapper.find('#heatmapLegendMaxVue').exists()).toBe(true)
    expect(wrapper.text()).toContain('Sequential plateau scale')
  })
})
