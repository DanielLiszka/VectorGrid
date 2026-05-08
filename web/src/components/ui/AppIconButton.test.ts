
import { mount } from '@vue/test-utils'
import { Info } from '@lucide/vue'
import { describe, expect, it } from 'vitest'
import AppIconButton from './AppIconButton.vue'

describe('AppIconButton', () => {
  it('renders an icon-only button with a required accessible name', () => {
    const wrapper = mount(AppIconButton, {
      props: {
        icon: Info,
        label: 'Open task center',
        size: 'sm',
      },
    })

    const button = wrapper.get('button')
    expect(button.attributes('aria-label')).toBe('Open task center')
    expect(button.attributes('title')).toBe('Open task center')
    expect(button.text()).toBe('')
    expect(button.classes()).toContain('appIconButton-sm')
  })

  it('surfaces disabled reasons through the title attribute', () => {
    const wrapper = mount(AppIconButton, {
      props: {
        icon: Info,
        label: 'Run',
        disabled: true,
        disabledReason: 'Load data first.',
      },
    })

    const button = wrapper.get('button')
    expect(button.attributes('disabled')).toBeDefined()
    expect(button.attributes('aria-label')).toBe('Run')
    expect(button.attributes('title')).toBe('Load data first.')
  })

  it('falls back to the title for the accessible name when label text is empty', () => {
    const wrapper = mount(AppIconButton, {
      props: {
        icon: Info,
        label: ' ',
        title: 'Inspect details',
      },
    })

    const button = wrapper.get('button')
    expect(button.attributes('aria-label')).toBe('Inspect details')
    expect(button.attributes('title')).toBe('Inspect details')
  })
})
