
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppButton from './AppButton.vue'

describe('AppButton', () => {
  it('renders a standard button while preserving click handlers and primary styling', async () => {
    let clicks = 0
    const wrapper = mount(AppButton, {
      props: { variant: 'primary', size: 'sm' },
      attrs: {
        onClick: () => {
          clicks += 1
        },
      },
      slots: { default: 'Run optimization' },
    })

    const button = wrapper.get('button')
    expect(button.classes()).toContain('primary')
    expect(button.attributes('type')).toBe('button')

    await button.trigger('click')

    expect(clicks).toBe(1)
  })

  it('surfaces disabled reasons through the title attribute', () => {
    const wrapper = mount(AppButton, {
      props: {
        disabled: true,
        disabledReason: 'Load data first.',
      },
      slots: { default: 'Run' },
    })

    const button = wrapper.get('button')
    expect(button.attributes('disabled')).toBeDefined()
    expect(button.attributes('title')).toBe('Load data first.')
  })

  it('disables and announces loading state', () => {
    const wrapper = mount(AppButton, {
      props: {
        loading: true,
        loadingLabel: 'Running',
      },
      slots: { default: 'Run' },
    })

    const button = wrapper.get('button')
    expect(button.attributes('disabled')).toBeDefined()
    expect(button.attributes('aria-busy')).toBe('true')
    expect(button.text()).toBe('Running')
    expect(wrapper.find('.appButtonSpinner').exists()).toBe(true)
  })

  it('uses an explicit title as the accessible name for icon-only buttons', () => {
    const wrapper = mount(AppButton, {
      props: {
        iconOnly: true,
        title: 'Open chart settings',
      },
    })

    const button = wrapper.get('button')
    expect(button.attributes('title')).toBe('Open chart settings')
    expect(button.attributes('aria-label')).toBe('Open chart settings')
  })
})
