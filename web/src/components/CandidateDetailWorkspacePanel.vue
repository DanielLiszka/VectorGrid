<script setup lang="ts">
import { computed, markRaw, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useDrillPriceChartRuntime } from '../composables/useDrillPriceChartRuntime'
import { useShellMount } from '../composables/useShellMount'
import { registerDrillChartController, type DrillChartController } from '../services/analysisService'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'
import {
  buildDrillTradeTableModel,
  type DrillChartModel,
  type DrillTradeTableModel,
} from '../visualizations/drillChartModel'
import PriceChartPanel from './visualizations/PriceChartPanel.vue'
import AppMetricStrip, { type AppMetricItem } from './ui/AppMetricStrip.vue'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  statusLine,
  updatedAtUnixMs,
  drillChartModel,
  drillTradeTableModel,
  selectedTopDecisionNote,
  selectedTopProvenanceChips,
  selectedTopSummary,
  selectedTopTitle,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#drillVueShellMount', {
  paneSelector: "[data-vue-panel='drill']",
  paneDatasetKey: 'vueDrill',
})

const visible = computed(() => String(resultsTab.value || '').trim() === 'drill')
const selectedTradeIdx = ref(-1)
const tradeRangeStart = ref(0)
const chartMetaText = ref('Hover for OHLC/MA values; select a Top result or click a marker to focus a trade.')

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))
const drillContentVisible = computed(() => analysisStore.drillWrapVisible || Boolean(drillChartModel.value))
const tradeTableVisible = computed(() => Boolean(drillTradeTableModel.value?.visible))
const tradeStatsText = computed(() => drillTradeTableModel.value?.statsText || '-')
const tradeRows = computed(() => drillTradeTableModel.value?.rows ?? [])
const hasTradeEvidence = computed(() => Number(drillChartModel.value?.trades?.length ?? 0) > 0)

function cleanText(value: unknown, fallback = ''): string {
  const text = String(value ?? '').trim()
  if (!text || text === '-' || text === '—' || text === 'undefined' || text === 'NaN') return fallback
  return text
}

function formatMetricNumber(value: unknown, digits = 2): string {
  const next = Number(value)
  if (!Number.isFinite(next)) return ''
  if (Number.isInteger(next)) return String(next)
  return next.toFixed(Math.max(0, digits)).replace(/(\.\d*?[1-9])0+$/u, '$1').replace(/\.0+$/u, '')
}

function formatMetricPercent(value: unknown): string {
  const next = Number(value)
  if (!Number.isFinite(next)) return ''
  const pct = Math.abs(next) <= 1 ? next * 100 : next
  return `${formatMetricNumber(pct, 1)}%`
}

const selectedCandidateTitle = computed(() => {
  const explicit = cleanText(selectedTopTitle.value)
  if (explicit && explicit !== 'No ranked candidate selected.') return explicit
  if (drillContentVisible.value) return 'Selected candidate evidence'
  return 'No ranked candidate selected.'
})

const selectedCandidateSummary = computed(() => {
  const explicit = cleanText(selectedTopSummary.value)
  if (
    explicit &&
    explicit !== 'No ranked candidate selected.' &&
    explicit !== 'No ranked candidates are available.'
  ) {
    return explicit
  }
  if (drillContentVisible.value) return 'Chart, trade, drawdown, and stability evidence are ready for the current drilldown target.'
  return 'Select a Top Results row to compute candidate detail curves.'
})

const selectedCandidateDecisionNote = computed(() => {
  const explicit = cleanText(selectedTopDecisionNote.value)
  if (
    explicit &&
    explicit !== 'Select a ranked candidate to lock a review target before moving into Candidate Detail or Validation.'
  ) {
    return explicit
  }
  return drillContentVisible.value
    ? 'Use trade markers and stability evidence to confirm whether this candidate deserves validation.'
    : 'Candidate Detail stays empty until a ranked row has been selected from Top Results.'
})

const chartEvidenceChips = computed(() => {
  const model = drillChartModel.value
  if (!model) return ['Awaiting Top Results selection']
  const chips = [
    model.bars.length > 0 ? `${model.bars.length} price bars` : '',
    model.trades.length > 0 ? `${model.trades.length} trades` : 'No trades',
    model.fastData.length || model.slowData.length ? 'MA overlays' : '',
    model.halftrendData.length ? 'Halftrend' : '',
    model.maczData.length ? 'MAC-Z' : '',
  ]
  return chips.filter((chip) => chip.length > 0)
})

