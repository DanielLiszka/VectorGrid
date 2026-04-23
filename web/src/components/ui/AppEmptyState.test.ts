// @vitest-environment jsdom
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppEmptyState from './AppEmptyState.vue'

describe('AppEmptyState', () => {
  it('renders title, body, actions, hint, and blocked reason', () => {
    const wrapper = mount(AppEmptyState, {
      props: {
        title: 'No dataset loaded',
        body: 'Load CSV or Polygon candles before running optimization.',
        blockedReason: 'Run is blocked until a dataset is loaded.',
        hint: 'Recent datasets stay available in the data source chooser.',
      },
      slots: {
        actions: '<button type="button">Browse CSV</button><button type="button">Open Polygon</button>',
      },
    })

    expect(wrapper.get('.emptyTitle').text()).toBe('No dataset loaded')
    expect(wrapper.get('.emptyBody').text()).toContain('Load CSV')
    expect(wrapper.get('.emptyBlockedReason').attributes('role')).toBe('status')
    expect(wrapper.get('.emptyBlockedReason').text()).toContain('blocked')
    expect(wrapper.findAll('.emptyActions button')).toHaveLength(2)
    expect(wrapper.get('.appEmptyHint').text()).toContain('Recent datasets')
  })
})
