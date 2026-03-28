// @vitest-environment jsdom
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import CommandPaletteOverlay from './CommandPaletteOverlay.vue'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'

describe('CommandPaletteOverlay', () => {
  beforeEach(() => {
    localStorage.clear()
    document.body.innerHTML = ''
  })

  it('focuses the query input when opened and closes on Escape', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const paletteStore = useCommandPaletteStore(pinia)
    paletteStore.openPalette()

    const wrapper = mount(CommandPaletteOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const dialog = document.querySelector<HTMLElement>('.cmdPaletteDialog')
    expect(dialog).not.toBeNull()

    const input = document.querySelector<HTMLInputElement>('.cmdPaletteDialog .query')
    expect(input).not.toBeNull()
    expect(dialog?.contains(document.activeElement)).toBe(true)
    dialog?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await nextTick()

    expect(paletteStore.open).toBe(false)
    wrapper.unmount()
  })

  it('exposes combobox and listbox semantics for the active command selection', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const paletteStore = useCommandPaletteStore(pinia)
    paletteStore.openPalette()

    const wrapper = mount(CommandPaletteOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    const dialog = document.querySelector<HTMLElement>('.cmdPaletteDialog')
    const input = document.querySelector<HTMLInputElement>('.cmdPaletteDialog .query')
    const listbox = document.querySelector<HTMLElement>('.cmdPaletteDialog [role="listbox"]')
    const commandRows = Array.from(document.querySelectorAll<HTMLElement>('.cmdPaletteDialog [role="option"]'))

    expect(dialog).not.toBeNull()
    expect(input).not.toBeNull()
    expect(input?.getAttribute('role')).toBe('combobox')
    expect(input?.getAttribute('aria-controls')).toBe('commandPaletteListbox')
    expect(listbox?.id).toBe('commandPaletteListbox')
    expect(commandRows.length).toBeGreaterThan(0)
    const firstRow = commandRows[0]
    const secondRow = commandRows[1]

    expect(input?.getAttribute('aria-activedescendant')).toBe(firstRow?.id)
    expect(firstRow?.getAttribute('aria-selected')).toBe('true')

    dialog?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }))
    await nextTick()
    expect(input?.getAttribute('aria-activedescendant')).toBe(secondRow?.id)
    expect(secondRow?.getAttribute('aria-selected')).toBe('true')
    expect(firstRow?.getAttribute('aria-selected')).toBe('false')

    dialog?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true }))
    await nextTick()
    expect(input?.getAttribute('aria-activedescendant')).toBe(firstRow?.id)
    expect(firstRow?.getAttribute('aria-selected')).toBe('true')

    wrapper.unmount()
  })
})
