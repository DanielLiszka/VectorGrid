import { defineStore } from 'pinia'
import {
  cancel as cancelAction,
  fillFromPolygon as fillFromPolygonAction,
  generateToken as generateTokenAction,
  refreshWorkerStatus as refreshWorkerStatusAction,
  runAssets as runAssetsAction,
  runTiles as runTilesAction,
  startWorker as startWorkerAction,
  stopWorker as stopWorkerAction,
  useCurrentCsv as useCurrentCsvAction,
} from '../services/distributedService'
import { useDataStore } from './dataStore'

function countNonEmptyLines(value: string): number {
  return String(value || '')
    .split(/\r?\n/g)
    .map((line) => String(line || '').trim())
    .filter((line) => line.length > 0)
    .length
}

function applyVueFieldUpdate(state: DistributedState, field: string, value: string): boolean {
  const key = String(field || '').trim()
  const next = String(value || '').trim()
  if (!key) return false
  switch (key) {
    case 'distLeaderMode':
      state.leaderMode = next || 'assets'
      return true
    case 'distWorkerBindAddr':
      state.workerBindAddr = next
      return true
    case 'distWorkerToken':
      state.workerToken = next
      return true
    case 'distLeaderToken':
      state.leaderToken = next
      return true
    case 'distLeaderWorkers':
      state.workersText = next
      state.workersCount = countNonEmptyLines(next)
      return true
    case 'distAssetsCsvs':
      state.assetsCsvText = next
      state.assetPathCount = countNonEmptyLines(next)
      return true
    case 'distPathFrom':
      state.pathFrom = next
      return true
    case 'distPathTo':
      state.pathTo = next
      return true
    case 'distTileCsvPath':
      state.tileCsvPath = next
      return true
    case 'distTilesPerWorker':
      state.tilesPerWorker = next
      return true
    default:
      return false
  }
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
  source: 'vue' | 'unknown'
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
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
    },

    setInputField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      }
    },

    setSelectField(field: string, value: string): void {
      const key = String(field || '').trim()
      const next = String(value || '')
      if (applyVueFieldUpdate(this, key, next)) {
        this.source = 'vue'
        this.updatedAtUnixMs = Date.now()
      }
    },

    generateToken(): void {
      void generateTokenAction()
        .then((token) => {
          const nextToken = String(token || '').trim()
          if (!nextToken) return
          this.workerToken = nextToken
          this.leaderToken = nextToken
          this.statusLine = 'Generated distributed auth token.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = `Token generation failed: ${String(error || '')}`.trim()
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    startWorker(): void {
      void startWorkerAction(this.workerBindAddr, this.workerToken)
        .then((status) => {
          this.workerStatus = String(status?.text || this.workerStatus || 'Worker: —').trim() || 'Worker: —'
          this.statusLine = 'Worker start requested.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = `Worker start failed: ${String(error || '')}`.trim()
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    stopWorker(): void {
      void stopWorkerAction()
        .then((status) => {
          this.workerStatus = String(status?.text || this.workerStatus || 'Worker: —').trim() || 'Worker: —'
          this.statusLine = 'Worker stop requested.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = `Worker stop failed: ${String(error || '')}`.trim()
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    refreshWorkerStatus(): void {
      void refreshWorkerStatusAction()
        .then((status) => {
          this.workerStatus = String(status?.text || this.workerStatus || 'Worker: —').trim() || 'Worker: —'
          this.statusLine = 'Worker status refreshed.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = `Worker status failed: ${String(error || '')}`.trim()
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    fillFromPolygon(): void {
      void fillFromPolygonAction()
        .then((summary) => {
          const paths = Array.isArray(summary?.paths) ? summary.paths.map((path) => String(path || '').trim()).filter((path) => !!path) : []
          const text = paths.join('\n')
          this.assetsCsvText = text
          this.assetPathCount = paths.length
          this.statusLine = `Loaded ${paths.length} Polygon CSV path${paths.length === 1 ? '' : 's'}.`
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = `Fill from Polygon failed: ${String(error || '')}`.trim()
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    useCurrentCsv(): void {
      const dataStore = useDataStore()
      void useCurrentCsvAction(dataStore.csvPath)
        .then((summary) => {
          const path = String(summary?.path || '').trim()
          this.tileCsvPath = path
          this.statusLine = path ? 'Loaded current CSV path for tiles mode.' : 'Current CSV path is empty.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = `Use current CSV failed: ${String(error || '')}`.trim()
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    runAssets(): void {
      this.isRunning = true
      this.canCancel = true
      this.statusLine = 'Distributed assets: starting...'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
      void runAssetsAction({
        workerUrlsText: this.workersText,
        authToken: this.leaderToken || this.workerToken,
        assetsCsvText: this.assetsCsvText,
        rewriteFrom: this.pathFrom,
        rewriteTo: this.pathTo,
      })
        .then((summary) => {
          this.outputPreview = String(summary?.outputPreview || this.outputPreview || '—').trim() || '—'
          this.statusLine = String(summary?.statusLine || 'Distributed assets: done.').trim() || 'Distributed assets: done.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Distributed assets run failed.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .finally(() => {
          this.isRunning = false
          this.canCancel = false
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    runTiles(): void {
      this.isRunning = true
      this.canCancel = true
      this.statusLine = 'Distributed tiles: starting...'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
      void runTilesAction({
        workerUrlsText: this.workersText,
        authToken: this.leaderToken || this.workerToken,
        tileCsvPath: this.tileCsvPath,
        rewriteFrom: this.pathFrom,
        rewriteTo: this.pathTo,
        tilesPerWorker: this.tilesPerWorker,
      })
        .then((summary) => {
          this.outputPreview = String(summary?.outputPreview || this.outputPreview || '—').trim() || '—'
          this.statusLine = String(summary?.statusLine || 'Distributed tiles: done.').trim() || 'Distributed tiles: done.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Distributed tiles run failed.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .finally(() => {
          this.isRunning = false
          this.canCancel = false
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },

    cancel(): void {
      this.statusLine = 'Cancel requested...'
      this.source = 'vue'
      this.updatedAtUnixMs = Date.now()
      void cancelAction()
        .then(() => {
          this.statusLine = 'Distributed run cancel requested.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
        .catch((error: unknown) => {
          this.statusLine = String(error || '').trim() || 'Distributed cancel failed.'
          this.source = 'vue'
          this.updatedAtUnixMs = Date.now()
        })
    },
  },
})

