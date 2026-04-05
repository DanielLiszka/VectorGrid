<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { type Cell, type Header } from '@tanstack/vue-table'
import {
  useHeadlessMatrixTable,
  type MatrixColumnMeta,
} from '../composables/useHeadlessMatrixTable'
import { usePersistedMatrixColumnState } from '../composables/usePersistedMatrixColumnState'
import { useShellMount } from '../composables/useShellMount'
import { useUiReadiness } from '../composables/useUiReadiness'
import { useVirtualTableRows } from '../composables/useVirtualTableRows'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import ContextHelpTip from './ContextHelpTip.vue'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'
import {
  previewValidationAdmissionComparisonFromStores,
  previewValidationExecutionSummaryFromStores,
} from '../services/validationService'
import { VALIDATION_RESULTS_TABLE_STATE_KEY } from '../utils/core/storageKeys'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

interface ValidationColumn {
  label: string
  alignLeft: boolean
}

interface ValidationCell {
  text: string
  className: string
  title: string
}

interface ValidationRow {
  cells: ValidationCell[]
}

interface CrossDatasetItem {
  path: string
  label: string
  meta: string
  checked: boolean
  disabled: boolean
}

const validationStore = useValidationStore()
const workflowStore = useWorkflowStore()
const { validationStage } = useUiReadiness()
const {
  validationHint,
  validationMeta,
  validationRouteSummary,
  validationRouteDetailSummary,
  validationCategorySummary,
  validationAuxCpuSummary,
  validationBootstrapSummary,
  validationRegimeSummary,
  validationFrictionSummary,
  validationCrossSummary,
  validationSkippedSummary,
  validationWarningSummary,
  validationReoptMeta,
  validationShortlistMeta,
  validationColumns,
  validationRows,
  validationOutputPreview,
  crossMeta,
  tableRowCount,
  candidates,
  admissionPolicy,
  seed,
  minPsr,
  minDsr,
  minCscvRank,
  minCscvMetric,
  maxPbo,
  spaSlices,
  spaBootstrapSamples,
  spaBlockLength,
  reoptWfWindow,
  reoptWfStep,
  reoptWfTopN,
  cscvSlices,
  autoRun,
  autoExportXlsx,
  autoRevealReport,
  autoExportTopN,
  wfEnable,
  wfWindow,
  wfStep,
  perturbEnable,
  perturbIters,
  perturbMin,
  perturbMax,
  mcEnable,
  mcSims,
  mcBlockLen,
  mcMode,
  cscvEnable,
  spaEnable,
  reoptWfEnable,
  crossEnable,
  crossMax,
  crossMode,
  crossSearch,
  crossDatasets,
  crossSelectedCount,
  crossTotalCount,
  tableVisible,
  canOpenReportsDir,
  canExportValidationCsv,
  canExportValidationJson,
  canExportReportJson,
  canExportReportXlsx,
  canCrossRefresh,
  canCrossSelectAll,
  canCrossSelectNone,
  canCrossSelectMatch,
  statusLine,
  validateDisabled,
  updatedAtUnixMs,
} = storeToRefs(validationStore)
const { resultsTab } = storeToRefs(workflowStore)
const expectedExecutionSummary = computed(() => previewValidationExecutionSummaryFromStores())
const expectedAdmissionComparison = computed(() =>
  previewValidationAdmissionComparisonFromStores(),
)

const { mountReady } = useShellMount('#validationVueShellMount', {
  paneSelector: '[data-vue-panel="validate"]',
  paneDatasetKey: 'vueValidation',
})

const candidatesDraft = ref('32')
const admissionPolicyDraft = ref('legacy_topk')
const seedDraft = ref('1')
const minPsrDraft = ref('')
const minDsrDraft = ref('')
const minCscvRankDraft = ref('')
const minCscvMetricDraft = ref('')
const maxPboDraft = ref('')
const spaSlicesDraft = ref('16')
const spaBootstrapSamplesDraft = ref('500')
const spaBlockLengthDraft = ref('4')
const reoptWfWindowDraft = ref('20000')
const reoptWfStepDraft = ref('5000')
const reoptWfTopNDraft = ref('8')
const cscvDraft = ref('')
const autoRunDraft = ref(true)
const autoExportXlsxDraft = ref(false)
const autoRevealReportDraft = ref(false)
const autoExportTopNDraft = ref('20')
const wfEnableDraft = ref(false)
const wfWindowDraft = ref('20000')
const wfStepDraft = ref('20000')
const perturbEnableDraft = ref(false)
const perturbItersDraft = ref('8')
const perturbMinDraft = ref('0.015')
const perturbMaxDraft = ref('0.06')
const mcEnableDraft = ref(false)
const mcSimsDraft = ref('500')
const mcBlockLenDraft = ref('')
const mcModeDraft = ref('compare')
const cscvEnableDraft = ref(false)
const spaEnableDraft = ref(false)
const reoptWfEnableDraft = ref(false)
const crossEnableDraft = ref(false)
const crossMaxDraft = ref('6')
const crossModeDraft = ref('auto')
const crossSearchDraft = ref('')

const visible = computed(() => String(resultsTab.value || '').trim() === 'validate')
const showValidationResults = computed(() => validationStage.value === 'ready')
const renderedColumns = computed<ValidationColumn[]>(() => validationColumns.value as ValidationColumn[])
const renderedRows = computed<ValidationRow[]>(() => validationRows.value as ValidationRow[])
const {
  columnOptions,
  columnVisibility,
  visibleCount: visibleColumnCount,
  hiddenCount: hiddenColumnCount,
  canRestoreDefaults,
  toggleColumnVisibility,
  restoreDefaultColumns,
  canHideColumn,
} = usePersistedMatrixColumnState(renderedColumns, VALIDATION_RESULTS_TABLE_STATE_KEY)
const { headerGroups, rowModelRows, visibleLeafColumns } = useHeadlessMatrixTable(
  renderedColumns,
  renderedRows,
  { columnVisibility },
)
const renderedCrossDatasets = computed<CrossDatasetItem[]>(() => crossDatasets.value as CrossDatasetItem[])
const {
  scrollElementRef: tableScrollElementRef,
  virtualRows,
  topSpacerPx: virtualTopSpacerPx,
  bottomSpacerPx: virtualBottomSpacerPx,
  scrollToTop,
} = useVirtualTableRows(rowModelRows)

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

