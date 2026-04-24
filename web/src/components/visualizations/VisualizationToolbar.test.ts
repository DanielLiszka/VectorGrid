// @vitest-environment jsdom
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import VisualizationToolbar from './VisualizationToolbar.vue'

describe('VisualizationToolbar', () => {
  it('provides a named toolbar surface with action slot support', () => {
    const wrapper = mount(VisualizationToolbar, {
      props: { label: 'Heatmap controls' },
      slots: {
        default: '<label>Mode<select><option>Objective</option></select></label>',
        actions: '<button type="button">Export PNG</button>',
      },
    })

    expect(wrapper.attributes('role')).toBe('toolbar')
    expect(wrapper.attributes('aria-label')).toBe('Heatmap controls')
    expect(wrapper.text()).toContain('Objective')
    expect(wrapper.text()).toContain('Export PNG')
  })
})
