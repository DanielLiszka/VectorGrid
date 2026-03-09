<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useUiReadiness } from '../composables/useUiReadiness'
import { useVirtualTableRows } from '../composables/useVirtualTableRows'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'

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

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

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

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
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

function clickTopRow(index: number): void {
  if (!Number.isInteger(index) || index < 0) return
  analysisStore.selectTopRow(index)
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="top"]') as HTMLElement | null
  const mount = document.querySelector('#topVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueTop = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  scrollToTop()
})

onBeforeUnmount(() => {
  flushQueuedFilterInputsNow()
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueTop
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#topVueShellMount">
    <section class="topPanelVueRoot topVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Top Results</div>
        <div class="meta mono">{{ topRows }} rows · {{ stamp }}</div>
      </header>

      <section v-if="topStage === 'ready'" class="pane">
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
        <div class="metaLine mono">{{ topFilterMeta }}</div>
        <div class="metaLine bad mono">{{ topSelectionGuardrail }}</div>
      </section>

      <section class="pane">
        <div v-if="topStage !== 'ready' || !hasTopResults" class="emptyState topEmptyState">
          <div class="emptyTitle">{{ emptyTitle }}</div>
          <div class="emptyBody">{{ emptyBody }}</div>
          <div v-if="topStage === 'ready' && (filtersActive || canClearTopFilters)" class="emptyActions">
            <button type="button" :disabled="!canClearTopFilters" @click="analysisStore.clearTopFilters()">Clear filters</button>
          </div>
          <div class="muted mono">{{ topFilterMeta || statusLine }}</div>
          <div v-if="topStage === 'ready' && topSelectionGuardrail" class="muted mono">{{ topSelectionGuardrail }}</div>
        </div>

        <div v-else ref="tableScrollElementRef" class="tableWrap">
          <table class="topTableVue">
            <thead>
              <tr>
                <th
                  v-for="column in renderedColumns"
                  :key="`top_col_${column.key}`"
                  :class="{ left: column.alignLeft }"
                  @click="clickTopHeader(column.key)"
                >
                  <span>{{ column.label }}</span>
                  <span v-if="column.sort === 'asc'" class="sortGlyph" aria-hidden="true">↑</span>
                  <span v-else-if="column.sort === 'desc'" class="sortGlyph" aria-hidden="true">↓</span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="virtualTopSpacerPx > 0" class="spacerRow" aria-hidden="true">
                <td :colspan="Math.max(1, renderedColumns.length)" :style="{ height: `${virtualTopSpacerPx}px` }"></td>
              </tr>
              <tr
                v-for="virtualRow in virtualRows"
                :key="`top_row_${virtualRow.index}`"
                :class="{ selected: virtualRow.row.selected }"
                @click="clickTopRow(virtualRow.index)"
              >
                <template v-for="cell in virtualRow.row.cells" :key="`top_row_${virtualRow.index}_cell_${cell.text}_${cell.className}`">
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

      <div class="status mono">{{ statusLine }}</div>
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

.metaLine {
  margin-top: 0.45rem;
  font-size: 0.78rem;
  color: var(--muted);
}

.metaLine.bad {
  color: var(--danger);
}

.status {
  font-size: 0.78rem;
  color: var(--muted);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
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
  cursor: pointer;
  user-select: none;
}

.topTableVue th.left,
.topTableVue td.left {
  text-align: left;
}

.topTableVue tbody tr {
  cursor: pointer;
}

.topTableVue tbody tr.selected {
  background: color-mix(in oklab, var(--accent) 14%, transparent);
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

<style>
[data-vue-panel='top'][data-vue-top='1'] > :not(#topVueShellMount) {
  display: none !important;
}
</style>



