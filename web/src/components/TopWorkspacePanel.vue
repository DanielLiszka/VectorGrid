<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
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
const tableScrollTop = ref(0)
const virtualRowHeightPx = 28
const virtualOverscanRows = 12
const virtualWindowRows = 80

const minPnlDraft = ref('')
const minSharpeDraft = ref('')
const maxDdDraft = ref('')
const minTradesDraft = ref('')
const minExposureDraft = ref('')

const visible = computed(() => String(resultsTab.value || '').trim() === 'top')
const renderedColumns = computed<TopColumn[]>(() => topColumns.value as TopColumn[])
const renderedRows = computed<TopRow[]>(() => topTableRows.value as TopRow[])
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

watch(filterMinPnl, (v) => { const next = String(v || ''); if (minPnlDraft.value !== next) minPnlDraft.value = next }, { immediate: true })
watch(filterMinSharpe, (v) => { const next = String(v || ''); if (minSharpeDraft.value !== next) minSharpeDraft.value = next }, { immediate: true })
watch(filterMaxDd, (v) => { const next = String(v || ''); if (maxDdDraft.value !== next) maxDdDraft.value = next }, { immediate: true })
watch(filterMinTrades, (v) => { const next = String(v || ''); if (minTradesDraft.value !== next) minTradesDraft.value = next }, { immediate: true })
watch(filterMinExposure, (v) => { const next = String(v || ''); if (minExposureDraft.value !== next) minExposureDraft.value = next }, { immediate: true })

function setInputValue(id: string, nextValue: string): void {
  analysisStore.setInputField(id, nextValue)
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

function onTableScroll(event: Event): void {
  const target = event.target as HTMLElement | null
  if (!target) return
  tableScrollTop.value = Math.max(0, Math.floor(target.scrollTop || 0))
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
  tableScrollTop.value = 0
})

onBeforeUnmount(() => {
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

      <section class="pane">
        <div class="gridFive">
          <div>
            <label for="vueFilterMinPnl">Min PnL</label>
            <input
              id="vueFilterMinPnl"
              type="number"
              step="any"
              :value="minPnlDraft"
              @input="minPnlDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('filterMinPnl', minPnlDraft)"
            />
          </div>
          <div>
            <label for="vueFilterMinSharpe">Min Sharpe</label>
            <input
              id="vueFilterMinSharpe"
              type="number"
              step="any"
              :value="minSharpeDraft"
              @input="minSharpeDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('filterMinSharpe', minSharpeDraft)"
            />
          </div>
          <div>
            <label for="vueFilterMaxDd">Max MaxDD</label>
            <input
              id="vueFilterMaxDd"
              type="number"
              step="any"
              :value="maxDdDraft"
              @input="maxDdDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('filterMaxDd', maxDdDraft)"
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
              @input="minTradesDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('filterMinTrades', minTradesDraft)"
            />
          </div>
          <div>
            <label for="vueFilterMinExposure">Min exposure</label>
            <input
              id="vueFilterMinExposure"
              type="number"
              step="any"
              :value="minExposureDraft"
              @input="minExposureDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('filterMinExposure', minExposureDraft)"
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
        <div class="tableWrap" @scroll="onTableScroll">
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
                v-for="(row, rowOffset) in virtualRows"
                :key="`top_row_${virtualFirstRowIndex + rowOffset}`"
                :class="{ selected: row.selected }"
                @click="clickTopRow(virtualFirstRowIndex + rowOffset)"
              >
                <template v-for="cell in row.cells" :key="`top_row_${virtualFirstRowIndex + rowOffset}_cell_${cell.text}_${cell.className}`">
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
  color: var(--bad);
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



