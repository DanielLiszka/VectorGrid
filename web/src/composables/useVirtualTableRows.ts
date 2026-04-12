import { useVirtualizer } from '@tanstack/vue-virtual'
import { useResizeObserver } from '@vueuse/core'
import { computed, ref, type Ref } from 'vue'

export interface VirtualTableRow<T> {
  index: number
  row: T
}

interface UseVirtualTableRowsOptions<T> {
  rowHeightPx?: number
  overscan?: number
  getItemKey?: (index: number, row: T) => string | number
}

export function useVirtualTableRows<T>(
  rows: Ref<readonly T[]>,
  options: UseVirtualTableRowsOptions<T> = {},
): {
  scrollElementRef: Ref<HTMLElement | null>
  virtualRows: Ref<VirtualTableRow<T>[]>
  topSpacerPx: Ref<number>
  bottomSpacerPx: Ref<number>
  scrollToTop: () => void
} {
  const { rowHeightPx = 28, overscan = 12, getItemKey } = options
  const scrollElementRef = ref<HTMLElement | null>(null)

  const virtualizer = useVirtualizer<HTMLElement, HTMLTableRowElement>(
    computed(() => ({
      count: rows.value.length,
      getScrollElement: () => scrollElementRef.value,
      estimateSize: () => rowHeightPx,
      overscan,
      getItemKey: (index: number) => {
        const row = rows.value[index]
        return row !== undefined && typeof getItemKey === 'function' ? getItemKey(index, row) : index
      },
    })),
  )

  useResizeObserver(scrollElementRef, () => {
    const maybeVirtualizer = virtualizer.value as { measure?: () => void }
    if (typeof maybeVirtualizer.measure === 'function') {
      maybeVirtualizer.measure()
    }
  })

  const virtualItems = computed(() => virtualizer.value.getVirtualItems())
  const totalSizePx = computed(() => virtualizer.value.getTotalSize())

  const topSpacerPx = computed(() => {
    const first = virtualItems.value[0]
    return first ? Math.max(0, Math.floor(first.start)) : 0
  })

  const bottomSpacerPx = computed(() => {
    const last = virtualItems.value[virtualItems.value.length - 1]
    if (!last) return 0
    return Math.max(0, Math.floor(totalSizePx.value - last.end))
  })

  const virtualRows = computed<VirtualTableRow<T>[]>(() => {
    const source = rows.value
    const items = virtualItems.value
    const result: VirtualTableRow<T>[] = []
    for (const item of items) {
      const row = source[item.index]
      if (row === undefined) continue
      result.push({ index: item.index, row })
    }
    return result
  })

  function scrollToTop(): void {
    const el = scrollElementRef.value
    if (el) el.scrollTop = 0
    virtualizer.value.scrollToOffset(0)
  }

  return {
    scrollElementRef,
    virtualRows,
    topSpacerPx,
    bottomSpacerPx,
    scrollToTop,
  }
}