const candidateProvenanceChips = computed(() => {
  const explicit = Array.isArray(selectedTopProvenanceChips.value)
    ? selectedTopProvenanceChips.value.map((chip) => cleanText(chip)).filter((chip) => chip.length > 0)
    : []
  const seen = new Set<string>()
  return [...explicit, ...chartEvidenceChips.value]
    .filter((chip) => {
      if (seen.has(chip)) return false
      seen.add(chip)
      return true
    })
    .slice(0, 6)
})

const drillEvidenceStats = computed<AppMetricItem[]>(() => {
  const model = drillChartModel.value
  const stats = model?.tradeStats ?? {}
  const closedTrades = formatMetricNumber(stats.closed_trades)
  const winRate = formatMetricPercent(stats.win_rate)
  const profitFactor = formatMetricNumber(stats.profit_factor, 2)
  const expectancy = formatMetricNumber(stats.expectancy, 4)
  const rows = [
    { key: 'bars', label: 'Bars', value: formatMetricNumber(model?.bars.length ?? Number.NaN) },
    { key: 'trades', label: 'Trades', value: formatMetricNumber(model?.trades.length ?? Number.NaN) },
    { key: 'closed', label: 'Closed', value: closedTrades },
    { key: 'win-rate', label: 'Win rate', value: winRate },
    { key: 'profit-factor', label: 'Profit factor', value: profitFactor },
    { key: 'expectancy', label: 'Expectancy', value: expectancy },
  ]
  return rows.filter((row) => cleanText(row.value).length > 0).slice(0, 6)
})

const chartRuntime = useDrillPriceChartRuntime({
  onTradeMarkerSelect: (tradeIdx) => {
    if (selectTradeByIndex(tradeIdx)) {
      setPanelStatus(`Selected trade ${tradeIdx + 1}.`)
    }
  },
  onMetaChange: (message) => {
    chartMetaText.value = message
  },
})

function reviewTopResults(): void {
  workflowStore.selectResultsTab('top')
}

function setDrillChartHost(element: HTMLElement | null): void {
  chartRuntime.setHost(element)
  if (!element || !visible.value || !mountReady.value) return
  void renderCurrentChart()
}

function setPanelStatus(message: string): void {
  analysisStore.statusLine = message
  analysisStore.source = 'vue'
  analysisStore.updatedAtUnixMs = Date.now()
}

function publishTradeTable(model: DrillChartModel | null = drillChartModel.value): DrillTradeTableModel | null {
  if (!model) {
    analysisStore.drillTradeTableModel = null
    analysisStore.tradeStatsWrapVisible = false
    return null
  }
  const tableModel = buildDrillTradeTableModel({
    chartModel: model,
    selectedTradeIdx: selectedTradeIdx.value,
    rangeStart: tradeRangeStart.value,
    maxRows: 200,
  })
  tradeRangeStart.value = tableModel.start
  analysisStore.drillTradeTableModel = markRaw(tableModel)
  analysisStore.tradeStatsWrapVisible = tableModel.visible
  return tableModel
}

function selectTradeByIndex(tradeIdxRaw: unknown): boolean {
  const model = drillChartModel.value
  const tradeIdx = Math.floor(Number(tradeIdxRaw))
  if (!model || !Number.isFinite(tradeIdx) || tradeIdx < 0 || tradeIdx >= model.trades.length) {
    return false
  }
  selectedTradeIdx.value = tradeIdx
  publishTradeTable(model)
  chartRuntime.updateSelectedTrade(model, tradeIdx)
  return true
}

function selectTradeFromRow(tradeIdx: number): void {
  if (selectTradeByIndex(tradeIdx)) {
    setPanelStatus(`Selected trade ${tradeIdx + 1}.`)
  }
}

function onTradeRowKeydown(event: KeyboardEvent, tradeIdx: number): void {
  if (event.key !== 'Enter' && event.key !== ' ') return
  event.preventDefault()
  selectTradeFromRow(tradeIdx)
}

