import { defineStore } from 'pinia'

export type LegacyBootPhase = 'idle' | 'booting' | 'ready' | 'error'
export type RuntimeLogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error'
export type DesktopNotificationPermission = 'unknown' | 'default' | 'granted' | 'denied' | 'unsupported'

export interface RuntimeLogEntry {
  id: string
  atUnixMs: number
  level: RuntimeLogLevel
  message: string
}

interface RuntimeState {
  legacyMounted: boolean
  bootPhase: LegacyBootPhase
  bootStage: string
  bootStartedAt: number | null
  bootReadyAt: number | null
  bootError: string | null
  bootErrorDetail: string | null
  desktopLogs: RuntimeLogEntry[]
  desktopLogConsoleAttached: boolean
  desktopWindowStateFile: string
  desktopWindowStateReady: boolean
  desktopNotificationPermission: DesktopNotificationPermission
}

const MAX_DESKTOP_LOGS = 24

export const useRuntimeStore = defineStore('runtime', {
  state: (): RuntimeState => ({
    legacyMounted: false,
    bootPhase: 'idle',
    bootStage: '',
    bootStartedAt: null,
    bootReadyAt: null,
    bootError: null,
    bootErrorDetail: null,
    desktopLogs: [],
    desktopLogConsoleAttached: false,
    desktopWindowStateFile: '',
    desktopWindowStateReady: false,
    desktopNotificationPermission: 'unknown',
  }),

  getters: {
    bootElapsedMs(state): number {
      if (!state.bootStartedAt) return 0
      const end = state.bootReadyAt ?? Date.now()
      return Math.max(0, end - state.bootStartedAt)
    },
  },

  actions: {
    setLegacyMounted(mounted: boolean): void {
      this.legacyMounted = mounted
    },

    markBooting(stage = 'Starting frontend runtime'): void {
      this.bootPhase = 'booting'
      this.bootStage = String(stage || '').trim() || 'Booting'
      this.bootError = null
      this.bootErrorDetail = null
      this.bootReadyAt = null
      if (!this.bootStartedAt) this.bootStartedAt = Date.now()
    },

    setBootStage(stage: string): void {
      const text = String(stage || '').trim()
      this.bootStage = text || this.bootStage || 'Booting'
      if (this.bootPhase === 'idle') this.bootPhase = 'booting'
      if (!this.bootStartedAt) this.bootStartedAt = Date.now()
    },

    markReady(): void {
      if (!this.bootStartedAt) this.bootStartedAt = Date.now()
      this.bootPhase = 'ready'
      this.bootStage = 'Frontend runtime ready'
      this.bootReadyAt = Date.now()
      this.bootError = null
      this.bootErrorDetail = null
    },

    markError(err: unknown): void {
      if (!this.bootStartedAt) this.bootStartedAt = Date.now()
      this.bootPhase = 'error'
      this.bootReadyAt = null
      this.bootStage = 'Frontend runtime failed'
      this.bootError = String(err instanceof Error ? err.message : err ?? 'Unknown error')
      const stack = err instanceof Error ? String(err.stack || '').trim() : ''
      if (stack) {
        this.bootErrorDetail = stack
        return
      }
      const raw = typeof err === 'string' ? err : (() => {
        try {
          return JSON.stringify(err)
        } catch {
          return String(err ?? '')
        }
      })()
      this.bootErrorDetail = String(raw || '').trim() || null
    },

    pushDesktopLog(level: RuntimeLogLevel, message: string): void {
      const text = String(message || '').trim()
      if (!text) return
      this.desktopLogs.unshift({
        id: `runtime_log_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
        atUnixMs: Date.now(),
        level,
        message: text,
      })
      if (this.desktopLogs.length > MAX_DESKTOP_LOGS) {
        this.desktopLogs.splice(MAX_DESKTOP_LOGS)
      }
    },

    clearDesktopLogs(): void {
      this.desktopLogs = []
    },

    setDesktopLogConsoleAttached(attached: boolean): void {
      this.desktopLogConsoleAttached = !!attached
    },

    setDesktopWindowState(file: string, ready: boolean): void {
      this.desktopWindowStateFile = String(file || '').trim()
      this.desktopWindowStateReady = !!ready
    },

    setDesktopNotificationPermission(permission: DesktopNotificationPermission): void {
      this.desktopNotificationPermission = permission
    },
  },
})
