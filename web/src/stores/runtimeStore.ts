import { defineStore } from 'pinia'
import { redactKnownSecrets } from '../utils/redaction'

export type LegacyBootPhase = 'idle' | 'booting' | 'ready' | 'error'
export type RuntimeLogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error'
export type LatestArtifactHydrationPhase = 'idle' | 'loading' | 'ready' | 'error'
export type DesktopNotificationPermission = 'unknown' | 'default' | 'granted' | 'denied' | 'unsupported'
export type DesktopUpdaterStatus =
  | 'unknown'
  | 'unavailable'
  | 'idle'
  | 'checking'
  | 'up-to-date'
  | 'update-available'
  | 'error'
export const BOOT_STAGES = {
  STARTING_FRONTEND_RUNTIME: 'Starting frontend runtime',
  RESTORING_WORKSPACE_STATE: 'Restoring workspace state',
  INITIALIZING_DESKTOP_SERVICES: 'Initializing desktop services',
  FINALIZING_DATASET_READINESS: 'Finalizing dataset readiness',
  MOUNTING_WORKSPACE_SHELL: 'Mounting workspace shell',
  FRONTEND_RUNTIME_READY: 'Frontend runtime ready',
  FRONTEND_RUNTIME_FAILED: 'Frontend runtime failed',
  BOOTING_FALLBACK: 'Booting',
} as const

export type BootStage = (typeof BOOT_STAGES)[keyof typeof BOOT_STAGES]

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
  latestArtifactHydrationPhase: LatestArtifactHydrationPhase
  latestArtifactHydrationError: string
  latestArtifactHydrationAttempts: number
  desktopLogs: RuntimeLogEntry[]
  desktopLogConsoleAttached: boolean
  desktopWindowStateFile: string
  desktopWindowStateReady: boolean
  desktopNotificationPermission: DesktopNotificationPermission
  desktopUpdaterStatus: DesktopUpdaterStatus
  desktopUpdaterVersion: string
  desktopUpdaterError: string
  desktopUpdaterCheckedAt: number | null
}

const MAX_DESKTOP_LOGS = 24

function normalizeBootStage(stage: string | null | undefined, fallback: BootStage): string {
  const text = String(stage || '').trim()
  return text || fallback
}

export const useRuntimeStore = defineStore('runtime', {
  state: (): RuntimeState => ({
    legacyMounted: false,
    bootPhase: 'idle',
    bootStage: '',
    bootStartedAt: null,
    bootReadyAt: null,
    bootError: null,
    bootErrorDetail: null,
    latestArtifactHydrationPhase: 'idle',
    latestArtifactHydrationError: '',
    latestArtifactHydrationAttempts: 0,
    desktopLogs: [],
    desktopLogConsoleAttached: false,
    desktopWindowStateFile: '',
    desktopWindowStateReady: false,
    desktopNotificationPermission: 'unknown',
    desktopUpdaterStatus: 'unknown',
    desktopUpdaterVersion: '',
    desktopUpdaterError: '',
    desktopUpdaterCheckedAt: null,
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

    markBooting(stage: BootStage = BOOT_STAGES.STARTING_FRONTEND_RUNTIME): void {
      this.bootPhase = 'booting'
      this.bootStage = normalizeBootStage(stage, BOOT_STAGES.BOOTING_FALLBACK)
      this.bootError = null
      this.bootErrorDetail = null
      this.bootReadyAt = null
      this.bootStartedAt = Date.now()
    },

    setBootStage(stage: BootStage): void {
      this.bootStage = normalizeBootStage(stage, (this.bootStage as BootStage) || BOOT_STAGES.BOOTING_FALLBACK)
      if (this.bootPhase === 'idle') this.bootPhase = 'booting'
      if (!this.bootStartedAt) this.bootStartedAt = Date.now()
    },

    markReady(): void {
      if (!this.bootStartedAt) this.bootStartedAt = Date.now()
      this.bootPhase = 'ready'
      this.bootStage = BOOT_STAGES.FRONTEND_RUNTIME_READY
      this.bootReadyAt = Date.now()
      this.bootError = null
      this.bootErrorDetail = null
    },

    markError(err: unknown): void {
      if (!this.bootStartedAt) this.bootStartedAt = Date.now()
      this.bootPhase = 'error'
      this.bootReadyAt = null
      this.bootStage = BOOT_STAGES.FRONTEND_RUNTIME_FAILED
      this.bootError = redactKnownSecrets(err instanceof Error ? err.message : err ?? 'Unknown error')
      const stack = redactKnownSecrets(err instanceof Error ? String(err.stack || '').trim() : '')
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
      this.bootErrorDetail = redactKnownSecrets(raw).trim() || null
    },

    markLatestArtifactHydrationStarted(): void {
      this.latestArtifactHydrationPhase = 'loading'
      this.latestArtifactHydrationError = ''
      this.latestArtifactHydrationAttempts += 1
    },

    markLatestArtifactHydrationReady(): void {
      this.latestArtifactHydrationPhase = 'ready'
      this.latestArtifactHydrationError = ''
    },

    markLatestArtifactHydrationError(error: unknown): void {
      const message = redactKnownSecrets(
        error instanceof Error ? error.message : error ?? 'Unknown artifact restore error',
      ).trim()
      this.latestArtifactHydrationPhase = 'error'
      this.latestArtifactHydrationError = message || 'Unknown artifact restore error'
      this.pushDesktopLog(
        'warn',
        `Latest artifact restore failed: ${this.latestArtifactHydrationError}`,
      )
    },

    pushDesktopLog(level: RuntimeLogLevel, message: string): void {
      const text = redactKnownSecrets(message).trim()
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

    setDesktopUpdaterStatus(
      status: DesktopUpdaterStatus,
      version = '',
      error = '',
    ): void {
      this.desktopUpdaterStatus = status
      this.desktopUpdaterVersion = String(version || '').trim()
      this.desktopUpdaterError = redactKnownSecrets(error).trim()
      this.desktopUpdaterCheckedAt = status === 'idle' || status === 'unknown' ? null : Date.now()
    },
  },
})
