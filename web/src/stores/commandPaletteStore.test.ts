import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useCommandPaletteStore } from './commandPaletteStore'

describe('commandPaletteStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('resets query and selection on open and close', () => {
    const store = useCommandPaletteStore()
    store.query = 'polygon'
    store.selectedIndex = 4

    store.openPalette()
    expect(store.open).toBe(true)
    expect(store.query).toBe('')
    expect(store.selectedIndex).toBe(0)

    store.query = 'history'
    store.selectedIndex = 2
    store.closePalette()
    expect(store.open).toBe(false)
    expect(store.query).toBe('')
    expect(store.selectedIndex).toBe(0)
  })

  it('clamps and moves selection within the current result bounds', () => {
    const store = useCommandPaletteStore()

    store.setSelection(8, 3)
    expect(store.selectedIndex).toBe(2)

    store.moveSelection(-5, 3)
    expect(store.selectedIndex).toBe(0)

    store.moveSelection(2, 3)
    expect(store.selectedIndex).toBe(2)

    store.setQuery('go')
    expect(store.selectedIndex).toBe(0)

    store.setSelection(5, 0)
    expect(store.selectedIndex).toBe(0)

    store.selectedIndex = 9
    store.clampSelection(2)
    expect(store.selectedIndex).toBe(1)
  })
})
