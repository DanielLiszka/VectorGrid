import { useDistributedStore } from '../../stores/distributedStore'
import { useExecutionStore } from '../../stores/executionStore'
import { createCoalescedSync } from './coalescedSync'

const DIST_TEXT_IDS = ['distWorkerStatus', 'distLeaderModeHint', 'distOutput', 'status'] as const
const DIST_INPUT_IDS = [
  'distLeaderMode',
  'distWorkerBindAddr',
  'distWorkerToken',
  'distLeaderToken',
  'distLeaderWorkers',
  'distAssetsCsvs',
  'distPathFrom',
  'distPathTo',
  'distTileCsvPath',
  'distTilesPerWorker',
] as const
const DISTRIBUTED_SNAPSHOT_EVENT = 'vta:distributed-snapshot'

const EXEC_TEXT_IDS = [
  'execLiveStatusHint',
  'execRunnerStatusHint',
  'execStatusHint',
  'execPositionMeta',
  'execTvChartMeta',
  'execOutput',
  'status',
] as const
const EXEC_INPUT_IDS = [
  'execMode',
  'execSymbol',
  'execQty',
  'execParamSource',
  'execRiskStopLossPct',
  'execRiskTakeProfitPct',
  'execManualFastLen',
  'execManualSlowLen',
  'execDryRun',
  'execRememberKeys',
  'execLiveUseDelayed',
] as const
const EXECUTION_SNAPSHOT_EVENT = 'vta:execution-snapshot'

function textById(id: string): string {
  const el = document.getElementById(id)
  return String(el?.textContent || '').trim()
}

function valueById(id: string): string {
  const el = document.getElementById(id) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
  return String(el?.value || '').trim()
}

function checkedById(id: string): boolean {
  const el = document.getElementById(id) as HTMLInputElement | null
  return !!el?.checked
}

function runtimeRunning(): boolean {
  return String(document.documentElement.dataset.running || '').trim() === '1'
}

function runtimeExecBusy(): boolean {
  return String(document.documentElement.dataset.execBusy || '').trim() === '1'
}

function activeControlsTab(): string {
  const rootTab = String(document.documentElement.dataset.controlsTab || '').trim()
  return rootTab || 'data'
}

function lineCount(text: string): number {
  return String(text || '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0).length
}

function orderRowsSnapshot(): { cells: string[] }[] {
  const body = document.getElementById('execOrdersTableBody')
  if (!body) return []
  const rows = Array.from(body.querySelectorAll('tr'))
  if (!rows.length) return []
  if (rows.length === 1) {
    const firstRow = rows[0] ?? null
    const firstCell = firstRow ? firstRow.querySelector('td') : null
    const text = String(firstCell?.textContent || '').trim().toLowerCase()
    if ((firstCell as HTMLTableCellElement | null)?.colSpan === 7 && text.includes('no broker orders')) {
      return []
    }
  }
  return rows.map((row) => {
    const cells = Array.from(row.querySelectorAll('td'))
      .map((cell) => String(cell.textContent || '').trim())
      .slice(0, 7)
    while (cells.length < 7) cells.push('—')
    return { cells }
  })
}

function previewText(text: string, maxLength = 220): string {
  const src = String(text || '').trim()
  if (!src) return '—'
  if (src.length <= maxLength) return src
  return `${src.slice(0, maxLength).trimEnd()}…`
}

function observeTextNodes(ids: readonly string[], onChange: () => void): MutationObserver {
  const observer = new MutationObserver(() => onChange())
  for (const id of ids) {
    const node = document.getElementById(id)
    if (!node) continue
    observer.observe(node, {
      childList: true,
      characterData: true,
      subtree: true,
    })
  }
  return observer
}

export function attachDistributedContextBridge(): () => void {
  const store = useDistributedStore()

  const syncNow = () => {
    const running = runtimeRunning()
    const canRun = !running
    store.applyLegacySnapshot({
      activeControlsTab: activeControlsTab(),
      leaderMode: valueById('distLeaderMode') || 'assets',
      leaderModeHint: textById('distLeaderModeHint') || '—',
      workerStatus: textById('distWorkerStatus') || 'Worker: —',
      workerBindAddr: valueById('distWorkerBindAddr'),
      workerToken: valueById('distWorkerToken'),
      leaderToken: valueById('distLeaderToken'),
      workersText: valueById('distLeaderWorkers'),
      assetsCsvText: valueById('distAssetsCsvs'),
      pathFrom: valueById('distPathFrom'),
      pathTo: valueById('distPathTo'),
      workersCount: lineCount(valueById('distLeaderWorkers')),
      assetPathCount: lineCount(valueById('distAssetsCsvs')),
      tileCsvPath: valueById('distTileCsvPath'),
      tilesPerWorker: valueById('distTilesPerWorker') || '4',
      outputPreview: previewText(textById('distOutput')),
      statusLine: textById('status') || 'Ready.',
      isRunning: running,
      canCancel: running,
      canStartWorker: canRun,
      canStopWorker: canRun,
      canWorkerStatus: canRun,
      canFillFromPolygon: canRun,
      canUseCurrentCsv: canRun,
      canRunAssets: canRun,
      canRunTiles: canRun,
    })
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const onInputLike = () => scheduleSync()
  const attachedInputs: (HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement)[] = []
  for (const id of DIST_INPUT_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
    if (!el) continue
    attachedInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }

  const textObserver = observeTextNodes(DIST_TEXT_IDS, scheduleSync)

  const rootTabObserver = new MutationObserver(() => scheduleSync())
  rootTabObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-controls-tab', 'data-running'],
  })

  const onDistributedSnapshot = (): void => scheduleSync()
  window.addEventListener(DISTRIBUTED_SNAPSHOT_EVENT, onDistributedSnapshot)

  syncNow()

  return () => {
    textObserver.disconnect()
    rootTabObserver.disconnect()
    window.removeEventListener(DISTRIBUTED_SNAPSHOT_EVENT, onDistributedSnapshot)
    for (const el of attachedInputs) {
      el.removeEventListener('input', onInputLike)
      el.removeEventListener('change', onInputLike)
    }
  }
}

