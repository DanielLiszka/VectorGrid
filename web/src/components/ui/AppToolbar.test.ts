// @vitest-environment jsdom
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppToolbar from './AppToolbar.vue'

describe('AppToolbar', () => {
  it('renders an accessible toolbar without adding command semantics to children', () => {
    const wrapper = mount(AppToolbar, {
      props: { ariaLabel: 'Chart actions', align: 'end' },
      slots: { default: '<button type="button">Export</button>' },
    })

    expect(wrapper.attributes('role')).toBe('toolbar')
    expect(wrapper.attributes('aria-label')).toBe('Chart actions')
    expect(wrapper.classes()).toContain('appToolbar-end')
    expect(wrapper.get('button').text()).toBe('Export')
  })
})
