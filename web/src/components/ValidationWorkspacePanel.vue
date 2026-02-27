<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
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
const tableScrollTop = ref(0)
const virtualRowHeightPx = 28
const virtualOverscanRows = 12
const virtualWindowRows = 80

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
const renderedColumns = computed<ValidationColumn[]>(() => validationColumns.value as ValidationColumn[])
const renderedRows = computed<ValidationRow[]>(() => validationRows.value as ValidationRow[])
const renderedCrossDatasets = computed<CrossDatasetItem[]>(() => crossDatasets.value as CrossDatasetItem[])
const virtualFirstRowIndex = computed(() => {
  const firstByScroll = Math.floor(tableScrollTop.value / virtualRowHeightPx)
  const maxFirst = Math.max(0, renderedRows.value.length - virtualWindowRows)
  return Math.min(maxFirst, Math.max(0, firstByScroll - virtualOverscanRows))
})
const virtualLastRowExclusive = computed(() => {
  return Math.min(renderedRows.value.length, virtualFirstRowIndex.value + virtualWindowRows)
})
const virtualRows = computed(() => renderedRows.value.slice(virtualFirstRowIndex.value, virtualLastRowExclusive.value))
const virtualTopSpacerPx = computed(() => virtualFirstRowIndex.value * virtualRowHeightPx)
const virtualBottomSpacerPx = computed(() => {
  const hiddenCount = Math.max(0, renderedRows.value.length - virtualLastRowExclusive.value)
  return hiddenCount * virtualRowHeightPx
})

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

watch(candidates, (v) => { const next = String(v || '32'); if (candidatesDraft.value !== next) candidatesDraft.value = next }, { immediate: true })
watch(seed, (v) => { const next = String(v || '1'); if (seedDraft.value !== next) seedDraft.value = next }, { immediate: true })
watch(minPsr, (v) => { const next = String(v || ''); if (minPsrDraft.value !== next) minPsrDraft.value = next }, { immediate: true })
watch(minDsr, (v) => { const next = String(v || ''); if (minDsrDraft.value !== next) minDsrDraft.value = next }, { immediate: true })
watch(cscvSlices, (v) => { const next = String(v || ''); if (cscvDraft.value !== next) cscvDraft.value = next }, { immediate: true })
watch(autoRun, (v) => { const next = Boolean(v); if (autoRunDraft.value !== next) autoRunDraft.value = next }, { immediate: true })
watch(autoExportXlsx, (v) => { const next = Boolean(v); if (autoExportXlsxDraft.value !== next) autoExportXlsxDraft.value = next }, { immediate: true })
watch(autoRevealReport, (v) => { const next = Boolean(v); if (autoRevealReportDraft.value !== next) autoRevealReportDraft.value = next }, { immediate: true })
watch(autoExportTopN, (v) => { const next = String(v || '20').trim() || '20'; if (autoExportTopNDraft.value !== next) autoExportTopNDraft.value = next }, { immediate: true })
watch(wfEnable, (v) => { const next = Boolean(v); if (wfEnableDraft.value !== next) wfEnableDraft.value = next }, { immediate: true })
watch(wfWindow, (v) => { const next = String(v || '20000').trim() || '20000'; if (wfWindowDraft.value !== next) wfWindowDraft.value = next }, { immediate: true })
watch(wfStep, (v) => { const next = String(v || '20000').trim() || '20000'; if (wfStepDraft.value !== next) wfStepDraft.value = next }, { immediate: true })
watch(perturbEnable, (v) => { const next = Boolean(v); if (perturbEnableDraft.value !== next) perturbEnableDraft.value = next }, { immediate: true })
watch(perturbIters, (v) => { const next = String(v || '8').trim() || '8'; if (perturbItersDraft.value !== next) perturbItersDraft.value = next }, { immediate: true })
watch(perturbMin, (v) => { const next = String(v || '0.015').trim() || '0.015'; if (perturbMinDraft.value !== next) perturbMinDraft.value = next }, { immediate: true })
watch(perturbMax, (v) => { const next = String(v || '0.06').trim() || '0.06'; if (perturbMaxDraft.value !== next) perturbMaxDraft.value = next }, { immediate: true })
watch(mcEnable, (v) => { const next = Boolean(v); if (mcEnableDraft.value !== next) mcEnableDraft.value = next }, { immediate: true })
watch(mcSims, (v) => { const next = String(v || '500').trim() || '500'; if (mcSimsDraft.value !== next) mcSimsDraft.value = next }, { immediate: true })
watch(cscvEnable, (v) => { const next = Boolean(v); if (cscvEnableDraft.value !== next) cscvEnableDraft.value = next }, { immediate: true })
watch(crossEnable, (v) => { const next = Boolean(v); if (crossEnableDraft.value !== next) crossEnableDraft.value = next }, { immediate: true })
watch(crossMax, (v) => { const next = String(v || '6').trim() || '6'; if (crossMaxDraft.value !== next) crossMaxDraft.value = next }, { immediate: true })
watch(crossSearch, (v) => { const next = String(v || '').trim(); if (crossSearchDraft.value !== next) crossSearchDraft.value = next }, { immediate: true })

function setInputValue(id: string, nextValue: string): void {
  validationStore.setInputField(id, nextValue)
}

