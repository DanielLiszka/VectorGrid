<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useAnalysisStore } from '../stores/analysisStore'
import { useWorkflowStore } from '../stores/workflowStore'

const analysisStore = useAnalysisStore()
const workflowStore = useWorkflowStore()
const {
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(analysisStore)
const { resultsTab } = storeToRefs(workflowStore)

const mountReady = ref(false)
const panelRoot = ref<HTMLElement | null>(null)

const visible = computed(() => String(resultsTab.value || '').trim() === 'drill')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

function nudgeDrillRerender(): void {
  analysisStore.rerenderDrill()
}

function resolveMount(): void {
  const panel = document.querySelector('[data-vue-panel="drill"]') as HTMLElement | null
  const mount = document.querySelector('#drillVueShellMount') as HTMLElement | null
  if (!panel || !mount) return
  panelRoot.value = panel
  panel.dataset.vueDrill = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
  window.setTimeout(() => {
    nudgeDrillRerender()
  }, 0)
})

onBeforeUnmount(() => {
  if (panelRoot.value) {
    delete panelRoot.value.dataset.vueDrill
    panelRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#drillVueShellMount">
    <section class="drillPanelVueRoot drillVueShellMountHost" v-show="visible">
      <header class="head">
        <div class="title">Drilldown</div>
        <div class="meta mono">{{ stamp }}</div>
      </header>

      <section class="pane">
        <div id="drillEmptyHintVue" class="muted" style="font-size: 0.8rem;">Click a row in the Top tab to compute drilldown curves.</div>
        <div id="drillWrapVue" style="margin-top: 0.75rem; display: none;">
          <div class="muted" style="margin-bottom: 0.5rem; font-size: 0.8rem;" id="drillMetaVue">-</div>
          <div class="muted" style="margin-bottom: 0.5rem; font-size: 0.8rem;" id="drillStatusVue">Click a Top row to view curves.</div>
          <div>
            <div class="muted" style="margin-bottom: 0.35rem; font-size: 0.8rem;">Price + trades (TradingView Lightweight Charts)</div>
            <div class="tvChartFrame">
              <div id="drillTvChartVue" class="tvChartHost" role="img" aria-label="Backtest price chart"></div>
            </div>
            <div class="muted" style="margin-top: 0.35rem; font-size: 0.8rem;" id="drillTvChartMetaVue">Hover for OHLC/MA values; click a marker to focus a trade.</div>
          </div>
          <div>
            <div class="muted" style="margin-bottom: 0.35rem; font-size: 0.8rem;">Equity (min/max per bin)</div>
            <canvas id="equityCanvasVue" class="chart" width="1024" height="160"></canvas>
          </div>
          <div style="margin-top: 0.75rem;">
            <div class="muted" style="margin-bottom: 0.35rem; font-size: 0.8rem;">Drawdown (min/max per bin)</div>
            <canvas id="drawdownCanvasVue" class="chart" width="1024" height="160"></canvas>
          </div>

          <div id="stabilityWrapVue" style="margin-top: 1.1rem; display: none;">
            <h2>Stability</h2>
            <div class="muted" style="margin-bottom: 0.35rem; font-size: 0.8rem;" id="stabilityStatusVue">-</div>
            <div class="muted" style="margin-bottom: 0.6rem; font-size: 0.8rem;" id="stabilityMetaVue">-</div>
            <div class="muted" style="margin-bottom: 0.6rem; font-size: 0.8rem;" id="rejectMetaVue">-</div>
            <div class="table-wrap" id="stabilityTableWrapVue" style="display: none; margin-top: 0.45rem;">
              <table>
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
            <div class="muted" style="margin-top: 0.5rem; font-size: 0.8rem;">Evaluates a small neighborhood around the selected MA lengths to detect sharp peaks.</div>
          </div>

          <div id="tradeStatsWrapVue" style="margin-top: 1.1rem; display: none;">
            <h2>Trades</h2>
            <div class="mono" id="tradeStatsVue">-</div>
            <div class="table-wrap" style="margin-top: 0.6rem;">
              <table>
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
            <div class="muted" style="margin-top: 0.5rem; font-size: 0.8rem;">Showing up to 200 trades (click a marker to focus).</div>
          </div>
        </div>
      </section>

      <div class="status mono">{{ statusLine }}</div>
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
</style>

<style>
[data-vue-panel='drill'][data-vue-drill='1'] > :not(#drillVueShellMount) {
  display: none !important;
}
</style>


