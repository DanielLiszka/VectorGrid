<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useUiReadiness } from '../composables/useUiReadiness'
import { useVirtualTableRows } from '../composables/useVirtualTableRows'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useValidationStore } from '../stores/validationStore'
import { useWorkflowStore } from '../stores/workflowStore'

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
  validationShortlistMeta,
  validationColumns,
  validationRows,
  validationOutputPreview,
  crossMeta,
  tableRowCount,
  candidates,
  seed,
  minPsr,
  minDsr,
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
  cscvEnable,
  crossEnable,
  crossMax,
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

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

const candidatesDraft = ref('32')
const seedDraft = ref('1')
const minPsrDraft = ref('')
const minDsrDraft = ref('')
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
const cscvEnableDraft = ref(false)
const crossEnableDraft = ref(false)
const crossMaxDraft = ref('6')
const crossSearchDraft = ref('')

const visible = computed(() => String(resultsTab.value || '').trim() === 'validate')
const showValidationResults = computed(() => validationStage.value === 'ready')
const renderedColumns = computed<ValidationColumn[]>(() => validationColumns.value as ValidationColumn[])
const renderedRows = computed<ValidationRow[]>(() => validationRows.value as ValidationRow[])
const renderedCrossDatasets = computed<CrossDatasetItem[]>(() => crossDatasets.value as CrossDatasetItem[])
const {
  scrollElementRef: tableScrollElementRef,
  virtualRows,
  topSpacerPx: virtualTopSpacerPx,
  bottomSpacerPx: virtualBottomSpacerPx,
  scrollToTop,
} = useVirtualTableRows(renderedRows)

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

