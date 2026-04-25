
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppPanel from './AppPanel.vue'

describe('AppPanel', () => {
  it('preserves pane classes and renders optional heading content', () => {
    const wrapper = mount(AppPanel, {
      props: {
        eyebrow: 'Import',
        title: 'CSV input',
        variant: 'prominent',
        headingId: 'csv-input-heading',
      },
      slots: { default: '<p>Body</p>' },
    })

    expect(wrapper.classes()).toContain('pane')
    expect(wrapper.classes()).toContain('paneProminent')
    expect(wrapper.text()).toContain('Import')
    expect(wrapper.text()).toContain('CSV input')
    expect(wrapper.text()).toContain('Body')
    expect(wrapper.attributes('aria-labelledby')).toBe('csv-input-heading')
    expect(wrapper.get('h3').attributes('id')).toBe('csv-input-heading')
  })
})
