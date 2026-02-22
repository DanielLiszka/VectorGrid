import { useOptimizeStore } from '../../stores/optimizeStore'
import { useValidationStore } from '../../stores/validationStore'
import { createCoalescedSync } from './coalescedSync'

const OPT_TEXT_IDS = [
  'strategyHint',
  'pairsPreview',
  'gpuStatus',
  'gpuParityMeta',
  'progressMeta',
  'runPlanMeta',
  'runReadiness',
  'runReadinessHint',
  'status',
] as const
const OPT_VALUE_IDS = [
  'strategyKind',
  'presetProfile',
  'backend',
  'deviceId',
  'objective',
  'mode',
  'fastStart',
  'fastEnd',
  'fastStep',
  'slowStart',
  'slowEnd',
  'slowStep',
  'fastMaType',
  'slowMaType',
] as const
const OPTIMIZE_SNAPSHOT_EVENT = 'vta:optimize-snapshot'
const VAL_TEXT_IDS = [
  'validationHint',
  'validationMeta',
  'validationOutput',
  'validationShortlistMeta',
  'valCrossMeta',
  'status',
] as const
const VAL_INPUT_IDS = [
  'valCandidates',
  'valSeed',
  'valMinPsr',
  'valMinDsr',
  'valCscvSlices',
  'valAutoExportTopN',
  'valWfWindow',
  'valWfStep',
  'valPerturbIters',
  'valPerturbMin',
  'valPerturbMax',
  'valMcSims',
  'valCrossMax',
  'valCrossSearch',
] as const
const VAL_CHECK_IDS = [
  'valAutoRun',
  'valAutoExportXlsx',
  'valAutoRevealReport',
  'valWfEnable',
  'valPerturbEnable',
  'valMcEnable',
  'valCscvEnable',
  'valCrossEnable',
] as const
const VALIDATION_SNAPSHOT_EVENT = 'vta:validation-snapshot'

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

function runtimeFlag(flag: string): boolean {
  return String(document.documentElement.dataset[flag] || '').trim() === '1'
}

function badgesById(id: string): string[] {
  const root = document.getElementById(id)
  if (!root) return []
  return Array.from(root.querySelectorAll('.badge'))
    .map((node) => String(node.textContent || '').trim())
    .filter((text) => text.length > 0)
}

function validationColumns(): { label: string; alignLeft: boolean }[] {
  const nodes = Array.from(document.querySelectorAll('#validationTable thead th')) as HTMLTableCellElement[]
  return nodes.map((th) => ({
    label: String(th.textContent || '').trim() || '—',
    alignLeft: th.classList.contains('left'),
  }))
}

function validationRows(): { cells: { text: string; className: string; title: string }[] }[] {
  const body = document.getElementById('validationTableBody')
  if (!body) return []
  const rows = Array.from(body.querySelectorAll('tr'))
  return rows.map((row) => ({
    cells: Array.from(row.querySelectorAll('td')).map((cell) => ({
      text: String(cell.textContent || '').trim(),
      className: String(cell.className || '').trim(),
      title: String(cell.getAttribute('title') || '').trim(),
    })),
  }))
}

function crossDatasetsSnapshot(): { path: string; label: string; meta: string; checked: boolean; disabled: boolean }[] {
  const nodes = Array.from(document.querySelectorAll('#valCrossDatasets .listItem')) as HTMLElement[]
  return nodes.map((node) => {
    const checkbox = node.querySelector('input[type="checkbox"][data-csv-path]') as HTMLInputElement | null
    const title = String((node.querySelector('.mono') as HTMLElement | null)?.textContent || '').trim()
    const meta = String((node.querySelector('.muted') as HTMLElement | null)?.textContent || '').trim()
    const path = String(checkbox?.dataset?.csvPath || '').trim()
    return {
      path,
      label: title || path || '—',
      meta: meta || '—',
      checked: !!checkbox?.checked,
      disabled: !!checkbox?.disabled,
    }
  })
}

function activeControlsTab(): string {
  const rootTab = String(document.documentElement.dataset.controlsTab || '').trim()
  return rootTab || 'data'
}