const validationStageLabel = computed(() => {
  if (validationStage.value === 'no_data') return 'Awaiting dataset'
  if (validationStage.value === 'data_loaded_no_run') return 'Awaiting shortlist'
  return showValidationResults.value ? 'Results ready' : 'Ready to run'
})

const validationSummaryChips = computed(() => {
  const routeSummary = String(
    (showValidationResults.value ? validationRouteSummary.value : expectedExecutionSummary.value.routeSummary) || '',
  ).trim()
  const crossSummary = Number(crossTotalCount.value || 0) > 0
    ? `Cross ${crossSelectedCount.value}/${crossTotalCount.value}`
    : String((showValidationResults.value ? validationCrossSummary.value : expectedExecutionSummary.value.crossSummary) || '').trim()
  return [
    validationStageLabel.value,
    `${tableRowCount.value} rows`,
    routeSummary ? `Route ${routeSummary}` : '',
    crossSummary || '',
  ].filter((chip) => String(chip || '').trim().length > 0)
})
const columnVisibilitySummary = computed(
  () => `Columns ${visibleColumnCount.value}/${renderedColumns.value.length}`,
)
const columnVisibilityHint = computed(() => {
  if (hiddenColumnCount.value > 0) {
    return `${hiddenColumnCount.value} hidden columns. Restore defaults to recover the full validation review table.`
  }
  return 'Hide secondary result columns when you want a tighter validation review pass.'
})

watch(
  () => [
    String(candidates.value || '32'),
    String(admissionPolicy.value || 'legacy_topk'),
    String(seed.value || '1'),
    String(minPsr.value || ''),
    String(minDsr.value || ''),
    String(minCscvRank.value || ''),
    String(minCscvMetric.value || ''),
    String(maxPbo.value || ''),
    String(spaSlices.value || '16').trim() || '16',
    String(spaBootstrapSamples.value || '500').trim() || '500',
    String(spaBlockLength.value || '4').trim() || '4',
    String(reoptWfWindow.value || '20000').trim() || '20000',
    String(reoptWfStep.value || '5000').trim() || '5000',
    String(reoptWfTopN.value || '8').trim() || '8',
    String(cscvSlices.value || ''),
    Boolean(autoRun.value),
    Boolean(autoExportXlsx.value),
    Boolean(autoRevealReport.value),
    String(autoExportTopN.value || '20').trim() || '20',
    Boolean(wfEnable.value),
    String(wfWindow.value || '20000').trim() || '20000',
    String(wfStep.value || '20000').trim() || '20000',
    Boolean(perturbEnable.value),
    String(perturbIters.value || '8').trim() || '8',
    String(perturbMin.value || '0.015').trim() || '0.015',
    String(perturbMax.value || '0.06').trim() || '0.06',
    Boolean(mcEnable.value),
    String(mcSims.value || '500').trim() || '500',
    String(mcBlockLen.value || '').trim(),
    String(mcMode.value || 'compare').trim() || 'compare',
    Boolean(cscvEnable.value),
    Boolean(spaEnable.value),
    Boolean(reoptWfEnable.value),
    Boolean(crossEnable.value),
    String(crossMax.value || '6').trim() || '6',
    String(crossMode.value || 'auto').trim() || 'auto',
    String(crossSearch.value || '').trim(),
  ] as const,
  ([
    nextCandidates,
    nextAdmissionPolicy,
    nextSeed,
    nextMinPsr,
    nextMinDsr,
    nextMinCscvRank,
    nextMinCscvMetric,
    nextMaxPbo,
    nextSpaSlices,
    nextSpaBootstrapSamples,
    nextSpaBlockLength,
    nextReoptWfWindow,
    nextReoptWfStep,
    nextReoptWfTopN,
    nextCscv,
    nextAutoRun,
    nextAutoExportXlsx,
    nextAutoRevealReport,
    nextAutoExportTopN,
    nextWfEnable,
    nextWfWindow,
    nextWfStep,
    nextPerturbEnable,
    nextPerturbIters,
    nextPerturbMin,
    nextPerturbMax,
    nextMcEnable,
    nextMcSims,
    nextMcBlockLen,
    nextMcMode,
    nextCscvEnable,
    nextSpaEnable,
    nextReoptWfEnable,
    nextCrossEnable,
    nextCrossMax,
    nextCrossMode,
    nextCrossSearch,
  ]) => {
    if (candidatesDraft.value !== nextCandidates) candidatesDraft.value = nextCandidates
    if (admissionPolicyDraft.value !== nextAdmissionPolicy) admissionPolicyDraft.value = nextAdmissionPolicy
    if (seedDraft.value !== nextSeed) seedDraft.value = nextSeed
    if (minPsrDraft.value !== nextMinPsr) minPsrDraft.value = nextMinPsr
    if (minDsrDraft.value !== nextMinDsr) minDsrDraft.value = nextMinDsr
    if (minCscvRankDraft.value !== nextMinCscvRank) minCscvRankDraft.value = nextMinCscvRank
    if (minCscvMetricDraft.value !== nextMinCscvMetric) minCscvMetricDraft.value = nextMinCscvMetric
    if (maxPboDraft.value !== nextMaxPbo) maxPboDraft.value = nextMaxPbo
    if (spaSlicesDraft.value !== nextSpaSlices) spaSlicesDraft.value = nextSpaSlices
    if (spaBootstrapSamplesDraft.value !== nextSpaBootstrapSamples) spaBootstrapSamplesDraft.value = nextSpaBootstrapSamples
    if (spaBlockLengthDraft.value !== nextSpaBlockLength) spaBlockLengthDraft.value = nextSpaBlockLength
    if (reoptWfWindowDraft.value !== nextReoptWfWindow) reoptWfWindowDraft.value = nextReoptWfWindow
    if (reoptWfStepDraft.value !== nextReoptWfStep) reoptWfStepDraft.value = nextReoptWfStep
    if (reoptWfTopNDraft.value !== nextReoptWfTopN) reoptWfTopNDraft.value = nextReoptWfTopN
    if (cscvDraft.value !== nextCscv) cscvDraft.value = nextCscv
    if (autoRunDraft.value !== nextAutoRun) autoRunDraft.value = nextAutoRun
    if (autoExportXlsxDraft.value !== nextAutoExportXlsx) autoExportXlsxDraft.value = nextAutoExportXlsx
    if (autoRevealReportDraft.value !== nextAutoRevealReport) autoRevealReportDraft.value = nextAutoRevealReport
    if (autoExportTopNDraft.value !== nextAutoExportTopN) autoExportTopNDraft.value = nextAutoExportTopN
    if (wfEnableDraft.value !== nextWfEnable) wfEnableDraft.value = nextWfEnable
    if (wfWindowDraft.value !== nextWfWindow) wfWindowDraft.value = nextWfWindow
    if (wfStepDraft.value !== nextWfStep) wfStepDraft.value = nextWfStep
    if (perturbEnableDraft.value !== nextPerturbEnable) perturbEnableDraft.value = nextPerturbEnable
    if (perturbItersDraft.value !== nextPerturbIters) perturbItersDraft.value = nextPerturbIters
    if (perturbMinDraft.value !== nextPerturbMin) perturbMinDraft.value = nextPerturbMin
    if (perturbMaxDraft.value !== nextPerturbMax) perturbMaxDraft.value = nextPerturbMax
    if (mcEnableDraft.value !== nextMcEnable) mcEnableDraft.value = nextMcEnable
    if (mcSimsDraft.value !== nextMcSims) mcSimsDraft.value = nextMcSims
    if (mcBlockLenDraft.value !== nextMcBlockLen) mcBlockLenDraft.value = nextMcBlockLen
    if (mcModeDraft.value !== nextMcMode) mcModeDraft.value = nextMcMode
    if (cscvEnableDraft.value !== nextCscvEnable) cscvEnableDraft.value = nextCscvEnable
    if (spaEnableDraft.value !== nextSpaEnable) spaEnableDraft.value = nextSpaEnable
    if (reoptWfEnableDraft.value !== nextReoptWfEnable) reoptWfEnableDraft.value = nextReoptWfEnable
    if (crossEnableDraft.value !== nextCrossEnable) crossEnableDraft.value = nextCrossEnable
    if (crossMaxDraft.value !== nextCrossMax) crossMaxDraft.value = nextCrossMax
    if (crossModeDraft.value !== nextCrossMode) crossModeDraft.value = nextCrossMode
    if (crossSearchDraft.value !== nextCrossSearch) crossSearchDraft.value = nextCrossSearch
  },
  { immediate: true },
)