function setCheckedValue(id: string, nextValue: boolean): void {
  validationStore.setCheckboxField(id, nextValue)
}

function runValidationAction(): void {
  validationStore.runValidation()
}

function toggleCrossDataset(index: number): void {
  const entry = renderedCrossDatasets.value[index]
  if (!entry || entry.disabled) return
  const path = String(entry.path || '').trim()
  if (!path) return
  validationStore.toggleCrossDataset(path, !entry.checked)
}

function onTableScroll(event: Event): void {
  const target = event.target as HTMLElement | null
  if (!target) return
  tableScrollTop.value = Math.max(0, Math.floor(target.scrollTop || 0))
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
  tableScrollTop.value = 0
})

onBeforeUnmount(() => {
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
              @input="candidatesDraft = String(($event.target as HTMLInputElement).value || '32'); setInputValue('valCandidates', candidatesDraft)"
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
              @input="seedDraft = String(($event.target as HTMLInputElement).value || '1'); setInputValue('valSeed', seedDraft)"
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
              @input="minPsrDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('valMinPsr', minPsrDraft)"
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
              @input="minDsrDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('valMinDsr', minDsrDraft)"
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
              @input="cscvDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('valCscvSlices', cscvDraft)"
            />
          </div>
        </div>

        <div class="actions">
          <button type="button" class="primary" :disabled="validateDisabled" @click="runValidationAction">Run validation</button>
          <button type="button" :disabled="!canExportValidationCsv" @click="validationStore.exportShortlistCsv()">Export shortlist CSV</button>
          <button type="button" :disabled="!canExportValidationJson" @click="validationStore.exportValidationJson()">Export validation JSON</button>
          <button type="button" :disabled="!canExportReportJson" @click="validationStore.exportReportJson()">Export report JSON</button>
          <button type="button" :disabled="!canExportReportXlsx" @click="validationStore.exportReportXlsx()">Export report XLSX</button>
          <button type="button" :disabled="!canOpenReportsDir" @click="validationStore.openReportsDir()">Open reports folder</button>
        </div>
      </section>

      <section class="pane">
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
              @input="autoExportTopNDraft = String(($event.target as HTMLInputElement).value || '20'); setInputValue('valAutoExportTopN', autoExportTopNDraft)"
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
              @input="wfWindowDraft = String(($event.target as HTMLInputElement).value || '20000'); setInputValue('valWfWindow', wfWindowDraft)"
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
              @input="wfStepDraft = String(($event.target as HTMLInputElement).value || '20000'); setInputValue('valWfStep', wfStepDraft)"
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
              @input="perturbItersDraft = String(($event.target as HTMLInputElement).value || '8'); setInputValue('valPerturbIters', perturbItersDraft)"
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
              @input="perturbMinDraft = String(($event.target as HTMLInputElement).value || '0.015'); setInputValue('valPerturbMin', perturbMinDraft)"
            />
          </div>
          <div>
            <label for="vueValPerturbMax">Perturb max %</label>
            <input
              id="vueValPerturbMax"
              type="number"
              step="any"
              :value="perturbMaxDraft"
              @input="perturbMaxDraft = String(($event.target as HTMLInputElement).value || '0.06'); setInputValue('valPerturbMax', perturbMaxDraft)"
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
              @input="mcSimsDraft = String(($event.target as HTMLInputElement).value || '500'); setInputValue('valMcSims', mcSimsDraft)"
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
              @input="crossMaxDraft = String(($event.target as HTMLInputElement).value || '6'); setInputValue('valCrossMax', crossMaxDraft)"
            />
          </div>
          <div>
            <label for="vueValCrossSearch">Search</label>
            <input
              id="vueValCrossSearch"
              type="text"
              :value="crossSearchDraft"
              @input="crossSearchDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('valCrossSearch', crossSearchDraft)"
            />
          </div>
          <div>
            <label>&nbsp;</label>
            <div class="actions" style="margin-top: 0;">
              <button type="button" :disabled="!canCrossRefresh" @click="validationStore.crossRefresh()">Refresh list</button>
              <button type="button" :disabled="!canCrossSelectAll" @click="validationStore.crossSelectAll()">All</button>
              <button type="button" :disabled="!canCrossSelectNone" @click="validationStore.crossSelectNone()">None</button>
              <button type="button" :disabled="!canCrossSelectMatch" @click="validationStore.crossSelectMatch()">Match current</button>
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

      <section class="pane">
        <div class="metaLine mono">{{ validationMeta }}</div>
        <div class="metaLine mono">{{ validationShortlistMeta }}</div>
        <div class="metaLine mono">Table visible: {{ tableVisible ? 'yes' : 'no' }}</div>
        <div class="tableWrap" @scroll="onTableScroll">
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
              <tr v-for="(row, rowOffset) in virtualRows" :key="`val_row_${virtualFirstRowIndex + rowOffset}`">
                <template v-for="cell in row.cells" :key="`val_row_${virtualFirstRowIndex + rowOffset}_${cell.text}_${cell.className}`">
                  <td
                    :class="cell.className"
                    :title="cell.title || undefined"
                    :colspan="row.cells.length === 1 && renderedColumns.length > 1 ? renderedColumns.length : undefined"
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

      <section class="pane">
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




