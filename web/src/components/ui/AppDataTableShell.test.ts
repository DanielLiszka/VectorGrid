
import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import AppDataTableShell from './AppDataTableShell.vue'

describe('AppDataTableShell', () => {
  it('renders table chrome, controls, body, and keyboard hint', () => {
    const wrapper = mount(AppDataTableShell, {
      props: {
        title: 'Top candidates',
        summary: '128 rows · sorted by return',
        rowCount: 128,
        keyboardHint: 'Use arrow keys to move between rows.',
        toolbarLabel: 'Top candidates table controls',
        density: 'compact',
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
    expect(wrapper.get('.appDataTableControls').attributes('role')).toBe('toolbar')
    expect(wrapper.get('.appDataTableControls').attributes('aria-label')).toBe('Top candidates table controls')
    expect(wrapper.get('.appDataTableShell').attributes('data-table-density')).toBe('compact')
    expect(wrapper.get('.appDataTableShell').attributes('data-table-row-count')).toBe('128')
    expect(wrapper.get('.appDataTableShell').attributes('data-table-has-rows')).toBe('true')
    expect(wrapper.get('.appDataTableShell').attributes('data-table-summary')).toContain('sorted by return')
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
    expect(wrapper.get('.appDataTableShell').attributes('data-table-row-count')).toBe('0')
    expect(wrapper.get('.appDataTableShell').attributes('data-table-has-rows')).toBe('false')
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

  it('moves focus through slotted table rows and cells with grid navigation keys', async () => {
    const host = document.createElement('div')
    document.body.appendChild(host)
    const originalScrollIntoView = HTMLElement.prototype.scrollIntoView
    HTMLElement.prototype.scrollIntoView = vi.fn()

    try {
      const wrapper = mount(AppDataTableShell, {
        attachTo: host,
        props: {
          title: 'Keyboard table',
          rowCount: 2,
        },
        slots: {
          default: `
            <table>
              <tbody>
                <tr tabindex="0"><td>A1</td><td>A2</td></tr>
                <tr tabindex="0"><td>B1</td><td>B2</td></tr>
              </tbody>
            </table>
          `,
        },
      })

      const viewport = wrapper.get('.appDataTableViewport')
      const rows = wrapper.findAll('tbody tr')
      const viewportElement = viewport.element as HTMLElement
      viewportElement.focus()
      await viewport.trigger('keydown', { key: 'ArrowDown' })

      expect(document.activeElement).toBe(rows[0]?.element)

      await rows[0]!.trigger('keydown', { key: 'ArrowDown' })
      expect(document.activeElement).toBe(rows[1]?.element)

      await rows[1]!.trigger('keydown', { key: 'ArrowRight' })
      const secondRowCells = rows[1]!.findAll('td')
      expect(document.activeElement).toBe(secondRowCells[0]?.element)

      await secondRowCells[0]!.trigger('keydown', { key: 'ArrowRight' })
      expect(document.activeElement).toBe(secondRowCells[1]?.element)

      wrapper.unmount()
    } finally {
      HTMLElement.prototype.scrollIntoView = originalScrollIntoView
      host.remove()
    }
  })
})