function tradeRowLabel(tradeIdx: number): string {
  return `Select trade ${tradeIdx + 1}`
}

function clearTradeSelection(): boolean {
  if (selectedTradeIdx.value < 0) return false
  selectedTradeIdx.value = -1
  const model = drillChartModel.value
  publishTradeTable(model)
  chartRuntime.updateSelectedTrade(model, -1)
  return true
}

function clearTradeSelectionFromPanel(): void {
  if (clearTradeSelection()) {
    setPanelStatus('Cleared selected trade.')
    return
  }
  setPanelStatus('No trade is currently selected.')
}

function selectNextTrade(): string | null {
  const model = drillChartModel.value
  if (!model?.trades.length) return null
  const next = selectedTradeIdx.value < 0
    ? 0
    : selectedTradeIdx.value >= model.trades.length - 1
      ? -1
      : selectedTradeIdx.value + 1
  if (next < 0 || !selectTradeByIndex(next)) return null
  return String(next + 1)
}

function selectNextTradeFromPanel(): void {
  const next = selectNextTrade()
  setPanelStatus(next ? `Selected trade ${next}.` : 'No next trade to select.')
}

function selectPrevTrade(): string | null {
  const model = drillChartModel.value
  if (!model?.trades.length) return null
  const prev = selectedTradeIdx.value < 0
    ? model.trades.length - 1
    : selectedTradeIdx.value <= 0
      ? -1
      : selectedTradeIdx.value - 1
  if (prev < 0 || !selectTradeByIndex(prev)) return null
  return String(prev + 1)
}

function selectPrevTradeFromPanel(): void {
  const prev = selectPrevTrade()
  setPanelStatus(prev ? `Selected trade ${prev}.` : 'No previous trade to select.')
}

const drillChartController: DrillChartController = {
  clearTradeSelection,
  selectPrevTrade,
  selectNextTrade,
  selectTradeByIndex,
}

registerDrillChartController(drillChartController)

async function renderCurrentChart(): Promise<void> {
  await nextTick()
  if (!visible.value || !mountReady.value) return
  await chartRuntime.render(drillChartModel.value, selectedTradeIdx.value)
}

async function ensureVisibleDrill(): Promise<void> {
  await nextTick()
  await analysisStore.rerenderDrill()
  await renderCurrentChart()
}

watch([visible, mountReady], async ([nextVisible, nextReady]) => {
  if (!nextVisible || !nextReady) return
  await ensureVisibleDrill()
}, { immediate: true })

watch(drillChartModel, async (model) => {
  selectedTradeIdx.value = -1
  tradeRangeStart.value = 0
  publishTradeTable(model)
  await renderCurrentChart()
}, { immediate: true })

onBeforeUnmount(() => {
  registerDrillChartController(null)
  chartRuntime.dispose()
})
</script>

