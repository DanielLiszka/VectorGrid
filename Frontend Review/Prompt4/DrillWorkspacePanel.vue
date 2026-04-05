<script setup lang="ts">
import { computed, nextTick, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useShellMount } from '../composables/useShellMount'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#drillVueShellMount')

const visible = computed(() => String(resultsTab.value || '').trim() === 'drill')

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

function reviewTopResults(): void {
  workflowStore.selectResultsTab('top')
}

async function ensureVisibleDrill(): Promise<void> {
  await nextTick()
  await analysisStore.rerenderDrill()
}

watch([visible, mountReady], async ([nextVisible, nextReady]) => {
  if (!nextVisible || !nextReady) return
  await ensureVisibleDrill()
}, { immediate: true })

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
        <div id="drillWrapVue" style="margin-top: 0.75rem; display: none;">
          <div class="panelStatus" style="margin-bottom: 0.5rem;" id="drillMetaVue" role="status" aria-live="polite">-</div>
          <div class="panelStatus" style="margin-bottom: 0.5rem;" id="drillStatusVue" role="status" aria-live="polite">Click a Top row to view curves.</div>
          <div>
            <div class="panelMuted" style="margin-bottom: 0.35rem;">Price + trades (TradingView Lightweight Charts)</div>
            <div
              class="tvChartFrame chartSurface"
              aria-label="Backtest price and trade chart for the selected candidate. Use the Top results tab to select a different candidate without clicking inside the chart."
              aria-describedby="drillKeyboardHint"
            >
              <div id="drillTvChartVue" class="tvChartHost" role="img" aria-label="Backtest price chart"></div>
            </div>
            <div class="panelStatus" style="margin-top: 0.35rem;" id="drillTvChartMetaVue" role="status" aria-live="polite">Hover for OHLC/MA values; select a Top result or click a marker to focus a trade.</div>
          </div>
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

          <div id="tradeStatsWrapVue" style="margin-top: 1.1rem; display: none;">
            <h3 class="panelPaneTitle">Trades</h3>
            <div class="mono" id="tradeStatsVue" role="status" aria-live="polite">-</div>
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
                <tbody id="tradeTableBodyVue"></tbody>
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


