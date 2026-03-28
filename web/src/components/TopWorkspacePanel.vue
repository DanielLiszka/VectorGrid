<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { useUiReadiness } from '../composables/useUiReadiness'
import { useVirtualTableRows } from '../composables/useVirtualTableRows'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

interface TopColumn {
  key: string
  label: string
  sort: '' | 'asc' | 'desc'
  alignLeft: boolean
}

interface TopCell {
  text: string
  className: string
  title: string
}

interface TopRow {
  selected: boolean
  cells: TopCell[]
}

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const { topStage } = useUiReadiness()
const {
  filterMinPnl,
  filterMinSharpe,
  filterMaxDd,
  filterMinTrades,
  filterMinExposure,
  canClearTopFilters,
  topRows,
  topColumns,
  topTableRows,
  topFilterMeta,
  topSelectionGuardrail,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#topVueShellMount')

const minPnlDraft = ref('')
const minSharpeDraft = ref('')
const maxDdDraft = ref('')
const minTradesDraft = ref('')
const minExposureDraft = ref('')

const visible = computed(() => String(resultsTab.value || '').trim() === 'top')
const renderedColumns = computed<TopColumn[]>(() => topColumns.value as TopColumn[])
const renderedRows = computed<TopRow[]>(() => topTableRows.value as TopRow[])
const hasTopResults = computed(() => Number(topRows.value || 0) > 0)
const topDisabledForRun = computed(() => String(topFilterMeta.value || '').toLowerCase().includes('top k disabled'))
const filtersActive = computed(() =>
  [
    minPnlDraft.value,
    minSharpeDraft.value,
    maxDdDraft.value,
    minTradesDraft.value,
    minExposureDraft.value,
  ].some((value) => String(value || '').trim().length > 0),
)
const emptyTitle = computed(() => {
  if (topStage.value === 'no_data') return 'Load data to unlock ranked results.'
  if (topStage.value === 'data_loaded_no_run') return 'No ranked runs yet.'
  if (topDisabledForRun.value) return 'This run returned only the best winner.'
  return filtersActive.value ? 'No rows match the current filters.' : 'No ranked runs yet.'
})
const emptyBody = computed(() =>
  topStage.value === 'no_data'
    ? 'Load a dataset first, then run an optimization to populate the ranking table.'
    : topStage.value === 'data_loaded_no_run'
      ? 'Your dataset is ready. Run an optimization or load prior analysis results to populate the ranked table.'
      : topDisabledForRun.value
        ? 'Increase Advanced > Top K above 0 and rerun optimization to populate the ranked shortlist table.'
      : filtersActive.value
        ? 'Broaden or clear the active filters to repopulate the ranking table.'
        : 'Run an optimization or load prior analysis results to populate the ranked table.',
)
const {
  scrollElementRef: tableScrollElementRef,
  virtualRows,
  topSpacerPx: virtualTopSpacerPx,
  bottomSpacerPx: virtualBottomSpacerPx,
  scrollToTop,
} = useVirtualTableRows(renderedRows)

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

const topStageLabel = computed(() => {
  if (topStage.value === 'no_data') return 'Awaiting dataset'
  if (topStage.value === 'data_loaded_no_run') return 'Awaiting optimization'
  if (topDisabledForRun.value) return 'Winner only'
  return hasTopResults.value ? 'Ranked shortlist ready' : 'No matching rows'
})

const topSummaryChips = computed(() => {
  return [
    topStageLabel.value,
    `${topRows.value} rows`,
    filtersActive.value ? 'Filters active' : 'All rows',
    topDisabledForRun.value ? 'Top-k disabled' : '',
  ].filter((chip) => String(chip || '').trim().length > 0)
})
const selectedTopSummary = computed(() => {
  const selectedIndex = renderedRows.value.findIndex((row) => row.selected)
  if (selectedIndex < 0) {
    return hasTopResults.value
      ? 'No ranked candidate selected.'
      : 'No ranked candidates are available.'
  }
  const selectedRow = renderedRows.value[selectedIndex]
  if (!selectedRow) {
    return hasTopResults.value
      ? 'No ranked candidate selected.'
      : 'No ranked candidates are available.'
  }
  const leadCells = selectedRow.cells
    .map((cell) => String(cell.text || '').trim())
    .filter((value) => value.length > 0)
    .slice(0, 3)
  const leadSummary = leadCells.join(' · ')
  return leadSummary
    ? `Selected candidate ${selectedIndex + 1} of ${renderedRows.value.length}: ${leadSummary}.`
    : `Selected candidate ${selectedIndex + 1} of ${renderedRows.value.length}.`
})

watch(
  () => [
    String(filterMinPnl.value || ''),
    String(filterMinSharpe.value || ''),
    String(filterMaxDd.value || ''),
    String(filterMinTrades.value || ''),
    String(filterMinExposure.value || ''),
  ] as const,
  ([nextMinPnl, nextMinSharpe, nextMaxDd, nextMinTrades, nextMinExposure]) => {
    if (minPnlDraft.value !== nextMinPnl) minPnlDraft.value = nextMinPnl
    if (minSharpeDraft.value !== nextMinSharpe) minSharpeDraft.value = nextMinSharpe
    if (maxDdDraft.value !== nextMaxDd) maxDdDraft.value = nextMaxDd
    if (minTradesDraft.value !== nextMinTrades) minTradesDraft.value = nextMinTrades
    if (minExposureDraft.value !== nextMinExposure) minExposureDraft.value = nextMinExposure
  },
  { immediate: true },
)

function setInputValue(id: string, nextValue: string): void {
  analysisStore.setInputField(id, nextValue)
}

const { queueInputValue: queueFilterInputValue, flushQueuedInputValuesNow: flushQueuedFilterInputsNow } =
  useQueuedInputWriter(setInputValue, { waitMs: 130, maxWaitMs: 360 })

function queueFilterInput(id: string, nextValue: string): void {
  queueFilterInputValue(id, nextValue)
}

function clickTopHeader(key: string): void {
  const sortKey = String(key || '').trim()
  if (!sortKey) return
  analysisStore.sortTopTable(sortKey)
}

function sortStateToAria(sort: TopColumn['sort']): 'ascending' | 'descending' | 'none' {
  if (sort === 'asc') return 'ascending'
  if (sort === 'desc') return 'descending'
  return 'none'
}

function clickTopRow(index: number): void {
  if (!Number.isInteger(index) || index < 0) return
  analysisStore.selectTopRow(index)
}

function focusAdjacentTopRow(currentTarget: EventTarget | null, direction: 'prev' | 'next' | 'first' | 'last'): void {
  const row = currentTarget instanceof HTMLElement ? currentTarget : null
  const tbody = row?.parentElement
  if (!row || !tbody) return
  const rows = Array.from(tbody.querySelectorAll<HTMLElement>('tr[data-top-row="1"]'))
  if (!rows.length) return
  const currentIndex = rows.indexOf(row)
  if (currentIndex < 0) return
  let nextIndex = currentIndex
  if (direction === 'prev') nextIndex = Math.max(0, currentIndex - 1)
  else if (direction === 'next') nextIndex = Math.min(rows.length - 1, currentIndex + 1)
  else if (direction === 'first') nextIndex = 0
  else nextIndex = rows.length - 1
  rows[nextIndex]?.focus()
}

function onTopRowKeydown(index: number, event: KeyboardEvent): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    clickTopRow(index)
    return
  }
  if (event.key === 'ArrowUp') {
    event.preventDefault()
    focusAdjacentTopRow(event.currentTarget, 'prev')
    return
  }
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    focusAdjacentTopRow(event.currentTarget, 'next')
    return
  }
  if (event.key === 'Home') {
    event.preventDefault()
    focusAdjacentTopRow(event.currentTarget, 'first')
    return
  }
  if (event.key === 'End') {
    event.preventDefault()
    focusAdjacentTopRow(event.currentTarget, 'last')
  }
}