function setInputValue(id: string, nextValue: string): void {
  validationStore.setInputField(id, nextValue)
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } = useQueuedInputWriter(setInputValue)

function setCheckedValue(id: string, nextValue: boolean): void {
  validationStore.setCheckboxField(id, nextValue)
}

function runValidationAction(): void {
  flushQueuedInputValuesNow()
  validationStore.runValidation()
}

function exportShortlistCsvAction(): void {
  flushQueuedInputValuesNow()
  validationStore.exportShortlistCsv()
}

function exportValidationJsonAction(): void {
  flushQueuedInputValuesNow()
  validationStore.exportValidationJson()
}

function exportReportJsonAction(): void {
  flushQueuedInputValuesNow()
  validationStore.exportReportJson()
}

function exportReportXlsxAction(): void {
  flushQueuedInputValuesNow()
  validationStore.exportReportXlsx()
}

function openReportsDirAction(): void {
  flushQueuedInputValuesNow()
  validationStore.openReportsDir()
}

function crossRefreshAction(): void {
  flushQueuedInputValuesNow()
  validationStore.crossRefresh()
}

function crossSelectAllAction(): void {
  flushQueuedInputValuesNow()
  validationStore.crossSelectAll()
}

function crossSelectNoneAction(): void {
  flushQueuedInputValuesNow()
  validationStore.crossSelectNone()
}

function crossSelectMatchAction(): void {
  flushQueuedInputValuesNow()
  validationStore.crossSelectMatch()
}

function toggleCrossDatasetByPath(pathValue: string, nextChecked: boolean): void {
  const path = String(pathValue || '').trim()
  if (!path) return
  validationStore.toggleCrossDataset(path, nextChecked)
}

function crossDatasetInputId(index: number): string {
  return `validation_cross_dataset_${index}`
}

function crossDatasetDescriptionId(index: number): string {
  return `validation_cross_dataset_meta_${index}`
}

function headerMeta(header: Header<ValidationRow, unknown>): MatrixColumnMeta {
  return header.column.columnDef.meta as MatrixColumnMeta
}

function validationCellPayload(cell: Cell<ValidationRow, unknown>): ValidationCell {
  return cell.getValue() as ValidationCell
}

function validationColumnToggleId(columnKey: string): string {
  return `validationColumnToggle_${String(columnKey || '').replace(/[^a-zA-Z0-9_-]+/g, '_')}`
}

onMounted(async () => {
  await nextTick()
  scrollToTop()
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
})
</script>