function activeResultsTab(): string {
  const rootTab = String(document.documentElement.dataset.resultsTab || '').trim()
  return rootTab || 'overview'
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

export function attachOptimizeContextBridge(): () => void {
  const store = useOptimizeStore()

  const syncNow = () => {
    const running = runtimeRunning()
    const canRun = runtimeFlag('optCanRun')
    const canValidate = runtimeFlag('optCanValidate')
    store.applyLegacySnapshot({
      activeControlsTab: activeControlsTab(),
      strategyKind: valueById('strategyKind'),
      strategyHint: textById('strategyHint') || '—',
      strategyOptions: optionsById('strategyKind'),
      presetProfile: valueById('presetProfile') || 'Balanced',
      presetOptions: optionsById('presetProfile'),
      backend: valueById('backend'),
      backendOptions: optionsById('backend'),
      deviceId: valueById('deviceId') || '0',
      deviceOptions: optionsById('deviceId'),
      objective: valueById('objective'),
      mode: valueById('mode'),
      fastStart: valueById('fastStart'),
      fastEnd: valueById('fastEnd'),
      fastStep: valueById('fastStep'),
      slowStart: valueById('slowStart'),
      slowEnd: valueById('slowEnd'),
      slowStep: valueById('slowStep'),
      fastMaType: valueById('fastMaType'),
      fastMaTypeOptions: optionsById('fastMaType'),
      slowMaType: valueById('slowMaType'),
      slowMaTypeOptions: optionsById('slowMaType'),
      pairsPreview: textById('pairsPreview') || 'Pairs: —',
      gpuStatus: textById('gpuStatus') || 'GPU: —',
      gpuParityMeta: textById('gpuParityMeta') || 'Parity: —',
      progressMeta: textById('progressMeta'),
      runPlanMeta: textById('runPlanMeta') || 'Plan: —',
      runReadinessHint: textById('runReadinessHint') || 'Readiness: —',
      runReadinessBadges: badgesById('runReadiness'),
      statusLine: textById('status') || 'Ready.',
      runDisabled: running || !canRun,
      canCancel: running,
      validateDisabled: running || !canValidate,
      canApplyPreset: runtimeFlag('optCanApplyPreset'),
      canPresetKernel: runtimeFlag('optCanPresetKernel'),
      canRefreshGpuStatus: runtimeFlag('optCanRefreshGpuStatus'),
      canParityCheck: runtimeFlag('optCanParityCheck'),
    })
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const onInputLike = () => scheduleSync()
  const attachedInputs: (HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement)[] = []
  for (const id of OPT_VALUE_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null
    if (!el) continue
    attachedInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }

  const textObserver = observeTextNodes(OPT_TEXT_IDS, scheduleSync)
  const rootTabObserver = new MutationObserver(() => scheduleSync())
  rootTabObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: [
      'data-controls-tab',
      'data-running',
      'data-opt-can-run',
      'data-opt-can-validate',
      'data-opt-can-apply-preset',
      'data-opt-can-preset-kernel',
      'data-opt-can-refresh-gpu-status',
      'data-opt-can-parity-check',
    ],
  })
  const selectObserver = new MutationObserver(() => scheduleSync())
  for (const id of ['strategyKind', 'presetProfile', 'backend', 'deviceId', 'fastMaType', 'slowMaType'] as const) {
    const el = document.getElementById(id)
    if (!el) continue
    selectObserver.observe(el, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['disabled', 'selected', 'value', 'class'],
    })
  }

  const onOptimizeSnapshot = (): void => scheduleSync()
  window.addEventListener(OPTIMIZE_SNAPSHOT_EVENT, onOptimizeSnapshot)

  syncNow()

  return () => {
    textObserver.disconnect()
    rootTabObserver.disconnect()
    selectObserver.disconnect()
    window.removeEventListener(OPTIMIZE_SNAPSHOT_EVENT, onOptimizeSnapshot)
    for (const el of attachedInputs) {
      el.removeEventListener('input', onInputLike)
      el.removeEventListener('change', onInputLike)
    }
  }
}