onMounted(async () => {
  await nextTick()
  scrollToTop()
})

onBeforeUnmount(() => {
  flushQueuedFilterInputsNow()
})
</script>

<template>
  <Teleport v-if="mountReady" to="#topVueShellMount">
    <section class="topPanelVueRoot topVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Ranking</div>
          <h2 class="panelTitle">Top Results</h2>
          <div class="panelSubtitle">
            Filter the ranked shortlist, inspect the best candidates, and send a selected row into the deeper analysis views.
          </div>
        </div>
        <div class="panelMeta mono panelNumeric">{{ topRows }} rows · {{ stamp }}</div>
      </header>

      <div class="panelSummaryChips" aria-label="Top results summary">
        <div v-for="chip in topSummaryChips" :key="chip" class="panelSummaryChip">{{ chip }}</div>
      </div>

      <section v-if="topStage === 'ready'" class="pane panelProminentPane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Filters</div>
            <h3 class="panelPaneTitle">Shortlist gates</h3>
            <div class="panelSectionLead">
              Tighten the ranked list by PnL, Sharpe, drawdown, trades, and exposure before choosing a candidate.
            </div>
          </div>
        </div>
        <div class="gridFive">
          <div>
            <label for="vueFilterMinPnl">Min PnL</label>
            <input
              id="vueFilterMinPnl"
              type="number"
              step="any"
              :value="minPnlDraft"
              @input="minPnlDraft = String(($event.target as HTMLInputElement).value || ''); queueFilterInput('filterMinPnl', minPnlDraft)"
            />
          </div>
          <div>
            <label for="vueFilterMinSharpe">Min Sharpe</label>
            <input
              id="vueFilterMinSharpe"
              type="number"
              step="any"
              :value="minSharpeDraft"
              @input="minSharpeDraft = String(($event.target as HTMLInputElement).value || ''); queueFilterInput('filterMinSharpe', minSharpeDraft)"
            />
          </div>
          <div>
            <label for="vueFilterMaxDd">Max MaxDD</label>
            <input
              id="vueFilterMaxDd"
              type="number"
              step="any"
              :value="maxDdDraft"
              @input="maxDdDraft = String(($event.target as HTMLInputElement).value || ''); queueFilterInput('filterMaxDd', maxDdDraft)"
            />
          </div>
          <div>
            <label for="vueFilterMinTrades">Min trades</label>
            <input
              id="vueFilterMinTrades"
              type="number"
              min="0"
              step="1"
              :value="minTradesDraft"
              @input="minTradesDraft = String(($event.target as HTMLInputElement).value || ''); queueFilterInput('filterMinTrades', minTradesDraft)"
            />
          </div>
          <div>
            <label for="vueFilterMinExposure">Min exposure</label>
            <input
              id="vueFilterMinExposure"
              type="number"
              step="any"
              :value="minExposureDraft"
              @input="minExposureDraft = String(($event.target as HTMLInputElement).value || ''); queueFilterInput('filterMinExposure', minExposureDraft)"
            />
          </div>
        </div>
        <div class="actions">
          <button type="button" :disabled="!canClearTopFilters" @click="analysisStore.clearTopFilters()">Clear filters</button>
        </div>
        <div class="panelAdvisory mono">{{ topFilterMeta }}</div>
        <div v-if="topSelectionGuardrail" class="panelAdvisory mono bad">{{ topSelectionGuardrail }}</div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Table</div>
            <h3 class="panelPaneTitle">Ranked candidates</h3>
            <div class="panelSectionLead">
              Sort the shortlist by the current metric, then choose a row to update the rest of the analysis workspace.
            </div>
          </div>
        </div>
        <div
          v-if="topStage === 'ready'"
          id="topSelectionStatus"
          class="stickySelectionSummary selectionRail topSelectionSummary"
          role="status"
          aria-live="polite"
          aria-atomic="true"
        >
          <div class="stickySelectionSummaryTitle">Current candidate</div>
          <div class="stickySelectionSummaryBody">{{ selectedTopSummary }}</div>
        </div>
        <div v-if="topStage !== 'ready' || !hasTopResults" class="emptyState topEmptyState">
          <div class="emptyTitle">{{ emptyTitle }}</div>
          <div class="emptyBody">{{ emptyBody }}</div>
          <div v-if="topStage === 'ready' && (filtersActive || canClearTopFilters)" class="emptyActions">
            <button type="button" :disabled="!canClearTopFilters" @click="analysisStore.clearTopFilters()">Clear filters</button>
          </div>
          <div class="panelMuted mono panelNumeric">{{ topFilterMeta || statusLine }}</div>
          <div v-if="topStage === 'ready' && topSelectionGuardrail" class="panelMuted mono panelNumeric">{{ topSelectionGuardrail }}</div>
        </div>

        <div
          v-else
          ref="tableScrollElementRef"
          class="tableWrap overflowShadowY overflowShadowX surfaceWorking"
          tabindex="0"
          aria-label="Top optimization results table"
          aria-describedby="topSelectionStatus"
        >
          <table class="topTableVue" role="grid" aria-describedby="topSelectionStatus">
            <caption class="srOnly">Top optimization results ranked by the selected table sort order.</caption>
            <thead role="rowgroup">
              <tr role="row">
                <th
                  v-for="column in renderedColumns"
                  :key="`top_col_${column.key}`"
                  :class="{ left: column.alignLeft }"
                  :aria-sort="sortStateToAria(column.sort)"
                  scope="col"
                  role="columnheader"
                >
                  <button
                    type="button"
                    class="headerButton"
                    :class="{ left: column.alignLeft }"
                    @click="clickTopHeader(column.key)"
                  >
                    <span>{{ column.label }}</span>
                    <span v-if="column.sort === 'asc'" class="sortGlyph" aria-hidden="true">↑</span>
                    <span v-else-if="column.sort === 'desc'" class="sortGlyph" aria-hidden="true">↓</span>
                  </button>
                </th>
              </tr>
            </thead>
            <tbody role="rowgroup">
              <tr v-if="virtualTopSpacerPx > 0" class="spacerRow" aria-hidden="true">
                <td :colspan="Math.max(1, renderedColumns.length)" :style="{ height: `${virtualTopSpacerPx}px` }"></td>
              </tr>
              <tr
                v-for="virtualRow in virtualRows"
                :key="`top_row_${virtualRow.index}`"
                :class="{ selected: virtualRow.row.selected }"
                data-top-row="1"
                role="row"
                :aria-selected="virtualRow.row.selected"
                tabindex="0"
                @click="clickTopRow(virtualRow.index)"
                @keydown="onTopRowKeydown(virtualRow.index, $event)"
              >
                <template v-for="cell in virtualRow.row.cells" :key="`top_row_${virtualRow.index}_cell_${cell.text}_${cell.className}`">
                  <td
                    :class="cell.className"
                    :title="cell.title || undefined"
                    :colspan="virtualRow.row.cells.length === 1 && renderedColumns.length > 1 ? renderedColumns.length : undefined"
                    role="gridcell"
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

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.topPanelVueRoot {
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
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
}