<template>
  <Teleport v-if="mountReady" to="#validationVueShellMount">
    <section class="validationPanelVueRoot validationVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Robustness</div>
          <h2 class="panelTitle">Validation</h2>
          <div class="panelSubtitle">
            Pressure-test shortlisted candidates with walk-forward, bootstrap, CSCV, and cross-dataset checks before export.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">{{ tableRowCount }} rows · {{ stamp }}</div>
      </header>

      <div class="panelSummaryChips" aria-label="Validation summary">
        <div v-for="chip in validationSummaryChips" :key="chip" class="panelSummaryChip">{{ chip }}</div>
      </div>

      <section v-if="validationStage !== 'ready'" class="emptyState">
        <div class="emptyTitle">
          {{
            validationStage === 'no_data'
              ? 'Load data before running validation.'
              : validationStage === 'data_loaded_no_run'
                ? 'Run an optimization before validation.'
                : 'Validation is ready to run.'
          }}
        </div>
        <div class="emptyBody">
          {{
            validationStage === 'no_data'
              ? 'Validation works on shortlisted optimization candidates, so the first step is loading a dataset.'
              : validationStage === 'data_loaded_no_run'
                ? 'Your dataset is ready. Generate candidates with an optimization run, then return here to execute robustness checks.'
                : 'Optimization results are available. Run validation to populate robustness tables, reports, and exports.'
          }}
        </div>
        <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
      </section>

      <section class="pane panelProminentPane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Run Setup</div>
            <h3 class="panelPaneTitle">Validation run</h3>
            <div class="panelSectionLead">
              Choose shortlist size, guardrails, and export behavior for the next robustness pass.
            </div>
          </div>
          <ContextHelpTip
            title="Validation actions"
            summary="Run validation and export its evidence package from this panel; the validation dock now stays focused on status and navigation."
            detail="This is the canonical validation action surface for shortlist export, report export, and reports-folder access. The dock still preserves a clear shortcut into validation without duplicating all of those buttons."
            aria-label="Validation actions help"
          />
        </div>
        <div class="panelAdvisory">{{ validationHint }}</div>
        <div class="gridFive">
          <div>
            <label for="vueValCandidates">Candidates</label>
            <input
              id="vueValCandidates"
              type="number"
              min="1"
              step="1"
              :value="candidatesDraft"
              @input="candidatesDraft = String(($event.target as HTMLInputElement).value || '32'); queueInputValue('valCandidates', candidatesDraft)"
              @change="commitInputValue('valCandidates', candidatesDraft)"
            />
          </div>
          <div>
            <label for="vueValAdmissionPolicy">Admission policy</label>
            <select
              id="vueValAdmissionPolicy"
              :value="admissionPolicyDraft"
              @change="admissionPolicyDraft = String(($event.target as HTMLSelectElement).value || 'legacy_topk'); setInputValue('valAdmissionPolicy', admissionPolicyDraft)"
            >
              <option value="legacy_topk">Legacy Top K</option>
              <option value="scalar_union">Scalar union</option>
              <option value="pareto_frontier">Pareto frontier</option>
              <option value="auto">Auto</option>
            </select>
          </div>
          <div>
            <label for="vueValSeed">Seed</label>
            <input
              id="vueValSeed"
              type="number"
              min="0"
              step="1"
              :value="seedDraft"
              @input="seedDraft = String(($event.target as HTMLInputElement).value || '1'); queueInputValue('valSeed', seedDraft)"
              @change="commitInputValue('valSeed', seedDraft)"
            />
          </div>
          <div>
            <label for="vueValMinPsr">Min PSR</label>
            <input
              id="vueValMinPsr"
              type="number"
              min="0"
              max="1"
              step="0.01"
              :value="minPsrDraft"
              @input="minPsrDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('valMinPsr', minPsrDraft)"
              @change="commitInputValue('valMinPsr', minPsrDraft)"
            />
          </div>
          <div>
            <label for="vueValMinDsr">Min DSR</label>
            <input
              id="vueValMinDsr"
              type="number"
              min="0"
              max="1"
              step="0.01"
              :value="minDsrDraft"
              @input="minDsrDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('valMinDsr', minDsrDraft)"
              @change="commitInputValue('valMinDsr', minDsrDraft)"
            />
          </div>
          <div>
            <label for="vueValCscvSlices">CSCV time slices</label>
            <input
              id="vueValCscvSlices"
              type="number"
              min="4"
              step="2"
              :value="cscvDraft"
              @input="cscvDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('valCscvSlices', cscvDraft)"
              @change="commitInputValue('valCscvSlices', cscvDraft)"
            />
          </div>
        </div>

        <div class="gridThree">
          <div>
            <label for="vueValMinCscvRank">Min CSCV rank fraction</label>
            <input
              id="vueValMinCscvRank"
              type="number"
              min="0"
              max="1"
              step="0.01"
              :value="minCscvRankDraft"
              @input="minCscvRankDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('valMinCscvRank', minCscvRankDraft)"
              @change="commitInputValue('valMinCscvRank', minCscvRankDraft)"
            />
          </div>
          <div>
            <label for="vueValMinCscvMetric">Min CSCV out-of-sample metric</label>
            <input
              id="vueValMinCscvMetric"
              type="number"
              step="any"
              :value="minCscvMetricDraft"
              @input="minCscvMetricDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('valMinCscvMetric', minCscvMetricDraft)"
              @change="commitInputValue('valMinCscvMetric', minCscvMetricDraft)"
            />
          </div>
          <div>
            <label for="vueValMaxPbo">Max PBO</label>
            <input
              id="vueValMaxPbo"
              type="number"
              min="0"
              max="1"
              step="0.01"
              :value="maxPboDraft"
              @input="maxPboDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('valMaxPbo', maxPboDraft)"
              @change="commitInputValue('valMaxPbo', maxPboDraft)"
            />
          </div>
        </div>

        <div class="actions">
          <button type="button" class="primary" :disabled="validateDisabled" @click="runValidationAction">Run validation</button>
          <button type="button" :disabled="!canExportValidationCsv" @click="exportShortlistCsvAction">Export shortlist CSV</button>
          <button type="button" :disabled="!canExportValidationJson" @click="exportValidationJsonAction">Export validation JSON</button>
          <button type="button" :disabled="!canExportReportJson" @click="exportReportJsonAction">Export report JSON</button>
          <button type="button" :disabled="!canExportReportXlsx" @click="exportReportXlsxAction">Export report XLSX</button>
          <button type="button" :disabled="!canOpenReportsDir" @click="openReportsDirAction">Open reports folder</button>
        </div>

        <div class="gridFour compactMetaGrid">
          <div>
            <label>Expected Route</label>
            <div class="metaLine mono">{{ expectedExecutionSummary.routeSummary }}</div>
          </div>
          <div>
            <label>Expected Cross</label>
            <div class="metaLine mono">{{ expectedExecutionSummary.crossSummary }}</div>
          </div>
          <div>
            <label>Expected Modules</label>
            <div class="metaLine mono">{{ expectedExecutionSummary.moduleSummary }}</div>
          </div>
          <div>
            <label>Expected Categories</label>
            <div class="metaLine mono">{{ expectedExecutionSummary.categorySummary }}</div>
          </div>
          <div>
            <label>Expected Skips</label>
            <div class="metaLine mono">{{ expectedExecutionSummary.skippedSummary }}</div>
          </div>
          <div>
            <label>Expected Notes</label>
            <div class="metaLine mono">{{ expectedExecutionSummary.warningSummary }}</div>
          </div>
          <div>
            <label>Admission Compare</label>
            <div class="metaLine mono">{{ expectedAdmissionComparison.summary }}</div>
          </div>
          <div>
            <label>Admission Delta</label>
            <div class="metaLine mono">{{ expectedAdmissionComparison.detail }}</div>
          </div>
        </div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Policies</div>
            <h3 class="panelPaneTitle">Modules and automation</h3>
            <div class="panelSectionLead">
              Configure robustness modules, automation, and cross-dataset policy before the next validation pass. Results, exports, and debug output remain below once a run completes.
            </div>
          </div>
        </div>
        <div class="panelMuted">
          {{ showValidationResults
            ? 'These settings still shape reruns and follow-up validation passes for the current shortlist.'
            : 'Set automation and module policy now so the first validation run already reflects the intended robustness workflow.' }}
        </div>
        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="autoRunDraft"
              @change="autoRunDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valAutoRun', autoRunDraft)"
            />
            Auto-run after optimization (composite)
          </label>
          <label>
            <input
              type="checkbox"
              :checked="autoExportXlsxDraft"
              @change="autoExportXlsxDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valAutoExportXlsx', autoExportXlsxDraft)"
            />
            Auto-export report XLSX after validation
          </label>
          <label>
            <input
              type="checkbox"
              :checked="autoRevealReportDraft"
              @change="autoRevealReportDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valAutoRevealReport', autoRevealReportDraft)"
            />
            Reveal report in folder after save
          </label>
        </div>

        <div class="gridThree">
          <div>
            <label for="vueValAutoExportTopN">Max rows in XLSX (0 = default)</label>
            <input
              id="vueValAutoExportTopN"
              type="number"
              min="0"
              step="1"
              :value="autoExportTopNDraft"
              @input="autoExportTopNDraft = String(($event.target as HTMLInputElement).value || '20'); queueInputValue('valAutoExportTopN', autoExportTopNDraft)"
              @change="commitInputValue('valAutoExportTopN', autoExportTopNDraft)"
            />
          </div>
          <div>
            <label>&nbsp;</label>
            <div class="panelMuted">Saved to app data under <span class="mono">reports/</span>.</div>
          </div>
        </div>

        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="wfEnableDraft"
              @change="wfEnableDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valWfEnable', wfEnableDraft)"
            />
            Walk-forward (recommended)
          </label>
          <label>
            <input
              type="checkbox"
              :checked="perturbEnableDraft"
              @change="perturbEnableDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valPerturbEnable', perturbEnableDraft)"
            />
            Parameter perturbations
          </label>
          <label>
            <input
              type="checkbox"
              :checked="mcEnableDraft"
              @change="mcEnableDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valMcEnable', mcEnableDraft)"
            />
            Trade bootstrap (Monte Carlo)
          </label>
          <label>
            <input
              type="checkbox"
              :checked="cscvEnableDraft"
              @change="cscvEnableDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valCscvEnable', cscvEnableDraft)"
            />
            CSCV/PBO (recommended)
          </label>
          <label>
            <input
              type="checkbox"
              :checked="spaEnableDraft"
              @change="spaEnableDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valSpaEnable', spaEnableDraft)"
            />
            Hansen SPA
          </label>
          <label>
            <input
              type="checkbox"
              :checked="reoptWfEnableDraft"
              @change="reoptWfEnableDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valReoptWfEnable', reoptWfEnableDraft)"
            />
            Finalist re-opt walk-forward
          </label>
        </div>

        <div class="gridThree">
          <div>
            <label for="vueValWfWindow">WF window (bars)</label>
            <input
              id="vueValWfWindow"
              type="number"
              min="64"
              step="1"
              :value="wfWindowDraft"
              @input="wfWindowDraft = String(($event.target as HTMLInputElement).value || '20000'); queueInputValue('valWfWindow', wfWindowDraft)"
              @change="commitInputValue('valWfWindow', wfWindowDraft)"
            />
          </div>
          <div>
            <label for="vueValWfStep">WF step (bars)</label>
            <input
              id="vueValWfStep"
              type="number"
              min="1"
              step="1"
              :value="wfStepDraft"
              @input="wfStepDraft = String(($event.target as HTMLInputElement).value || '20000'); queueInputValue('valWfStep', wfStepDraft)"
              @change="commitInputValue('valWfStep', wfStepDraft)"
            />
          </div>
          <div>
            <label for="vueValPerturbIters">Perturb iterations</label>
            <input
              id="vueValPerturbIters"
              type="number"
              min="0"
              step="1"
              :value="perturbItersDraft"
              @input="perturbItersDraft = String(($event.target as HTMLInputElement).value || '8'); queueInputValue('valPerturbIters', perturbItersDraft)"
              @change="commitInputValue('valPerturbIters', perturbItersDraft)"
            />
          </div>
        </div>

        <div class="gridThree">
          <div>
            <label for="vueValSpaSlices">SPA slices</label>
            <input
              id="vueValSpaSlices"
              type="number"
              min="4"
              step="1"
              :value="spaSlicesDraft"
              @input="spaSlicesDraft = String(($event.target as HTMLInputElement).value || '16'); queueInputValue('valSpaSlices', spaSlicesDraft)"
              @change="commitInputValue('valSpaSlices', spaSlicesDraft)"
            />
          </div>
          <div>
            <label for="vueValSpaBootstrapSamples">SPA bootstrap samples</label>
            <input
              id="vueValSpaBootstrapSamples"
              type="number"
              min="100"
              step="1"
              :value="spaBootstrapSamplesDraft"
              @input="spaBootstrapSamplesDraft = String(($event.target as HTMLInputElement).value || '500'); queueInputValue('valSpaBootstrapSamples', spaBootstrapSamplesDraft)"
              @change="commitInputValue('valSpaBootstrapSamples', spaBootstrapSamplesDraft)"
            />
          </div>
          <div>
            <label for="vueValSpaBlockLength">SPA mean block length</label>
            <input
              id="vueValSpaBlockLength"
              type="number"
              min="1"
              step="1"
              :value="spaBlockLengthDraft"
              @input="spaBlockLengthDraft = String(($event.target as HTMLInputElement).value || '4'); queueInputValue('valSpaBlockLength', spaBlockLengthDraft)"
              @change="commitInputValue('valSpaBlockLength', spaBlockLengthDraft)"
            />
          </div>
        </div>

        <div class="gridThree">
          <div>
            <label for="vueValReoptWfWindow">Re-opt walk-forward window (bars)</label>
            <input
              id="vueValReoptWfWindow"
              type="number"
              min="64"
              step="1"
              :value="reoptWfWindowDraft"
              @input="reoptWfWindowDraft = String(($event.target as HTMLInputElement).value || '20000'); queueInputValue('valReoptWfWindow', reoptWfWindowDraft)"
              @change="commitInputValue('valReoptWfWindow', reoptWfWindowDraft)"
            />
          </div>
          <div>
            <label for="vueValReoptWfStep">Re-opt walk-forward step (bars)</label>
            <input
              id="vueValReoptWfStep"
              type="number"
              min="1"
              step="1"
              :value="reoptWfStepDraft"
              @input="reoptWfStepDraft = String(($event.target as HTMLInputElement).value || '5000'); queueInputValue('valReoptWfStep', reoptWfStepDraft)"
              @change="commitInputValue('valReoptWfStep', reoptWfStepDraft)"
            />
          </div>
          <div>
            <label for="vueValReoptWfTopN">Re-opt finalist shortlist</label>
            <input
              id="vueValReoptWfTopN"
              type="number"
              min="1"
              step="1"
              :value="reoptWfTopNDraft"
              @input="reoptWfTopNDraft = String(($event.target as HTMLInputElement).value || '8'); queueInputValue('valReoptWfTopN', reoptWfTopNDraft)"
              @change="commitInputValue('valReoptWfTopN', reoptWfTopNDraft)"
            />
          </div>
        </div>

        <div class="gridThree">
          <div>
            <label for="vueValPerturbMin">Perturb min %</label>
            <input
              id="vueValPerturbMin"
              type="number"
              step="any"
              :value="perturbMinDraft"
              @input="perturbMinDraft = String(($event.target as HTMLInputElement).value || '0.015'); queueInputValue('valPerturbMin', perturbMinDraft)"
              @change="commitInputValue('valPerturbMin', perturbMinDraft)"
            />
          </div>
          <div>
            <label for="vueValPerturbMax">Perturb max %</label>
            <input
              id="vueValPerturbMax"
              type="number"
              step="any"
              :value="perturbMaxDraft"
              @input="perturbMaxDraft = String(($event.target as HTMLInputElement).value || '0.06'); queueInputValue('valPerturbMax', perturbMaxDraft)"
              @change="commitInputValue('valPerturbMax', perturbMaxDraft)"
            />
          </div>
          <div>
            <label for="vueValMcSims">Monte Carlo simulations</label>
            <input
              id="vueValMcSims"
              type="number"
              min="0"
              step="1"
              :value="mcSimsDraft"
              @input="mcSimsDraft = String(($event.target as HTMLInputElement).value || '500'); queueInputValue('valMcSims', mcSimsDraft)"
              @change="commitInputValue('valMcSims', mcSimsDraft)"
            />
          </div>
          <div>
            <label for="vueValMcMode">Bootstrap mode</label>
            <select
              id="vueValMcMode"
              :value="mcModeDraft"
              @change="mcModeDraft = String(($event.target as HTMLSelectElement).value || 'compare'); commitInputValue('valMcMode', mcModeDraft)"
            >
              <option value="compare">Compare (IID + Stationary)</option>
              <option value="iid_only">IID only</option>
            </select>
          </div>
          <div>
            <label for="vueValMcBlockLen">Bootstrap mean block length</label>
            <input
              id="vueValMcBlockLen"
              type="number"
              min="0"
              step="1"
              :disabled="mcModeDraft === 'iid_only'"
              :value="mcBlockLenDraft"
              @input="mcBlockLenDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('valMcBlockLen', mcBlockLenDraft)"
              @change="commitInputValue('valMcBlockLen', mcBlockLenDraft)"
            />
          </div>
        </div>

        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="crossEnableDraft"
              @change="crossEnableDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valCrossEnable', crossEnableDraft)"
            />
            Cross-ticker validation
          </label>
        </div>

        <div class="gridThree">
          <div>
            <label for="vueValCrossMax">Max tickers</label>
            <input
              id="vueValCrossMax"
              type="number"
              min="0"
              step="1"
              :value="crossMaxDraft"
              @input="crossMaxDraft = String(($event.target as HTMLInputElement).value || '6'); queueInputValue('valCrossMax', crossMaxDraft)"
              @change="commitInputValue('valCrossMax', crossMaxDraft)"
            />
          </div>
          <div>
            <label for="vueValCrossMode">Cross mode</label>
            <select
              id="vueValCrossMode"
              :value="crossModeDraft"
              @change="crossModeDraft = String(($event.target as HTMLSelectElement).value || 'auto'); setInputValue('valCrossMode', crossModeDraft)"
            >
              <option value="auto">Auto by strategy</option>
              <option value="preserved_finalists">Preserved finalists</option>
              <option value="lightweight">Lightweight</option>
            </select>
          </div>
          <div>
            <label for="vueValCrossSearch">Search</label>
            <input
              id="vueValCrossSearch"
              type="text"
              :value="crossSearchDraft"
              @input="crossSearchDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('valCrossSearch', crossSearchDraft)"
              @change="commitInputValue('valCrossSearch', crossSearchDraft)"
            />
          </div>
          <div>
            <label>&nbsp;</label>
            <div class="actions" style="margin-top: 0;">
              <button type="button" :disabled="!canCrossRefresh" @click="crossRefreshAction">Refresh list</button>
              <button type="button" :disabled="!canCrossSelectAll" @click="crossSelectAllAction">All</button>
              <button type="button" :disabled="!canCrossSelectNone" @click="crossSelectNoneAction">None</button>
              <button type="button" :disabled="!canCrossSelectMatch" @click="crossSelectMatchAction">Match current</button>
            </div>
          </div>
        </div>

        <div class="metaLine mono">{{ crossMeta }}</div>
        <div class="metaLine mono" role="status" aria-live="polite">
          Selected {{ crossSelectedCount }}/{{ crossTotalCount }}
        </div>

        <div class="crossList" role="group" aria-label="Cross-ticker validation datasets">
          <label
            v-for="(item, index) in renderedCrossDatasets"
            :key="`cross_item_${item.path || item.label}_${index}`"
            class="crossItem"
            :class="{ disabled: item.disabled }"
            :for="crossDatasetInputId(index)"
          >
            <div class="left">
              <div class="mono">{{ item.label }}</div>
              <div :id="crossDatasetDescriptionId(index)" class="panelMuted">{{ item.meta }}</div>
            </div>
            <input
              :id="crossDatasetInputId(index)"
              type="checkbox"
              :checked="item.checked"
              :disabled="item.disabled"
              :aria-describedby="crossDatasetDescriptionId(index)"
              @change="toggleCrossDatasetByPath(item.path, ($event.target as HTMLInputElement).checked)"
            />
          </label>
        </div>
      </section>

      <section v-if="showValidationResults" class="pane validationResultsPane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Results</div>
            <h3 class="panelPaneTitle">Validation summary table</h3>
            <div class="panelSectionLead">
              Review route, category, cross-check, and warning outcomes for the current shortlist before export.
            </div>
          </div>
        </div>
        <div class="validationSummary surfaceSunken">
          <div class="gridFour compactMetaGrid">
            <div>
              <label>Route</label>
              <div class="metaLine mono">{{ validationRouteSummary }}</div>
            </div>
            <div>
              <label>Route Detail</label>
              <div class="metaLine mono">{{ validationRouteDetailSummary }}</div>
            </div>
            <div>
              <label>Categories</label>
              <div class="metaLine mono">{{ validationCategorySummary }}</div>
            </div>
            <div>
              <label>Aux CPU</label>
              <div class="metaLine mono">{{ validationAuxCpuSummary }}</div>
            </div>
            <div>
              <label>Bootstrap</label>
              <div class="metaLine mono">{{ validationBootstrapSummary }}</div>
            </div>
            <div>
              <label>Regime</label>
              <div class="metaLine mono">{{ validationRegimeSummary }}</div>
            </div>
            <div>
              <label>Friction</label>
              <div class="metaLine mono">{{ validationFrictionSummary }}</div>
            </div>
            <div>
              <label>Cross</label>
              <div class="metaLine mono">{{ validationCrossSummary }}</div>
            </div>
            <div>
              <label>Skipped</label>
              <div class="metaLine mono">{{ validationSkippedSummary }}</div>
            </div>
            <div>
              <label>Top Warning</label>
              <div class="metaLine mono">{{ validationWarningSummary }}</div>
            </div>
          </div>
          <div class="validationSummaryMeta">
            <div class="metaLine mono" role="status" aria-live="polite">{{ validationMeta }}</div>
            <div v-if="validationReoptMeta !== '-'" class="metaLine mono">{{ validationReoptMeta }}</div>
            <div class="metaLine mono">{{ validationShortlistMeta }}</div>
            <div class="metaLine mono">Table visible: {{ tableVisible ? 'yes' : 'no' }}</div>
          </div>
        </div>
        <details v-if="renderedColumns.length > 0" class="columnDetails">
          <summary class="columnDetailsSummary">
            <span>Visible columns</span>
            <span class="mono panelNumeric">{{ columnVisibilitySummary }}</span>
          </summary>
          <div class="columnDetailsBody">
            <div class="panelMuted">{{ columnVisibilityHint }}</div>
            <div class="columnToggleGrid" role="group" aria-label="Visible validation result columns">
              <label
                v-for="column in columnOptions"
                :key="column.key"
                class="columnToggle"
                :class="{ disabled: !canHideColumn(column.key) && column.visible }"
                :for="validationColumnToggleId(column.key)"
              >
                <input
                  :id="validationColumnToggleId(column.key)"
                  type="checkbox"
                  :checked="column.visible"
                  :disabled="!canHideColumn(column.key) && column.visible"
                  @change="toggleColumnVisibility(column.key)"
                />
                <span>{{ column.label }}</span>
              </label>
            </div>
            <div class="actions columnDetailsActions">
              <button type="button" :disabled="!canRestoreDefaults" @click="restoreDefaultColumns()">
                Restore defaults
              </button>
            </div>
          </div>
        </details>
        <div
          ref="tableScrollElementRef"
          class="tableWrap overflowShadowY overflowShadowX surfaceWorking"
          tabindex="0"
          aria-label="Validation results table"
        >
          <table class="validationTableVue">
            <caption class="srOnly">
              Validation metrics for the current shortlist, including route, category, and cross-ticker results.
            </caption>
            <thead>
              <tr v-for="headerGroup in headerGroups" :key="headerGroup.id">
                <th
                  v-for="header in headerGroup.headers"
                  :key="header.id"
                  :class="{ left: headerMeta(header).alignLeft }"
                  scope="col"
                >
                  {{ headerMeta(header).columnLabel }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="virtualTopSpacerPx > 0" class="spacerRow" aria-hidden="true">
                <td :colspan="Math.max(1, visibleLeafColumns.length)" :style="{ height: `${virtualTopSpacerPx}px` }"></td>
              </tr>
              <tr v-for="virtualRow in virtualRows" :key="virtualRow.row.id">
                <template v-for="cell in virtualRow.row.getVisibleCells()" :key="cell.id">
                  <td
                    :class="validationCellPayload(cell).className"
                    :title="validationCellPayload(cell).title || undefined"
                  >
                    {{ validationCellPayload(cell).text }}
                  </td>
                </template>
              </tr>
              <tr v-if="virtualBottomSpacerPx > 0" class="spacerRow" aria-hidden="true">
                <td :colspan="Math.max(1, visibleLeafColumns.length)" :style="{ height: `${virtualBottomSpacerPx}px` }"></td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section v-if="showValidationResults" class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Debug</div>
            <h3 class="panelPaneTitle">Validation output</h3>
            <div class="panelSectionLead">
              Keep the structured payload available for troubleshooting without letting it crowd the operational view.
            </div>
          </div>
        </div>
        <details class="panelDetails">
          <summary>Validation JSON preview</summary>
          <div class="panelDetailsBody">
            <pre class="mono preview">{{ validationOutputPreview }}</pre>
          </div>
        </details>
      </section>

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.validationPanelVueRoot {
  display: grid;
  gap: 0.85rem;
}

.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
}