export function attachValidationContextBridge(): () => void {
  const store = useValidationStore()

  const syncNow = () => {
    const running = runtimeRunning()
    const canMutate = !running
    const canValidate = runtimeFlag('optCanValidate')
    const currentResultsTab = activeResultsTab()
    const validationTabActive = currentResultsTab === 'validate'
    const outputText = textById('validationOutput')
    const preview = outputText.length > 200 ? `${outputText.slice(0, 200).trimEnd()}…` : outputText
    const tableBody = validationTabActive ? document.getElementById('validationTableBody') : null
    const rowCount = tableBody ? tableBody.querySelectorAll('tr').length : store.tableRowCount
    const tableWrap = validationTabActive ? document.getElementById('validationTableWrap') as HTMLElement | null : null
    const tableVisible = tableWrap ? window.getComputedStyle(tableWrap).display !== 'none' : store.tableVisible
    const crossDatasets = validationTabActive ? crossDatasetsSnapshot() : store.crossDatasets
    const crossSelectedCount = validationTabActive
      ? crossDatasets.filter((entry) => entry.checked).length
      : store.crossSelectedCount
    const crossTotalCount = validationTabActive ? crossDatasets.length : store.crossTotalCount

    store.applyLegacySnapshot({
      activeResultsTab: currentResultsTab,
      validationHint: textById('validationHint') || '—',
      validationMeta: textById('validationMeta') || '-',
      validationOutputPreview: preview || '-',
      validationShortlistMeta: textById('validationShortlistMeta') || '-',
      validationColumns: validationTabActive ? validationColumns() : store.validationColumns,
      validationRows: validationTabActive ? validationRows() : store.validationRows,
      crossMeta: textById('valCrossMeta') || '—',
      tableRowCount: rowCount,
      candidates: valueById('valCandidates') || '32',
      seed: valueById('valSeed') || '1',
      minPsr: valueById('valMinPsr'),
      minDsr: valueById('valMinDsr'),
      cscvSlices: valueById('valCscvSlices'),
      autoRun: checkedById('valAutoRun'),
      autoExportXlsx: checkedById('valAutoExportXlsx'),
      autoRevealReport: checkedById('valAutoRevealReport'),
      autoExportTopN: valueById('valAutoExportTopN') || '20',
      wfEnable: checkedById('valWfEnable'),
      wfWindow: valueById('valWfWindow') || '20000',
      wfStep: valueById('valWfStep') || '20000',
      perturbEnable: checkedById('valPerturbEnable'),
      perturbIters: valueById('valPerturbIters') || '8',
      perturbMin: valueById('valPerturbMin') || '0.015',
      perturbMax: valueById('valPerturbMax') || '0.06',
      mcEnable: checkedById('valMcEnable'),
      mcSims: valueById('valMcSims') || '500',
      cscvEnable: checkedById('valCscvEnable'),
      crossEnable: checkedById('valCrossEnable'),
      crossMax: valueById('valCrossMax') || '6',
      crossSearch: valueById('valCrossSearch'),
      crossDatasets,
      crossSelectedCount,
      crossTotalCount,
      tableVisible,
      canOpenReportsDir: canMutate,
      canExportValidationCsv: runtimeFlag('valCanExportCsv'),
      canExportValidationJson: runtimeFlag('valCanExportJson'),
      canExportReportJson: runtimeFlag('valCanExportReportJson'),
      canExportReportXlsx: runtimeFlag('valCanExportReportXlsx'),
      canCrossRefresh: runtimeFlag('valCanCrossRefresh'),
      canCrossSelectAll: runtimeFlag('valCanCrossSelectAll'),
      canCrossSelectNone: runtimeFlag('valCanCrossSelectNone'),
      canCrossSelectMatch: runtimeFlag('valCanCrossSelectMatch'),
      statusLine: textById('status') || 'Ready.',
      validateDisabled: running || !canValidate,
    })
  }
  const scheduleSync = createCoalescedSync(syncNow)

  const onInputLike = () => scheduleSync()
  const attachedInputs: (HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement)[] = []
  for (const id of VAL_INPUT_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null
    if (!el) continue
    attachedInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }
  for (const id of VAL_CHECK_IDS) {
    const el = document.getElementById(id) as HTMLInputElement | null
    if (!el) continue
    attachedInputs.push(el)
    el.addEventListener('input', onInputLike)
    el.addEventListener('change', onInputLike)
  }

  const observer = observeTextNodes(VAL_TEXT_IDS, scheduleSync)
  const tableBody = document.getElementById('validationTableBody')
  if (tableBody) {
    observer.observe(tableBody, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['class'],
    })
  }
  const tableHead = document.querySelector('#validationTable thead')
  if (tableHead) {
    observer.observe(tableHead, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['class'],
    })
  }
  const tableWrap = document.getElementById('validationTableWrap')
  if (tableWrap) {
    observer.observe(tableWrap, {
      attributes: true,
      attributeFilter: ['style', 'class'],
    })
  }
  const crossDatasets = document.getElementById('valCrossDatasets')
  if (crossDatasets) {
    observer.observe(crossDatasets, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['class', 'style', 'disabled', 'checked', 'value'],
    })
  }
  const rootResultsObserver = new MutationObserver(() => scheduleSync())
  rootResultsObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: [
      'data-results-tab',
      'data-running',
      'data-opt-can-validate',
      'data-val-can-export-csv',
      'data-val-can-export-json',
      'data-val-can-export-report-json',
      'data-val-can-export-report-xlsx',
      'data-val-can-cross-refresh',
      'data-val-can-cross-select-all',
      'data-val-can-cross-select-none',
      'data-val-can-cross-select-match',
    ],
  })
  const onValidationSnapshot = (): void => scheduleSync()
  window.addEventListener(VALIDATION_SNAPSHOT_EVENT, onValidationSnapshot)

  syncNow()

  return () => {
    observer.disconnect()
    rootResultsObserver.disconnect()
    window.removeEventListener(VALIDATION_SNAPSHOT_EVENT, onValidationSnapshot)
    for (const el of attachedInputs) {
      el.removeEventListener('input', onInputLike)
      el.removeEventListener('change', onInputLike)
    }
  }
}