.gridFive {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.6rem;
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

.tableWrap {
  overflow: auto;
}

.topEmptyState {
  min-height: 260px;
  display: grid;
  align-content: center;
  gap: 0.65rem;
}

.topTableVue {
  width: 100%;
  border-collapse: collapse;
  min-width: 760px;
}

.topTableVue th,
.topTableVue td {
  border-bottom: 1px solid var(--border);
  padding: 0.38rem 0.45rem;
  text-align: right;
  vertical-align: top;
}

.topTableVue th {
  user-select: none;
}

.topTableVue th.left,
.topTableVue td.left {
  text-align: left;
}

.topTableVue tbody tr {
  cursor: pointer;
}

.topTableVue tbody tr:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: -2px;
}

.topTableVue tbody tr.selected {
  background: color-mix(in oklab, var(--selection-fill-strong) 84%, var(--surface-working));
}

.topTableVue tbody tr.selected td:first-child {
  box-shadow: inset 3px 0 0 var(--selection-rail);
}

.topSelectionSummary {
  margin-bottom: 0.65rem;
  top: -0.1rem;
}

.topTableVue tbody tr.spacerRow {
  pointer-events: none;
}

.topTableVue tbody tr.spacerRow td {
  border-bottom: none;
  padding: 0;
}

.sortGlyph {
  margin-left: 0.25rem;
  font-size: 0.72rem;
  opacity: 0.8;
}

.headerButton {
  width: 100%;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  display: inline-flex;
  justify-content: flex-end;
  align-items: center;
  gap: 0.25rem;
  font: inherit;
}

.headerButton.left {
  justify-content: flex-start;
}

.headerButton:hover:not(:disabled) {
  background: transparent;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1100px) {
  .gridFive {
    grid-template-columns: 1fr 1fr;
  }
}

@media (max-width: 680px) {
  .gridFive {
    grid-template-columns: 1fr;
  }
}
</style>