watch(
  () => [
    String(candidates.value || '32'),
    String(seed.value || '1'),
    String(minPsr.value || ''),
    String(minDsr.value || ''),
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
    Boolean(cscvEnable.value),
    Boolean(crossEnable.value),
    String(crossMax.value || '6').trim() || '6',
    String(crossSearch.value || '').trim(),
  ] as const,
  ([
    nextCandidates,
    nextSeed,
    nextMinPsr,
    nextMinDsr,
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
    nextCscvEnable,
    nextCrossEnable,
    nextCrossMax,
    nextCrossSearch,
  ]) => {
    if (candidatesDraft.value !== nextCandidates) candidatesDraft.value = nextCandidates
    if (seedDraft.value !== nextSeed) seedDraft.value = nextSeed
    if (minPsrDraft.value !== nextMinPsr) minPsrDraft.value = nextMinPsr
    if (minDsrDraft.value !== nextMinDsr) minDsrDraft.value = nextMinDsr
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
    if (cscvEnableDraft.value !== nextCscvEnable) cscvEnableDraft.value = nextCscvEnable
    if (crossEnableDraft.value !== nextCrossEnable) crossEnableDraft.value = nextCrossEnable
    if (crossMaxDraft.value !== nextCrossMax) crossMaxDraft.value = nextCrossMax
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

function toggleCrossDataset(index: number): void {
  const entry = renderedCrossDatasets.value[index]
  if (!entry || entry.disabled) return
  const path = String(entry.path || '').trim()
  if (!path) return
  validationStore.toggleCrossDataset(path, !entry.checked)
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="validate"]') as HTMLElement | null
  const mount = document.querySelector('#validationVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueValidation = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  scrollToTop()
})

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueValidation
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#validationVueShellMount">
    <section class="validationPanelVueRoot validationVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Validation</div>
        <div class="meta mono">{{ tableRowCount }} rows · {{ stamp }}</div>
      </header>

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
        <div class="muted mono">{{ statusLine }}</div>
      </section>

      <section class="pane">
        <div class="muted">{{ validationHint }}</div>
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
            <label for="vueValCscvSlices">CSCV slices</label>
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

        <div class="actions">
          <button type="button" class="primary" :disabled="validateDisabled" @click="runValidationAction">Run validation</button>
          <button type="button" :disabled="!canExportValidationCsv" @click="exportShortlistCsvAction">Export shortlist CSV</button>
          <button type="button" :disabled="!canExportValidationJson" @click="exportValidationJsonAction">Export validation JSON</button>
          <button type="button" :disabled="!canExportReportJson" @click="exportReportJsonAction">Export report JSON</button>
          <button type="button" :disabled="!canExportReportXlsx" @click="exportReportXlsxAction">Export report XLSX</button>
          <button type="button" :disabled="!canOpenReportsDir" @click="openReportsDirAction">Open reports folder</button>
        </div>
      </section>

      <section v-if="showValidationResults" class="pane">
        <h3>Options</h3>
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
            <div class="muted">Saved to app data under <span class="mono">reports/</span>.</div>
          </div>
        </div>

        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="wfEnableDraft"
              @change="wfEnableDraft = ($event.target as HTMLInputElement).checked; setCheckedValue('valWfEnable', wfEnableDraft)"
            />
            Walk-forward
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
            CSCV/PBO (overfitting risk)
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
        <div class="metaLine mono">Selected {{ crossSelectedCount }}/{{ crossTotalCount }}</div>

        <div class="crossList">
          <div
            v-for="(item, index) in renderedCrossDatasets"
            :key="`cross_item_${item.path || item.label}_${index}`"
            class="crossItem"
            @click="toggleCrossDataset(index)"
          >
            <div class="left">
              <div class="mono">{{ item.label }}</div>
              <div class="muted">{{ item.meta }}</div>
            </div>
            <input type="checkbox" :checked="item.checked" :disabled="item.disabled" @click.prevent="toggleCrossDataset(index)" />
          </div>
        </div>
      </section>

      <section v-if="showValidationResults" class="pane">
        <div class="metaLine mono">{{ validationMeta }}</div>
        <div class="metaLine mono">{{ validationShortlistMeta }}</div>
        <div class="metaLine mono">Table visible: {{ tableVisible ? 'yes' : 'no' }}</div>
        <div ref="tableScrollElementRef" class="tableWrap">
          <table class="validationTableVue">
            <thead>
              <tr>
                <th
                  v-for="(column, columnIndex) in renderedColumns"
                  :key="`val_col_${columnIndex}_${column.label}`"
                  :class="{ left: column.alignLeft }"
                >
                  {{ column.label }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="virtualTopSpacerPx > 0" class="spacerRow" aria-hidden="true">
                <td :colspan="Math.max(1, renderedColumns.length)" :style="{ height: `${virtualTopSpacerPx}px` }"></td>
              </tr>
              <tr v-for="virtualRow in virtualRows" :key="`val_row_${virtualRow.index}`">
                <template v-for="cell in virtualRow.row.cells" :key="`val_row_${virtualRow.index}_${cell.text}_${cell.className}`">
                  <td
                    :class="cell.className"
                    :title="cell.title || undefined"
                    :colspan="virtualRow.row.cells.length === 1 && renderedColumns.length > 1 ? renderedColumns.length : undefined"
                  >
                    {{ cell.text }}
                  </td>
                </template>
              </tr>
              <tr v-if="virtualBottomSpacerPx > 0" class="spacerRow" aria-hidden="true">
                <td :colspan="Math.max(1, renderedColumns.length)" :style="{ height: `${virtualBottomSpacerPx}px` }"></td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section v-if="showValidationResults" class="pane">
        <h3>Raw JSON</h3>
        <pre class="mono preview">{{ validationOutputPreview }}</pre>
      </section>

      <div class="status mono">{{ statusLine }}</div>
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
  align-items: center;
  gap: 0.75rem;
}

.title {
  font-size: 0.95rem;
  font-weight: 650;
}

.meta {
  font-size: 0.78rem;
  color: var(--muted);
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.pane h3 {
  margin: 0 0 0.55rem;
  font-size: 0.86rem;
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
  color: var(--muted);
}

.tableWrap {
  margin-top: 0.55rem;
  overflow: auto;
}

.validationTableVue {
  width: 100%;
  border-collapse: collapse;
  min-width: 980px;
}

.validationTableVue th,
.validationTableVue td {
  border-bottom: 1px solid var(--border);
  padding: 0.36rem 0.44rem;
  text-align: right;
  vertical-align: top;
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
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0.25rem;
}

.crossItem {
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 0.42rem 0.5rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.7rem;
}

.crossItem:hover {
  border-color: var(--border);
  background: color-mix(in oklab, var(--panel) 92%, var(--panel-footer));
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1100px) {
  .gridFive,
  .gridThree {
    grid-template-columns: 1fr 1fr;
  }
}

@media (max-width: 680px) {
  .gridFive,
  .gridThree {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
[data-vue-panel='validate'][data-vue-validation='1'] > :not(#validationVueShellMount) {
  display: none !important;
}
</style>

