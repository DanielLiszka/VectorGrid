// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { computed, nextTick, ref } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ExecutionWorkspacePanel from './ExecutionWorkspacePanel.vue'
import { useExecutionStore } from '../stores/executionStore'
import { useWorkflowStore } from '../stores/workflowStore'

vi.mock('../composables/useVirtualTableRows', () => ({
  useVirtualTableRows: (rows: { value: unknown[] }) => ({
    scrollElementRef: ref(null),
    virtualRows: computed(() => rows.value.map((row, index) => ({ index, row }))),
    topSpacerPx: computed(() => 0),
    bottomSpacerPx: computed(() => 0),
    scrollToTop: vi.fn(),
  }),
}))

describe('ExecutionWorkspacePanel', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = `
      <div id="settingsShellGrid">
        <div id="executionShellPane">
          <div id="executionVueShellMount"></div>
        </div>
      </div>
    `
  })

  it('renders the empty broker-order state when no orders exist', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    workflowStore.selectWorkspace('settings')

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const text = document.body.textContent || ''
    expect(text).toContain('Broker order activity')
    expect(text).toContain('No broker orders for current symbol.')
  })

  it('renders broker order rows through the shared headless table path', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const workflowStore = useWorkflowStore(pinia)
    const executionStore = useExecutionStore(pinia)
    workflowStore.selectWorkspace('settings')
    executionStore.orderRows = [
      {
        cells: [
          '2026-03-28 14:55:00',
          'BUY',
          'filled',
          '10',
          '10',
          '182.40',
          'order-1',
        ],
      },
    ]

    mount(ExecutionWorkspacePanel, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const tableText = document.querySelector('.execOrdersTableVue')?.textContent || ''
    expect(tableText).toContain('Time (UTC)')
    expect(tableText).toContain('BUY')
    expect(tableText).toContain('order-1')
  })
})