.pane {
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-working) 95%, var(--panel-footer));
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.validationResultsPane {
  display: grid;
  gap: 0.55rem;
}

.gridFive {
  margin-top: 0.55rem;
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.6rem;
}

.gridThree {
  margin-top: 0.55rem;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.6rem;
}

.gridFour {
  margin-top: 0.55rem;
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.6rem;
}

.checks {
  margin-top: 0.6rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.actions {
  margin-top: 0.65rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.actions .primary {
  border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
}

.metaLine {
  font-size: 0.78rem;
  color: var(--muted-strong);
}

.validationSummary {
  margin-top: 0;
  margin-bottom: 0;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-sunken) 94%, var(--panel));
  padding: calc(var(--density-pane-y) - 0.08rem) calc(var(--density-pane-x) - 0.04rem);
  display: grid;
  gap: 0.55rem;
}

.validationSummaryMeta {
  display: grid;
  gap: 0.22rem;
  padding-top: 0.35rem;
  border-top: 1px solid color-mix(in oklab, var(--border) 78%, transparent);
}

.compactMetaGrid {
  margin-bottom: 0;
}

.compactMetaGrid label {
  display: block;
  margin-bottom: 0.15rem;
  font-size: 0.72rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--muted-soft);
}

.tableWrap {
  margin-top: 0;
  overflow: auto;
  border: 1px solid color-mix(in oklab, var(--border) 90%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-working) 96%, var(--panel));
}

