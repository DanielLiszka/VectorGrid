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

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  statusLine,
  updatedAtUnixMs,
  drillChartModel,
  drillTradeTableModel,
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

function clearTradeSelection(): boolean {
  if (selectedTradeIdx.value < 0) return false
  selectedTradeIdx.value = -1
  const model = drillChartModel.value
  publishTradeTable(model)
  chartRuntime.updateSelectedTrade(model, -1)
  return true
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
        <h2 class="panelTitle">Drilldown</h2>
        <div class="panelMeta mono panelNumeric">{{ stamp }}</div>
      </header>

      <section class="pane">
        <div id="drillEmptyHintVue" class="panelStatus" role="status" aria-live="polite">
          Select a row in the Top tab to compute drilldown curves.
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
            <div class="panelStatus" style="margin-bottom: 0.5rem;" id="drillStatusVue" role="status" aria-live="polite">Click a Top row to view curves.</div>
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
          <div>
            <div class="panelMuted" style="margin-bottom: 0.35rem;">Equity (min/max per bin)</div>
            <canvas id="equityCanvasVue" class="chart" width="1024" height="160" role="img" aria-label="Equity envelope chart for the selected candidate"></canvas>
          </div>
          <div style="margin-top: 0.75rem;">
            <div class="panelMuted" style="margin-bottom: 0.35rem;">Drawdown (min/max per bin)</div>
            <canvas id="drawdownCanvasVue" class="chart" width="1024" height="160" role="img" aria-label="Drawdown envelope chart for the selected candidate"></canvas>
          </div>

          <div id="stabilityWrapVue" style="margin-top: 1.1rem; display: none;">
            <h3 class="panelPaneTitle">Stability</h3>
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
            <h3 class="panelPaneTitle">Trades</h3>
            <div class="mono" id="tradeStatsVue" role="status" aria-live="polite">{{ tradeStatsText }}</div>
            <div class="table-wrap" style="margin-top: 0.6rem;" tabindex="0" aria-label="Trade list table">
              <table>
                <caption class="srOnly">Trade-level results for the selected candidate, including entry, exit, duration, profit and loss, and current state.</caption>
                <thead>
                  <tr>
                    <th class="left">#</th>
                    <th class="left">Dir</th>
                    <th class="left">Entry</th>
                    <th class="left">EntryTime</th>
                    <th class="left">Exit</th>
                    <th class="left">ExitTime</th>
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
                    @click="selectTradeFromRow(row.tradeIdx)"
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
  align-items: center;
  gap: 0.75rem;
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
  min-width: 0;
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
</style>

<style>
[data-vue-panel='drill'][data-vue-drill='1'] > :not(#drillVueShellMount) {
  display: none !important;
}
</style>


