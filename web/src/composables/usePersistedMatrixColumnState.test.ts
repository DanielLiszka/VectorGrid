import { beforeEach, describe, expect, it } from 'vitest'
import { nextTick, ref } from 'vue'
import { usePersistedMatrixColumnState } from './usePersistedMatrixColumnState'

describe('usePersistedMatrixColumnState', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('keeps newly added columns visible when persisted layouts predate schema expansion', () => {
    localStorage.setItem('matrix_columns', JSON.stringify({
      visibleColumnKeys: ['rank', 'name'],
      knownColumnKeys: ['rank', 'name'],
    }))

    const columns = ref([
      { key: 'rank', label: '#' },
      { key: 'name', label: 'Name' },
      { key: 'pnl', label: 'PnL' },
    ])

    const state = usePersistedMatrixColumnState(columns, 'matrix_columns')

    expect(state.visibleColumnKeys.value).toEqual(['rank', 'name', 'pnl'])
    expect(state.columnOptions.value).toEqual([
      { key: 'rank', label: '#', visible: true },
      { key: 'name', label: 'Name', visible: true },
      { key: 'pnl', label: 'PnL', visible: true },
    ])
    expect(state.columnVisibility.value).toEqual({})
    expect(state.hiddenCount.value).toBe(0)
    expect(state.canRestoreDefaults.value).toBe(false)
  })

  it('persists column order separately from source column indexes', async () => {
    const columns = ref([
      { key: 'rank', label: '#' },
      { key: 'name', label: 'Name' },
      { key: 'pnl', label: 'PnL' },
    ])

    const state = usePersistedMatrixColumnState(columns, 'matrix_columns')
    state.applyColumnViewPreset({
      visibleColumnKeys: ['pnl', 'rank'],
      columnOrderKeys: ['pnl', 'rank', 'name'],
    })
    await nextTick()

    expect(state.orderedVisibleColumnKeys.value).toEqual(['pnl', 'rank'])
    expect(state.orderedColumns.value.map((column) => column.key)).toEqual(['pnl', 'rank', 'name'])
    expect(state.orderedColumns.value.map((column) => column.sourceColumnIndex)).toEqual([2, 0, 1])
    expect(state.columnVisibility.value).toEqual({ name: false })
    expect(JSON.parse(localStorage.getItem('matrix_columns') || '{}')).toMatchObject({
      visibleColumnKeys: ['rank', 'pnl'],
      columnOrderKeys: ['pnl', 'rank', 'name'],
    })
  })

  it('appends newly added columns to persisted order and keeps them visible', () => {
    localStorage.setItem('matrix_columns', JSON.stringify({
      visibleColumnKeys: ['name'],
      knownColumnKeys: ['rank', 'name'],
      columnOrderKeys: ['name', 'rank'],
    }))

    const columns = ref([
      { key: 'rank', label: '#' },
      { key: 'name', label: 'Name' },
      { key: 'pnl', label: 'PnL' },
    ])

    const state = usePersistedMatrixColumnState(columns, 'matrix_columns')

    expect(state.visibleColumnKeys.value).toEqual(['name', 'pnl'])
    expect(state.columnOrderKeys.value).toEqual(['name', 'rank', 'pnl'])
    expect(state.orderedColumns.value.map((column) => column.sourceColumnIndex)).toEqual([1, 0, 2])
  })

  it('persists sanitized column widths with the existing column layout state', async () => {
    localStorage.setItem('matrix_columns', JSON.stringify({
      visibleColumnKeys: ['rank', 'name', 'pnl'],
      knownColumnKeys: ['rank', 'name', 'pnl'],
      columnOrderKeys: ['rank', 'name', 'pnl'],
      columnWidths: {
        rank: 12,
        pnl: 5000,
        stale: 220,
        name: 'wide',
      },
    }))

    const columns = ref([
      { key: 'rank', label: '#' },
      { key: 'name', label: 'Name' },
      { key: 'pnl', label: 'PnL' },
    ])

    const state = usePersistedMatrixColumnState(columns, 'matrix_columns')

    expect(state.columnWidths.value).toEqual({ rank: 48, pnl: 960 })
    expect(state.canRestoreDefaults.value).toBe(true)
    expect(state.columnWidthStyle('rank')).toEqual({
      width: '48px',
      minWidth: '48px',
      maxWidth: '48px',
    })

    state.setColumnWidth('name', 184.4)
    await nextTick()

    expect(state.columnWidths.value).toEqual({ rank: 48, pnl: 960, name: 184 })
    expect(JSON.parse(localStorage.getItem('matrix_columns') || '{}')).toMatchObject({
      columnWidths: { rank: 48, pnl: 960, name: 184 },
    })

    state.clearColumnWidth('rank')
    await nextTick()

    expect(state.columnWidths.value).toEqual({ pnl: 960, name: 184 })

    state.restoreDefaultColumns()
    await nextTick()

    expect(state.columnWidths.value).toEqual({})
    expect(localStorage.getItem('matrix_columns')).toBeNull()
  })
})
