
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppStatusChip from './AppStatusChip.vue'

describe('AppStatusChip', () => {
  it('renders semantic status without button semantics', () => {
    const wrapper = mount(AppStatusChip, {
      props: { tone: 'warning', label: 'Route fallback' },
      attrs: { class: 'panelSummaryChip mono' },
    })

    expect(wrapper.element.tagName).toBe('SPAN')
    expect(wrapper.classes()).toContain('appStatusChip-warning')
    expect(wrapper.classes()).toContain('panelSummaryChip')
    expect(wrapper.attributes('aria-label')).toBe('Route fallback')
    expect(wrapper.find('button').exists()).toBe(false)
  })
})
