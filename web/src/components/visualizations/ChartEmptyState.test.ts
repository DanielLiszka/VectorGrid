// @vitest-environment jsdom
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ChartEmptyState from './ChartEmptyState.vue'

describe('ChartEmptyState', () => {
  it('renders status copy and emits the optional action', async () => {
    const wrapper = mount(ChartEmptyState, {
      props: {
        title: 'Run an optimization first.',
        body: 'Chart output appears after candidate scoring.',
        status: 'No run loaded.',
        actionLabel: 'Review Top results',
      },
    })

    expect(wrapper.text()).toContain('Run an optimization first.')
    expect(wrapper.attributes('role')).toBe('status')
    await wrapper.get('button').trigger('click')
    expect(wrapper.emitted('action')).toHaveLength(1)
  })
})
