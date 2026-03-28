import { defineStore } from 'pinia'

interface CommandPaletteState {
  open: boolean
  query: string
  selectedIndex: number
}

export const useCommandPaletteStore = defineStore('commandPalette', {
  state: (): CommandPaletteState => ({
    open: false,
    query: '',
    selectedIndex: 0,
  }),

  actions: {
    openPalette(): void {
      this.open = true
      this.query = ''
      this.selectedIndex = 0
    },

    closePalette(): void {
      this.open = false
      this.query = ''
      this.selectedIndex = 0
    },

    setQuery(value: string): void {
      this.query = String(value || '')
      this.selectedIndex = 0
    },

    setSelection(index: number, total: number): void {
      const max = Math.max(0, Math.floor(Number(total || 0)))
      if (max <= 0) {
        this.selectedIndex = 0
        return
      }
      this.selectedIndex = Math.max(0, Math.min(max - 1, Math.trunc(index)))
    },

    moveSelection(delta: number, total: number): void {
      const max = Math.max(0, Math.floor(Number(total || 0)))
      if (max <= 0) {
        this.selectedIndex = 0
        return
      }
      const next = Math.max(0, Math.min(max - 1, this.selectedIndex + Math.trunc(delta)))
      this.selectedIndex = next
    },

    clampSelection(total: number): void {
      const max = Math.max(0, Math.floor(Number(total || 0)))
      if (max <= 0) {
        this.selectedIndex = 0
        return
      }
      if (this.selectedIndex < 0) this.selectedIndex = 0
      if (this.selectedIndex >= max) this.selectedIndex = max - 1
    },
  },
})
