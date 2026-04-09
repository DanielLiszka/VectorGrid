import { describe, expect, it, vi, beforeEach } from 'vitest'
import { ref } from 'vue'

const virtualMockState = vi.hoisted(() => ({
  virtualItems: [] as Array<{ index: number; start: number; end: number }>,
  totalSize: 0,
  measureCalls: 0,
  scrollCalls: [] as number[],
}))

const resizeObserverState = vi.hoisted(() => ({
  trigger: null as null | (() => void),
}))

vi.mock('@tanstack/vue-virtual', () => ({
  useVirtualizer: () => ({
    value: {
      getVirtualItems: () => virtualMockState.virtualItems,
      getTotalSize: () => virtualMockState.totalSize,
      measure: () => {
        virtualMockState.measureCalls += 1
      },
      scrollToOffset: (offset: number) => {
        virtualMockState.scrollCalls.push(offset)
      },
    },
  }),
}))

vi.mock('@vueuse/core', () => ({
  useResizeObserver: (_target: unknown, callback: () => void) => {
    resizeObserverState.trigger = callback
    return {
      stop: vi.fn(),
    }
  },
}))

import { useVirtualTableRows } from './useVirtualTableRows'

describe('useVirtualTableRows', () => {
  beforeEach(() => {
    virtualMockState.virtualItems = []
    virtualMockState.totalSize = 0
    virtualMockState.measureCalls = 0
    virtualMockState.scrollCalls = []
    resizeObserverState.trigger = null
  })

  it('maps virtual rows to source rows and computes spacer bounds', () => {
    virtualMockState.virtualItems = [
      { index: 1, start: 28, end: 56 },
      { index: 2, start: 56, end: 84 },
    ]
    virtualMockState.totalSize = 140

    const rows = ref([{ id: 'r0' }, { id: 'r1' }, { id: 'r2' }, { id: 'r3' }, { id: 'r4' }])
    const table = useVirtualTableRows(rows)

    expect(table.topSpacerPx.value).toBe(28)
    expect(table.bottomSpacerPx.value).toBe(56)
    expect(table.virtualRows.value.map((entry) => entry.index)).toEqual([1, 2])
    expect(table.virtualRows.value[0]?.row).toBe(rows.value[1])
    expect(table.virtualRows.value[1]?.row).toBe(rows.value[2])
  })

  it('keeps row identity stable for unchanged source objects', () => {
    virtualMockState.virtualItems = [{ index: 1, start: 28, end: 56 }]
    virtualMockState.totalSize = 84

    const row0 = { id: 'r0' }
    const row1 = { id: 'r1' }
    const row2 = { id: 'r2' }
    const rows = ref([row0, row1, row2])
    const table = useVirtualTableRows(rows)
    const initialRowRef = table.virtualRows.value[0]?.row

    expect(initialRowRef).toBe(rows.value[1])

    rows.value = [row0, row1, row2]
    expect(table.virtualRows.value[0]?.row).toBe(initialRowRef)
  })

  it('handles empty virtual windows and supports scrollToTop', () => {
    virtualMockState.virtualItems = []
    virtualMockState.totalSize = 0

    const rows = ref([{ id: 'r0' }])
    const table = useVirtualTableRows(rows)
    const scrollHost = { scrollTop: 220 } as unknown as HTMLElement
    scrollHost.scrollTop = 220
    table.scrollElementRef.value = scrollHost

    expect(table.topSpacerPx.value).toBe(0)
    expect(table.bottomSpacerPx.value).toBe(0)
    expect(table.virtualRows.value).toEqual([])

    table.scrollToTop()

    expect(scrollHost.scrollTop).toBe(0)
    expect(virtualMockState.scrollCalls).toEqual([0])
  })

  it('remeasures the virtualizer when the scroll host is resized', () => {
    const rows = ref([{ id: 'r0' }, { id: 'r1' }])
    const table = useVirtualTableRows(rows)
    table.scrollElementRef.value = {} as HTMLElement

    resizeObserverState.trigger?.()

    expect(virtualMockState.measureCalls).toBe(1)
  })
})
