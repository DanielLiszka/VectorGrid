import { defineStore } from 'pinia'
import {
  invokeLegacyDistributedCancelRuntime,
  invokeLegacyDistributedFillFromPolygonRuntime,
  invokeLegacyDistributedGenerateTokenRuntime,
  invokeLegacyDistributedRunAssetsRuntime,
  invokeLegacyDistributedRunTilesRuntime,
  invokeLegacyDistributedStartWorkerRuntime,
  invokeLegacyDistributedStopWorkerRuntime,
  invokeLegacyDistributedUseCurrentCsvRuntime,
  invokeLegacyDistributedWorkerStatusRuntime,
} from '../legacy/actions'

function updateLegacyField(field: string, value: string, eventType: 'input' | 'change'): void {
  if (typeof document === 'undefined') return
  const key = String(field || '').trim()
  if (!key) return
  const el = document.getElementById(key) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
  if (!el || el.disabled) return
  if (String(el.value ?? '') !== value) el.value = value
  el.dispatchEvent(new Event(eventType, { bubbles: true }))
}

interface DistributedState {
  activeControlsTab: string
  leaderMode: string
  leaderModeHint: string
  workerStatus: string
  workerBindAddr: string
  workerToken: string
  leaderToken: string
  workersText: string
  assetsCsvText: string
  pathFrom: string
  pathTo: string
  workersCount: number
  assetPathCount: number
  tileCsvPath: string
  tilesPerWorker: string
  outputPreview: string
  statusLine: string
  isRunning: boolean
  canCancel: boolean
  canStartWorker: boolean
  canStopWorker: boolean
  canWorkerStatus: boolean
  canFillFromPolygon: boolean
  canUseCurrentCsv: boolean
  canRunAssets: boolean
  canRunTiles: boolean
  source: 'legacy-dom' | 'vue' | 'unknown'
  updatedAtUnixMs: number
}

export const useDistributedStore = defineStore('distributed', {
  state: (): DistributedState => ({
    activeControlsTab: 'data',
    leaderMode: 'assets',
    leaderModeHint: '—',
    workerStatus: 'Worker: —',
    workerBindAddr: '',
    workerToken: '',
    leaderToken: '',
    workersText: '',
    assetsCsvText: '',
    pathFrom: '',
    pathTo: '',
    workersCount: 0,
    assetPathCount: 0,
    tileCsvPath: '',
    tilesPerWorker: '4',
    outputPreview: '—',
    statusLine: 'Ready.',
    isRunning: false,
    canCancel: false,
    canStartWorker: true,
    canStopWorker: true,
    canWorkerStatus: true,
    canFillFromPolygon: true,
    canUseCurrentCsv: true,
    canRunAssets: true,
    canRunTiles: true,
    source: 'unknown',
    updatedAtUnixMs: 0,
  }),

  actions: {
    applyLegacySnapshot(snapshot: Partial<DistributedState>): void {
      this.activeControlsTab = String(snapshot.activeControlsTab ?? this.activeControlsTab ?? 'data').trim() || 'data'
      this.leaderMode = String(snapshot.leaderMode ?? this.leaderMode ?? 'assets').trim() || 'assets'
      this.leaderModeHint = String(snapshot.leaderModeHint ?? this.leaderModeHint ?? '—').trim() || '—'
      this.workerStatus = String(snapshot.workerStatus ?? this.workerStatus ?? 'Worker: —').trim() || 'Worker: —'
      this.workerBindAddr = String(snapshot.workerBindAddr ?? this.workerBindAddr ?? '').trim()
      this.workerToken = String(snapshot.workerToken ?? this.workerToken ?? '').trim()
      this.leaderToken = String(snapshot.leaderToken ?? this.leaderToken ?? '').trim()
      this.workersText = String(snapshot.workersText ?? this.workersText ?? '').trim()
      this.assetsCsvText = String(snapshot.assetsCsvText ?? this.assetsCsvText ?? '').trim()
      this.pathFrom = String(snapshot.pathFrom ?? this.pathFrom ?? '').trim()
      this.pathTo = String(snapshot.pathTo ?? this.pathTo ?? '').trim()
      this.workersCount = Math.max(0, Math.floor(Number(snapshot.workersCount ?? this.workersCount ?? 0)))
      this.assetPathCount = Math.max(0, Math.floor(Number(snapshot.assetPathCount ?? this.assetPathCount ?? 0)))
      this.tileCsvPath = String(snapshot.tileCsvPath ?? this.tileCsvPath ?? '').trim()
      this.tilesPerWorker = String(snapshot.tilesPerWorker ?? this.tilesPerWorker ?? '4').trim() || '4'
      this.outputPreview = String(snapshot.outputPreview ?? this.outputPreview ?? '—').trim() || '—'
      this.statusLine = String(snapshot.statusLine ?? this.statusLine ?? 'Ready.').trim() || 'Ready.'
      this.isRunning = Boolean(snapshot.isRunning ?? this.isRunning)
      this.canCancel = Boolean(snapshot.canCancel ?? this.canCancel)
      this.canStartWorker = Boolean(snapshot.canStartWorker ?? this.canStartWorker)
      this.canStopWorker = Boolean(snapshot.canStopWorker ?? this.canStopWorker)
      this.canWorkerStatus = Boolean(snapshot.canWorkerStatus ?? this.canWorkerStatus)
      this.canFillFromPolygon = Boolean(snapshot.canFillFromPolygon ?? this.canFillFromPolygon)
      this.canUseCurrentCsv = Boolean(snapshot.canUseCurrentCsv ?? this.canUseCurrentCsv)
      this.canRunAssets = Boolean(snapshot.canRunAssets ?? this.canRunAssets)
      this.canRunTiles = Boolean(snapshot.canRunTiles ?? this.canRunTiles)
      this.source = 'legacy-dom'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      updateLegacyField(field, String(value || ''), 'input')
    },

    setSelectField(field: string, value: string): void {
      updateLegacyField(field, String(value || ''), 'change')
    },

    generateToken(): void {
      void invokeLegacyDistributedGenerateTokenRuntime()
    },

    startWorker(): void {
      void invokeLegacyDistributedStartWorkerRuntime()
    },

    stopWorker(): void {
      void invokeLegacyDistributedStopWorkerRuntime()
    },

    refreshWorkerStatus(): void {
      void invokeLegacyDistributedWorkerStatusRuntime()
    },

    fillFromPolygon(): void {
      void invokeLegacyDistributedFillFromPolygonRuntime()
    },

    useCurrentCsv(): void {
      void invokeLegacyDistributedUseCurrentCsvRuntime()
    },

    runAssets(): void {
      void invokeLegacyDistributedRunAssetsRuntime()
    },

    runTiles(): void {
      void invokeLegacyDistributedRunTilesRuntime()
    },

    cancel(): void {
      void invokeLegacyDistributedCancelRuntime()
    },
  },
})