.columnDetails {
  margin-top: 0;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-sunken) 94%, var(--panel));
}

.columnDetails[open] {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 38%, var(--border));
  background: color-mix(in oklab, var(--surface-working) 94%, var(--panel-footer));
}

.columnDetailsSummary {
  list-style: none;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.75rem;
  cursor: pointer;
  padding: calc(var(--density-pane-y) - 0.12rem) calc(var(--density-pane-x) - 0.02rem);
  font-size: 0.78rem;
  font-weight: 600;
  color: var(--muted-strong);
}

.columnDetailsSummary::-webkit-details-marker {
  display: none;
}

.columnDetailsBody {
  border-top: 1px solid color-mix(in oklab, var(--border) 78%, transparent);
  padding: calc(var(--density-pane-y) - 0.05rem) calc(var(--density-pane-x) - 0.02rem);
  display: grid;
  gap: 0.65rem;
}

.columnToggleGrid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.45rem 0.75rem;
}

.columnToggle {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  min-width: 0;
  font-size: 0.76rem;
}

.columnToggle.disabled {
  opacity: 0.62;
}

.columnToggle input {
  margin: 0;
}

.columnDetailsActions {
  margin-top: 0;
}

.validationTableVue {
  width: 100%;
  border-collapse: collapse;
  min-width: 980px;
}

