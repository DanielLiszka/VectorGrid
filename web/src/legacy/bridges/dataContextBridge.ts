import { useDataStore } from '../../stores/dataStore'
import { createCoalescedSync } from './coalescedSync'

const DATA_TEXT_IDS = [
  'dataId',
  'datasetLabel',
  'datasetRange',
  'dataLen',
  'marketDataDirHint',
  'dataTimeframeHint',
  'datasetContext',
  'dataFieldHint',
  'status',
] as const
const DATA_INPUT_IDS = [
  'csvPath',
  'marketDataDir',
  'dataTimeframe',
  'marketDataMoveExisting',
  'autoRestoreDataset',
] as const
const DATA_SNAPSHOT_EVENT = 'vta:data-snapshot'

function textById(id: string): string {
  const el = document.getElementById(id)
  return String(el?.textContent || '').trim()
}

function valueById(id: string): string {
  const el = document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null
  return String(el?.value || '').trim()
}

function checkedById(id: string): boolean {
  const el = document.getElementById(id) as HTMLInputElement | null
  return !!el?.checked
}

function runtimeRunning(): boolean {
  return String(document.documentElement.dataset.running || '').trim() === '1'
}

function hasLoadedDataset(dataId: string): boolean {
  const value = String(dataId || '').trim()
  return value.length > 0 && value !== '—' && value !== '-'
}

function activeControlsTab(): string {
  const rootTab = String(document.documentElement.dataset.controlsTab || '').trim()
  return rootTab || 'data'
}

function listOptionsById(id: string): string[] {
  const root = document.getElementById(id)
  if (!root) return []
  return Array.from(root.querySelectorAll('option'))
    .map((option) => String(option.value || option.textContent || '').trim())
    .filter((value) => value.length > 0)
}

function sampleCsvOptions(): { label: string; path: string; disabled: boolean }[] {
  const root = document.getElementById('sampleCsvs')
  if (!root) return []
  return Array.from(root.querySelectorAll('button'))
    .map((button) => ({
      label: String(button.textContent || '').trim(),
      path: String(button.getAttribute('data-csv-path') || button.title || '').trim(),
      disabled: !!button.disabled,
    }))
    .filter((entry) => entry.label.length > 0)
}

function sampleCsvVisible(): boolean {
  const wrap = document.getElementById('sampleCsvWrap') as HTMLElement | null
  if (!wrap) return false
  return wrap.style.display !== 'none'
}

function dataFieldList(): string[] {
  const root = document.getElementById('dataFields')
  if (!root) return []
  return Array.from(root.querySelectorAll('.badge, [data-field-badge], span, button'))
    .map((node) => String(node.textContent || '').trim())
    .filter((value) => value.length > 0)
}

function snapshot() {
  const running = runtimeRunning()
  const dataId = textById('dataId') || '—'
  const canApplyTimeframe = !running && hasLoadedDataset(dataId)

  return {
    activeControlsTab: activeControlsTab(),
    csvPath: valueById('csvPath'),
    recentCsvPaths: listOptionsById('csvRecentList'),
    sampleCsvOptions: sampleCsvOptions(),
    sampleCsvVisible: sampleCsvVisible(),
    marketDataDir: valueById('marketDataDir'),
    marketDataMoveExisting: checkedById('marketDataMoveExisting'),
    marketDataDirHint: textById('marketDataDirHint') || '—',
    dataId,
    datasetLabel: textById('datasetLabel') || '—',
    datasetRange: textById('datasetRange') || '—',
    dataLen: textById('dataLen') || '—',
    dataTimeframe: valueById('dataTimeframe'),
    dataTimeframeHint: textById('dataTimeframeHint') || '—',
    autoRestoreDataset: checkedById('autoRestoreDataset'),
    datasetContext: textById('datasetContext') || '—',
    dataFields: dataFieldList(),
    dataFieldHint: textById('dataFieldHint'),
    canBrowseCsv: !running,
    canPolygonOpen: !running,
    canLoadCsv: !running,
    canClearData: !running,
    canBrowseMarketDataDir: !running,
    canSetMarketDataDir: !running,
    canResetMarketDataDir: !running,
    canOpenMarketDataDir: !running,
    canApplyTimeframe,
    statusLine: textById('status') || 'Ready.',
  }
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

export function attachDataContextBridge(): () => void {
  const store = useDataStore()
  const syncNow = () => {
    store.applyLegacySnapshot(snapshot())
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const onInputLike = () => scheduleSync()
  const attachedInputs: (HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement)[] = []
  for (const id of DATA_INPUT_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null
    if (!el) continue
    attachedInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }

  const textObserver = observeTextNodes(DATA_TEXT_IDS, scheduleSync)

  const listObserver = new MutationObserver(() => scheduleSync())
  for (const id of ['csvRecentList', 'sampleCsvs', 'sampleCsvWrap', 'dataFields'] as const) {
    const node = document.getElementById(id)
    if (!node) continue
    listObserver.observe(node, {
      childList: true,
      characterData: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['style', 'class', 'disabled'],
    })
  }

  const rootTabObserver = new MutationObserver(() => scheduleSync())
  rootTabObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-controls-tab', 'data-running'],
  })

  const onDataSnapshot = (): void => scheduleSync()
  window.addEventListener(DATA_SNAPSHOT_EVENT, onDataSnapshot)

  syncNow()
  return () => {
    textObserver.disconnect()
    listObserver.disconnect()
    rootTabObserver.disconnect()
    window.removeEventListener(DATA_SNAPSHOT_EVENT, onDataSnapshot)
    for (const el of attachedInputs) {
      el.removeEventListener('input', onInputLike)
      el.removeEventListener('change', onInputLike)
    }
  }
}