<template>
  <Teleport v-if="mountReady" to="#drillVueShellMount">
    <section class="drillPanelVueRoot drillVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Candidate evidence</div>
          <h2 class="panelTitle">Candidate Detail</h2>
          <div class="panelSubtitle">{{ selectedCandidateSummary }}</div>
        </div>
        <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
      </header>

      <section class="pane candidateDecisionPane">
        <div class="candidateDecisionHeader">
          <div class="candidateDecisionCopy">
            <div class="panelEyebrow">Current review target</div>
            <h3 class="candidateDecisionTitle">{{ selectedCandidateTitle }}</h3>
            <div class="panelMuted">{{ selectedCandidateDecisionNote }}</div>
          </div>
          <div class="actions candidateDecisionActions">
            <button type="button" @click="reviewTopResults">Review Top results</button>
          </div>
        </div>
        <div class="panelSummaryChips" aria-label="Candidate detail provenance">
          <span
            v-for="chip in candidateProvenanceChips"
            :key="`drill_chip_${chip}`"
            class="panelSummaryChip mono"
          >
            {{ chip }}
          </span>
        </div>
        <AppMetricStrip
          v-if="drillEvidenceStats.length > 0"
          class="candidateMetricStrip"
          :items="drillEvidenceStats"
          aria-label="Candidate detail metrics"
          min-item-width="5.8rem"
          size="compact"
          surface="sunken"
        />
      </section>

      <section class="pane">
        <div id="drillEmptyHintVue" class="panelStatus" role="status" aria-live="polite">
          Select a row in the Top tab to compute candidate detail curves.
        </div>
        <div class="chartAssist">
          <div id="drillKeyboardHint" class="panelMuted">Use the Top results table to select another candidate, or click chart markers if you prefer direct chart interaction.</div>
          <div class="actions">
            <button type="button" @click="reviewTopResults">Review Top results</button>
          </div>
        </div>
          <div class="panelAdvisory">
            If the charts stay empty, confirm that a Top result is selected and that the latest optimization run finished writing analysis results.
          </div>
          <div
            id="drillWrapVue"
            :style="{ marginTop: '0.75rem', display: drillContentVisible ? 'block' : 'none' }"
          >
            <div class="panelStatus" style="margin-bottom: 0.5rem;" id="drillMetaVue" role="status" aria-live="polite">-</div>
            <div class="panelStatus" style="margin-bottom: 0.5rem;" id="drillStatusVue" role="status" aria-live="polite">Click a Top row to view candidate detail curves.</div>
          <div class="chartEvidenceGrid">
            <div class="priceEvidencePane">
              <PriceChartPanel
                chart-id="drillTvChartVue"
                meta-id="drillTvChartMetaVue"
                title="Price + trades (TradingView Lightweight Charts)"
                :meta-text="chartMetaText"
                frame-aria-label="Backtest price and trade chart for the selected candidate. Use the Top results tab to select a different candidate without clicking inside the chart."
                host-aria-label="Backtest price chart"
                described-by="drillKeyboardHint"
                @host-ready="setDrillChartHost"
              />
            </div>
            <div class="supportChartGrid">
              <div class="supportChartPane">
                <div class="panelMuted">Equity (min/max per bin)</div>
                <canvas id="equityCanvasVue" class="chart supportChart" width="1024" height="160" role="img" aria-label="Equity envelope chart for the selected candidate"></canvas>
              </div>
              <div class="supportChartPane">
                <div class="panelMuted">Drawdown (min/max per bin)</div>
                <canvas id="drawdownCanvasVue" class="chart supportChart" width="1024" height="160" role="img" aria-label="Drawdown envelope chart for the selected candidate"></canvas>
              </div>
            </div>
          </div>

          <div id="stabilityWrapVue" style="margin-top: 1.1rem; display: none;">
            <div class="panelSectionHeader compactSectionHeader">
              <div>
                <div class="panelEyebrow">Neighborhood</div>
                <h3 class="panelPaneTitle">Stability</h3>
              </div>
            </div>
            <div class="panelStatus" style="margin-bottom: 0.35rem;" id="stabilityStatusVue" role="status" aria-live="polite">-</div>
            <div class="panelStatus" style="margin-bottom: 0.6rem;" id="stabilityMetaVue" role="status" aria-live="polite">-</div>
            <div class="panelStatus" style="margin-bottom: 0.6rem;" id="rejectMetaVue" role="status" aria-live="polite">-</div>
            <div class="table-wrap" id="stabilityTableWrapVue" style="display: none; margin-top: 0.45rem;" tabindex="0" aria-label="Stability neighborhood table">
              <table>
                <caption class="srOnly">Stability neighborhood metrics around the selected fast and slow parameter pair.</caption>
                <thead>
                  <tr>
                    <th class="left">Fast</th>
                    <th class="left">Slow</th>
                    <th>Score</th>
                    <th>PnL</th>
                    <th>Sharpe</th>
                    <th>MaxDD</th>
                    <th>Trades</th>
                  </tr>
                </thead>
                <tbody id="stabilityTableBodyVue"></tbody>
              </table>
            </div>
            <div class="panelMuted" style="margin-top: 0.5rem;">Evaluates a small neighborhood around the selected MA lengths to detect sharp peaks.</div>
          </div>

          <div
            id="tradeStatsWrapVue"
            :style="{ marginTop: '1.1rem', display: tradeTableVisible ? 'block' : 'none' }"
          >
            <div class="tradeEvidenceHeader">
              <div>
                <div class="panelEyebrow">Trade evidence</div>
                <h3 class="panelPaneTitle">Trades</h3>
              </div>
              <div class="actions tradeEvidenceActions">
                <button type="button" :disabled="!hasTradeEvidence" @click="selectPrevTradeFromPanel()">Previous trade</button>
                <button type="button" :disabled="!hasTradeEvidence" @click="selectNextTradeFromPanel()">Next trade</button>
                <button type="button" :disabled="!hasTradeEvidence" @click="clearTradeSelectionFromPanel()">Clear selection</button>
              </div>
            </div>
            <div class="mono" id="tradeStatsVue" role="status" aria-live="polite">{{ tradeStatsText }}</div>
            <div class="table-wrap" style="margin-top: 0.6rem;" tabindex="0" aria-label="Trade list table">
              <table>
                <caption class="srOnly">Trade-level results for the selected candidate, including entry, exit, duration, profit and loss, and current state.</caption>
                <thead>
                  <tr>
                    <th class="left">#</th>
                    <th class="left">Dir</th>
                    <th class="left">Entry</th>
                    <th class="left">Entry time</th>
                    <th class="left">Exit</th>
                    <th class="left">Exit time</th>
                    <th>Bars</th>
                    <th>PnL</th>
                    <th class="left">State</th>
                  </tr>
                </thead>
                <tbody id="tradeTableBodyVue">
                  <tr
                    v-for="row in tradeRows"
                    :key="row.tradeIdx"
                    :data-trade-idx="String(row.tradeIdx)"
                    :class="{ selected: row.selected }"
                    tabindex="0"
                    :aria-selected="row.selected ? 'true' : 'false'"
                    :aria-label="tradeRowLabel(row.tradeIdx)"
                    @click="selectTradeFromRow(row.tradeIdx)"
                    @keydown="onTradeRowKeydown($event, row.tradeIdx)"
                  >
                    <td
                      v-for="(cell, cellIndex) in row.cells"
                      :key="cellIndex"
                      :class="cell.className"
                      :title="cell.title || undefined"
                    >
                      {{ cell.text }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div class="panelMuted" style="margin-top: 0.5rem;">Showing up to 200 trades. Select a Top result or click a marker to focus a specific trade.</div>
          </div>
        </div>
      </section>

      <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</div>
    </section>
  </Teleport>
</template>

<style scoped>
.drillPanelVueRoot {
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
  padding: var(--density-pane-y) var(--density-pane-x);
  min-width: 0;
}

.candidateDecisionPane {
  display: grid;
  gap: 0.65rem;
}

.candidateDecisionHeader,
.tradeEvidenceHeader {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
  min-width: 0;
}

.candidateDecisionCopy {
  min-width: 0;
  display: grid;
  gap: 0.18rem;
}

.candidateDecisionTitle {
  margin: 0;
  font-size: 1rem;
  line-height: var(--leading-tight);
  font-weight: 700;
  overflow-wrap: anywhere;
}

.chartEvidenceGrid {
  display: grid;
  gap: 0.85rem;
}

.priceEvidencePane,
.supportChartPane {
  min-width: 0;
}

.supportChartGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.supportChartPane {
  display: grid;
  gap: 0.35rem;
}

.supportChart {
  min-height: 118px;
}

.compactSectionHeader {
  margin-bottom: 0.45rem;
}

.actions {
  margin-top: 0.65rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.candidateDecisionActions,
.tradeEvidenceActions {
  margin-top: 0;
  flex: 0 0 auto;
  justify-content: flex-end;
}

.actions button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.chartAssist {
  margin-top: 0.75rem;
  display: grid;
  gap: 0.35rem;
}

.chartSurface:focus-visible {
  outline: 2px solid var(--focus-border);
  outline-offset: 2px;
  box-shadow: var(--focus-ring);
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

.mono {
  font-family: var(--mono);
}

@media (max-width: 980px) {
  .supportChartGrid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 680px) {
  .head,
  .candidateDecisionHeader,
  .tradeEvidenceHeader {
    display: grid;
  }

  .supportChartGrid {
    grid-template-columns: 1fr;
  }

  .candidateDecisionActions,
  .tradeEvidenceActions {
    justify-content: flex-start;
  }
}
</style>

<style>
[data-vue-panel='drill'][data-vue-drill='1'] > :not(#drillVueShellMount) {
  display: none !important;
}
</style>