export function attachExecutionContextBridge(): () => void {
  const store = useExecutionStore()

  const syncNow = () => {
    const disabled = runtimeRunning() || runtimeExecBusy()
    const orderRows = orderRowsSnapshot()
    store.applyLegacySnapshot({
      mode: valueById('execMode') || 'paper',
      symbol: valueById('execSymbol'),
      qty: valueById('execQty') || '1',
      paramSource: valueById('execParamSource') || 'selected',
      riskStopLossPct: valueById('execRiskStopLossPct'),
      riskTakeProfitPct: valueById('execRiskTakeProfitPct'),
      manualFastLen: valueById('execManualFastLen'),
      manualSlowLen: valueById('execManualSlowLen'),
      dryRun: checkedById('execDryRun'),
      rememberKeys: checkedById('execRememberKeys'),
      liveUseDelayed: checkedById('execLiveUseDelayed'),
      liveStatusHint: textById('execLiveStatusHint') || 'Live candles: off',
      runnerStatusHint: textById('execRunnerStatusHint') || 'Auto-runner: off',
      statusHint: textById('execStatusHint') || '-',
      positionMeta: textById('execPositionMeta') || 'Position: -',
      chartMeta: textById('execTvChartMeta') || '-',
      orderCount: orderRows.length,
      orderRows,
      outputPreview: previewText(textById('execOutput')),
      canTestConnection: !disabled,
      canRunOnce: !disabled,
      canRefreshOrders: !disabled,
      canStartLive: !disabled,
      canStopLive: !disabled,
      canStartRunner: !disabled,
      canStopRunner: !disabled,
      canClearKeys: !disabled,
      statusLine: textById('status') || 'Ready.',
    })
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const onInputLike = () => scheduleSync()
  const attachedInputs: (HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement)[] = []
  for (const id of EXEC_INPUT_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | null
    if (!el) continue
    attachedInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }

  const textObserver = observeTextNodes(EXEC_TEXT_IDS, scheduleSync)

  const tableObserver = new MutationObserver(() => scheduleSync())
  const tableBody = document.getElementById('execOrdersTableBody')
  if (tableBody) {
    tableObserver.observe(tableBody, {
      childList: true,
      subtree: true,
      characterData: true,
    })
  }

  const rootStateObserver = new MutationObserver(() => scheduleSync())
  rootStateObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-running', 'data-exec-busy'],
  })

  const onExecutionSnapshot = (): void => scheduleSync()
  window.addEventListener(EXECUTION_SNAPSHOT_EVENT, onExecutionSnapshot)

  syncNow()

  return () => {
    textObserver.disconnect()
    tableObserver.disconnect()
    rootStateObserver.disconnect()
    window.removeEventListener(EXECUTION_SNAPSHOT_EVENT, onExecutionSnapshot)
    for (const el of attachedInputs) {
      el.removeEventListener('input', onInputLike)
      el.removeEventListener('change', onInputLike)
    }
  }
}
