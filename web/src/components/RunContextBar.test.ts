
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import RunContextBar, { type RunContextItem } from './RunContextBar.vue'

const primaryItems: RunContextItem[] = [
  { key: 'Dataset', value: 'AAPL', current: true, defaultVisible: true },
  { key: 'Status', value: 'Ready', current: false, defaultVisible: true },
]

const secondaryItems: RunContextItem[] = [
  { key: 'Rows', value: '500', current: false, defaultVisible: false },
  { key: 'Loaded ID', value: 'data-1', current: false, defaultVisible: false },
]

describe('RunContextBar', () => {
  it('preserves context-strip compatibility classes and overflow content', () => {
    const wrapper = mount(RunContextBar, {
      props: {
        items: primaryItems,
        secondaryItems,
        secondarySummary: 'Rows · Loaded ID',
      },
    })

    expect(wrapper.get('[aria-label="Run context"]').classes()).toContain('contextStrip')
    expect(wrapper.findAll('.contextStrip > .contextCard')).toHaveLength(2)
    expect(wrapper.get('.contextCard.current').text()).toContain('Dataset')
    expect(wrapper.get('.contextOverflowSummary').text()).toContain('Rows · Loaded ID')
    expect(wrapper.get('.contextOverflowPanel').text()).toContain('Loaded ID')
  })
})
