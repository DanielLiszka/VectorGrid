import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppTableViewPresetControl from './AppTableViewPresetControl.vue'

describe('AppTableViewPresetControl', () => {
  it('renders view presets and emits the selected preset id', async () => {
    const wrapper = mount(AppTableViewPresetControl, {
      props: {
        label: 'Top view',
        activePresetId: 'risk',
        qaPrefix: 'top.table.view',
        presets: [
          { id: 'default', label: 'Default' },
          { id: 'risk', label: 'Risk', summary: 'Risk-first metrics' },
        ],
      },
    })

    expect(wrapper.get('[role="group"]').attributes('aria-label')).toBe('Top view')
    expect(wrapper.get('[data-qa="top.table.view.risk"]').attributes('aria-pressed')).toBe('true')

    await wrapper.get('[data-qa="top.table.view.default"]').trigger('click')

    expect(wrapper.emitted('apply')?.[0]).toEqual(['default'])
  })
})
