import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import CommandPaletteOverlay from './CommandPaletteOverlay.vue'
import { useCommandPaletteStore } from '../stores/commandPaletteStore'
import { useTaskCenterVisibility } from '../composables/useTaskCenterVisibility'
import { useWorkflowStore } from '../stores/workflowStore'

describe('CommandPaletteOverlay', () => {
  beforeEach(() => {
    localStorage.clear()
    localStorage.setItem('vta_ui_mode_v1', 'advanced')
    document.body.innerHTML = ''
    useTaskCenterVisibility().value = false
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

  it('uses fuzzy search so typo-tolerant queries still surface the intended command', async () => {
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

    paletteStore.setQuery('polygn')
    await nextTick()

    const commandRows = Array.from(document.querySelectorAll<HTMLElement>('.cmdPaletteDialog [role="option"]'))
    expect(commandRows.length).toBeGreaterThan(0)
    expect(commandRows[0]?.textContent).toContain('Open Polygon Downloader')

    const summary = document.querySelector<HTMLElement>('.cmdPaletteDialog .summary')
    expect(summary?.textContent).toContain('match')

    wrapper.unmount()
  })

  it('prefers direct name-prefix matches over incidental fuzzy hits for short queries', async () => {
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

    paletteStore.setQuery('go')
    await nextTick()

    const commandRows = Array.from(document.querySelectorAll<HTMLElement>('.cmdPaletteDialog [role="option"]'))
    expect(commandRows.length).toBeGreaterThan(0)
    expect(commandRows[0]?.textContent).toContain('Go to Workflow Step 1 (Data)')

    wrapper.unmount()
  })

  it('omits retired UI mode commands and routes advanced navigation', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const paletteStore = useCommandPaletteStore(pinia)
    const workflowStore = useWorkflowStore(pinia)
    paletteStore.openPalette()

    const wrapper = mount(CommandPaletteOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    paletteStore.setQuery('mode')
    await nextTick()
    const commandRows = Array.from(document.querySelectorAll<HTMLElement>('.cmdPaletteDialog [role="option"]'))
    expect(commandRows.map((row) => row.textContent || '').join('\n')).not.toContain('Switch to')

    paletteStore.openPalette()
    paletteStore.setQuery('go aggregate')
    await nextTick()
    let firstRow = document.querySelector<HTMLElement>('.cmdPaletteDialog [role="option"]')
    expect(firstRow?.textContent).toContain('Go to Aggregate')
    firstRow?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()
    expect(workflowStore.workspace).toBe('aggregate')

    paletteStore.openPalette()
    paletteStore.setQuery('go portfolio')
    await nextTick()
    firstRow = document.querySelector<HTMLElement>('.cmdPaletteDialog [role="option"]')
    expect(firstRow?.textContent).toContain('Go to Portfolio')
    firstRow?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()
    expect(workflowStore.workspace).toBe('portfolio')

    wrapper.unmount()
  })

  it('routes the shared task center command from palette search', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const paletteStore = useCommandPaletteStore(pinia)
    const taskCenterOpen = useTaskCenterVisibility()
    paletteStore.openPalette()

    const wrapper = mount(CommandPaletteOverlay, {
      global: { plugins: [pinia] },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    paletteStore.setQuery('task center')
    await nextTick()
    const firstRow = document.querySelector<HTMLElement>('.cmdPaletteDialog [role="option"]')
    expect(firstRow?.textContent).toContain('Open Task Center')
    firstRow?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await nextTick()

    expect(taskCenterOpen.value).toBe(true)
    expect(paletteStore.open).toBe(false)

    wrapper.unmount()
  })

  it('supports Home and End keyboard navigation across the filtered command set', async () => {
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
    const commandRows = Array.from(document.querySelectorAll<HTMLElement>('.cmdPaletteDialog [role="option"]'))
    const firstRow = commandRows[0]
    const lastRow = commandRows[commandRows.length - 1]

    dialog?.dispatchEvent(new KeyboardEvent('keydown', { key: 'End', bubbles: true }))
    await nextTick()
    expect(input?.getAttribute('aria-activedescendant')).toBe(lastRow?.id)

    dialog?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Home', bubbles: true }))
    await nextTick()
    expect(input?.getAttribute('aria-activedescendant')).toBe(firstRow?.id)

    wrapper.unmount()
  })

  it('resets the query and active selection when the palette is reopened', async () => {
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

    paletteStore.setQuery('go')
    await nextTick()

    const commandRows = Array.from(document.querySelectorAll<HTMLElement>('.cmdPaletteDialog [role="option"]'))
    expect(commandRows.length).toBeGreaterThan(3)
    paletteStore.setSelection(3, commandRows.length)
    await nextTick()

    expect(document.querySelector<HTMLInputElement>('.cmdPaletteDialog .query')?.value).toBe('go')
    paletteStore.closePalette()
    await nextTick()
    paletteStore.openPalette()
    await nextTick()
    await nextTick()

    const input = document.querySelector<HTMLInputElement>('.cmdPaletteDialog .query')
    const firstRow = document.querySelector<HTMLElement>('.cmdPaletteDialog [role="option"]')
    expect(input?.value).toBe('')
    expect(input?.getAttribute('aria-activedescendant')).toBe(firstRow?.id)

    wrapper.unmount()
  })
})
