// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ExecutionWorkspacePanel from './ExecutionWorkspacePanel.vue'
import { useExecutionStore } from '../stores/executionStore'
import { useWorkflowStore } from '../stores/workflowStore'

const virtualizerState = vi.hoisted(() => ({
  virtualItems: [
    { index: 1, start: 30, end: 60 },
    { index: 2, start: 60, end: 90 },
  ] as Array<{ index: number; start: number; end: number }>,
  totalSize: 120,
}))

vi.mock('@tanstack/vue-virtual', () => ({
  useVirtualizer: () => ({
    value: {
      getVirtualItems: () => virtualizerState.virtualItems,
      getTotalSize: () => virtualizerState.totalSize,
      scrollToOffset: vi.fn(),
      measure: vi.fn(),
    },
  }),
}))

vi.mock('@vueuse/core', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@vueuse/core')>()
  return {
    ...actual,
    useResizeObserver: () => ({
      stop: vi.fn(),
    }),
  }
})

describe('ExecutionWorkspacePanel virtualization', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = `
      <div id="liveShellGrid">
        <div id="executionShellPane">
          <div id="executionVueShellMount"></div>
        </div>
      </div>
    `
  })

  it('renders spacer rows and the current virtual order window through the shared table composable', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('live')
    executionStore.orderRows = [
      { cells: ['2026-03-28 14:54:00', 'BUY', 'filled', '5', '5', '181.40', 'order-0'] },
      { cells: ['2026-03-28 14:55:00', 'SELL', 'filled', '6', '6', '182.40', 'order-1'] },
      { cells: ['2026-03-28 14:56:00', 'BUY', 'partial', '7', '7', '183.40', 'order-2'] },
      { cells: ['2026-03-28 14:57:00', 'SELL', 'queued', '8', '8', '184.40', 'order-3'] },
    ]

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const table = document.querySelector('.execOrdersTableVue')
    expect(table?.textContent || '').toContain('order-1')
    expect(table?.textContent || '').toContain('order-2')
    expect(table?.textContent || '').not.toContain('order-0')
    expect(table?.textContent || '').not.toContain('order-3')

    const spacerRows = Array.from(document.querySelectorAll<HTMLTableRowElement>('.execOrdersTableVue tbody tr.spacerRow'))
    expect(spacerRows).toHaveLength(2)
    expect(spacerRows.map((row) => row.querySelector('td')?.getAttribute('style'))).toEqual([
      'height: 30px;',
      'height: 30px;',
    ])
  })
})
