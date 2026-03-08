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

  it('traps Tab focus within the dialog and preserves tab order cycling', async () => {
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
    const commandRows = Array.from(document.querySelectorAll<HTMLButtonElement>('.cmdPaletteDialog .row'))

    expect(dialog).not.toBeNull()
    expect(input).not.toBeNull()
    expect(commandRows.length).toBeGreaterThan(0)
    const firstRow = commandRows[0]
    const lastRow = commandRows[commandRows.length - 1]

    input?.focus()
    dialog?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }))
    await nextTick()
    expect(document.activeElement).toBe(firstRow)
    expect(dialog?.contains(document.activeElement)).toBe(true)

    input?.focus()
    dialog?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true }))
    await nextTick()
    expect(document.activeElement).toBe(lastRow)
    expect(dialog?.contains(document.activeElement)).toBe(true)

    lastRow?.focus()
    dialog?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }))
    await nextTick()
    expect(dialog?.contains(document.activeElement)).toBe(true)

    firstRow?.focus()
    dialog?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true }))
    await nextTick()
    expect(dialog?.contains(document.activeElement)).toBe(true)

    wrapper.unmount()
  })
})
