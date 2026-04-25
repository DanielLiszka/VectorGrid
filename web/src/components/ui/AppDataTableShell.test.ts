
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppDataTableShell from './AppDataTableShell.vue'

describe('AppDataTableShell', () => {
  it('renders table chrome, controls, body, and keyboard hint', () => {
    const wrapper = mount(AppDataTableShell, {
      props: {
        title: 'Top candidates',
        summary: '128 rows · sorted by return',
        rowCount: 128,
        keyboardHint: 'Use arrow keys to move between rows.',
      },
      slots: {
        actions: '<button type="button">Export</button>',
        controls: '<button type="button">Columns</button>',
        default: '<table><tbody><tr><td>Candidate 1</td></tr></tbody></table>',
      },
    })

    expect(wrapper.get('.appDataTableTitle').text()).toBe('Top candidates')
    expect(wrapper.get('.appDataTableSummary').text()).toContain('128 rows')
    expect(wrapper.find('.appDataTableActions button').text()).toBe('Export')
    expect(wrapper.find('.appDataTableControls button').text()).toBe('Columns')
    expect(wrapper.get('.appDataTableViewport').text()).toContain('Candidate 1')
    expect(wrapper.get('.appDataTableHint').text()).toContain('arrow keys')
  })

  it('renders an action-oriented empty state when no rows are available', () => {
    const wrapper = mount(AppDataTableShell, {
      props: {
        title: 'Validation rows',
        rowCount: 0,
        emptyTitle: 'No validation results',
        emptyBody: 'Run validation to populate this table.',
      },
      slots: {
        emptyActions: '<button type="button">Run validation</button>',
      },
    })

    expect(wrapper.find('.appDataTableViewport').exists()).toBe(false)
    expect(wrapper.get('.appEmptyState').text()).toContain('No validation results')
    expect(wrapper.get('.emptyActions button').text()).toBe('Run validation')
  })

  it('can delegate viewport framing to a specialized virtual table', () => {
    const wrapper = mount(AppDataTableShell, {
      props: {
        title: 'Validation rows',
        rowCount: 4,
        viewport: false,
      },
      slots: {
        default: '<div class="customVirtualViewport">Rows</div>',
      },
    })

    expect(wrapper.find('.appDataTableViewport').exists()).toBe(false)
    expect(wrapper.get('.appDataTableContent .customVirtualViewport').text()).toBe('Rows')
  })
})