.srOnly {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.validationTableVue th,
.validationTableVue td {
  border-bottom: 1px solid var(--border);
  padding: 0.36rem 0.44rem;
  text-align: right;
  vertical-align: top;
}

.validationTableVue th {
  font-size: 0.72rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--muted-soft);
}

.validationTableVue th.left,
.validationTableVue td.left {
  text-align: left;
}

.validationTableVue tbody tr.spacerRow {
  pointer-events: none;
}

.validationTableVue tbody tr.spacerRow td {
  border-bottom: none;
  padding: 0;
}

.preview {
  margin-top: 0.45rem;
  max-height: 220px;
  overflow: auto;
}

.crossList {
  margin-top: 0.55rem;
  max-height: 260px;
  overflow: auto;
  border: 1px solid color-mix(in oklab, var(--border) 88%, transparent);
  border-radius: 10px;
  padding: 0.25rem;
  background: color-mix(in oklab, var(--surface-sunken) 94%, var(--panel));
}

.crossItem {
  border: 1px solid color-mix(in oklab, var(--border) 68%, transparent);
  border-radius: 8px;
  padding: 0.42rem 0.5rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.7rem;
  cursor: pointer;
  background: color-mix(in oklab, var(--surface-working) 94%, var(--panel));
}

.crossItem:hover {
  border-color: color-mix(in oklab, var(--panel-focus-edge) 34%, var(--border));
  background: color-mix(in oklab, var(--surface-focus) 86%, var(--panel));
}

.crossItem.disabled {
  cursor: not-allowed;
  opacity: 0.7;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1100px) {
  .gridFive,
  .gridFour,
  .gridThree {
    grid-template-columns: 1fr 1fr;
  }

  .columnToggleGrid {
    grid-template-columns: 1fr 1fr;
  }
}

@media (max-width: 680px) {
  .gridFive,
  .gridFour,
  .gridThree {
    grid-template-columns: 1fr;
  }

  .columnToggleGrid {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='validate'][data-vue-validation='1'] > :not(#validationVueShellMount) {
  display: none !important;
}
</style>

