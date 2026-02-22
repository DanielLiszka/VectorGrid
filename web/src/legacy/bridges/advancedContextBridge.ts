import { useAdvancedStore } from '../../stores/advancedStore'
import { createCoalescedSync } from './coalescedSync'

const ADVANCED_INPUT_IDS = [
  'maSource',
  'objective',
  'mode',
  'heatmapBins',
  'topK',
  'includeAll',
  'exportAllCsvPath',
] as const
const ADVANCED_TEXT_IDS = ['vdpsTelemetryHint', 'status'] as const
const ADVANCED_SELECT_IDS = ['maSource', 'objective', 'mode', 'includeAll'] as const
const ADVANCED_SNAPSHOT_EVENT = 'vta:advanced-snapshot'

function valueById(id: string): string {
  const el = document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null
  return String(el?.value || '').trim()
}

function textById(id: string): string {
  const el = document.getElementById(id)
  return String(el?.textContent || '').trim()
}

function disabledStateById(id: string): boolean {
  const el = document.getElementById(id) as (HTMLInputElement | HTMLButtonElement | HTMLSelectElement) | null
  return !!el?.disabled
}

function runtimeFlag(flag: string, fallback = false): boolean {
  const raw = document.documentElement.dataset[flag]
  if (raw == null) return fallback
  return String(raw).trim() === '1'
}

function activeControlsTab(): string {
  const rootTab = String(document.documentElement.dataset.controlsTab || '').trim()
  return rootTab || 'data'
}

function isVisibleById(id: string): boolean {
  const el = document.getElementById(id) as HTMLElement | null
  if (!el) return false
  return window.getComputedStyle(el).display !== 'none'
}

function optionsById(id: string): { value: string; label: string; disabled: boolean }[] {
  const el = document.getElementById(id) as HTMLSelectElement | null
  if (!el) return []
  return Array.from(el.options)
    .map((option) => {
      const value = String(option.value || '').trim()
      const label = String(option.textContent || option.value || '').trim()
      return {
        value: value || label,
        label: label || value,
        disabled: !!option.disabled,
      }
    })
    .filter((entry) => entry.value.length > 0 || entry.label.length > 0)
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

export function attachAdvancedContextBridge(): () => void {
  const store = useAdvancedStore()

  const syncNow = () => {
    store.applyLegacySnapshot({
      activeControlsTab: activeControlsTab(),
      maSource: valueById('maSource') || 'close',
      maSourceOptions: optionsById('maSource'),
      objective: valueById('objective') || 'Sharpe',
      objectiveOptions: optionsById('objective'),
      mode: valueById('mode') || 'Grid',
      modeOptions: optionsById('mode'),
      vdpsVisible: isVisibleById('vdpsConfigWrap'),
      vdpsTelemetryHint: textById('vdpsTelemetryHint') || 'VDPS telemetry appears after a run.',
      compositeMoVisible: isVisibleById('compositeMoWrap'),
      heatmapBins: valueById('heatmapBins') || '128',
      topK: valueById('topK') || '20',
      includeAll: valueById('includeAll') || 'false',
      includeAllOptions: optionsById('includeAll'),
      exportAllCsvPath: valueById('exportAllCsvPath'),
      canBrowseExportAll: !disabledStateById('exportAllCsvPath'),
      canResetMoSearchSpace: runtimeFlag('advCanResetMoSearchSpace', false),
      statusLine: textById('status') || 'Ready.',
    })
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const onInputLike = () => scheduleSync()
  const attachedInputs: (HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement)[] = []
  for (const id of ADVANCED_INPUT_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null
    if (!el) continue
    attachedInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }

  const textObserver = observeTextNodes(ADVANCED_TEXT_IDS, scheduleSync)

  const rootTabObserver = new MutationObserver(() => scheduleSync())
  rootTabObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-controls-tab', 'data-running', 'data-adv-can-reset-mo-search-space'],
  })

  const selectObserver = new MutationObserver(() => scheduleSync())
  for (const id of ADVANCED_SELECT_IDS) {
    const el = document.getElementById(id)
    if (!el) continue
    selectObserver.observe(el, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['disabled', 'selected', 'value', 'class'],
    })
  }

  const visibilityObserver = new MutationObserver(() => scheduleSync())
  for (const id of ['vdpsConfigWrap', 'compositeMoWrap'] as const) {
    const node = document.getElementById(id)
    if (!node) continue
    visibilityObserver.observe(node, {
      attributes: true,
      attributeFilter: ['style', 'class'],
    })
  }

  const exportInputObserver = new MutationObserver(() => scheduleSync())
  const exportInput = document.getElementById('exportAllCsvPath')
  if (exportInput) {
    exportInputObserver.observe(exportInput, {
      attributes: true,
      attributeFilter: ['disabled', 'class'],
    })
  }

  const onAdvancedSnapshot = (): void => scheduleSync()
  window.addEventListener(ADVANCED_SNAPSHOT_EVENT, onAdvancedSnapshot)

  syncNow()

  return () => {
    textObserver.disconnect()
    rootTabObserver.disconnect()
    selectObserver.disconnect()
    visibilityObserver.disconnect()
    exportInputObserver.disconnect()
    window.removeEventListener(ADVANCED_SNAPSHOT_EVENT, onAdvancedSnapshot)
    for (const el of attachedInputs) {
      el.removeEventListener('input', onInputLike)
      el.removeEventListener('change', onInputLike)
    }
  }
}
