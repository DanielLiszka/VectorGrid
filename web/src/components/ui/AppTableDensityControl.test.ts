import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppTableDensityControl from './AppTableDensityControl.vue'

describe('AppTableDensityControl', () => {
  it('renders a segmented density selector and emits selected density', async () => {
    const wrapper = mount(AppTableDensityControl, {
      props: {
        density: 'comfortable',
        label: 'Top table density',
        qaPrefix: 'top.table.density',
      },
    })

    expect(wrapper.get('[role="group"]').attributes('aria-label')).toBe('Top table density')
    expect(wrapper.get('[data-qa="top.table.density.comfortable"]').attributes('aria-pressed')).toBe('true')

    await wrapper.get('[data-qa="top.table.density.compact"]').trigger('click')

    expect(wrapper.emitted('update:density')?.[0]).toEqual(['compact'])
  })
})
