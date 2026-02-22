<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useExecutionStore } from '../stores/executionStore'
import { useWorkflowStore } from '../stores/workflowStore'

const execStore = useExecutionStore()
const workflowStore = useWorkflowStore()
const {
  mode,
  symbol,
  qty,
  paramSource,
  riskStopLossPct,
  riskTakeProfitPct,
  manualFastLen,
  manualSlowLen,
  dryRun,
  rememberKeys,
  liveUseDelayed,
  liveStatusHint,
  runnerStatusHint,
  statusHint,
  positionMeta,
  chartMeta,
  orderCount,
  orderRows,
  outputPreview,
  canTestConnection,
  canRunOnce,
  canRefreshOrders,
  canStartLive,
  canStopLive,
  canStartRunner,
  canStopRunner,
  canClearKeys,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(execStore)
const { workspace } = storeToRefs(workflowStore)

const mountReady = ref(false)
const paneRoot = ref<HTMLElement | null>(null)

const symbolDraft = ref('')
const qtyDraft = ref('')
const riskSlDraft = ref('')
const riskTpDraft = ref('')
const manualFastDraft = ref('')
const manualSlowDraft = ref('')
const modeDraft = ref('paper')
const sourceDraft = ref('selected')

const visible = computed(() => String(workspace.value || '').trim() === 'settings')

const stamp = computed(() => {
  const ms = Number(updatedAtUnixMs.value || 0)
  if (!Number.isFinite(ms) || ms <= 0) return '—'
  return new Date(ms).toLocaleTimeString([], { hour12: false })
})

watch(symbol, (v) => { const next = String(v || ''); if (symbolDraft.value !== next) symbolDraft.value = next }, { immediate: true })
watch(qty, (v) => { const next = String(v || ''); if (qtyDraft.value !== next) qtyDraft.value = next }, { immediate: true })
watch(riskStopLossPct, (v) => { const next = String(v || ''); if (riskSlDraft.value !== next) riskSlDraft.value = next }, { immediate: true })
watch(riskTakeProfitPct, (v) => { const next = String(v || ''); if (riskTpDraft.value !== next) riskTpDraft.value = next }, { immediate: true })
watch(manualFastLen, (v) => { const next = String(v || ''); if (manualFastDraft.value !== next) manualFastDraft.value = next }, { immediate: true })
watch(manualSlowLen, (v) => { const next = String(v || ''); if (manualSlowDraft.value !== next) manualSlowDraft.value = next }, { immediate: true })
watch(mode, (v) => { const next = String(v || 'paper'); if (modeDraft.value !== next) modeDraft.value = next }, { immediate: true })
watch(paramSource, (v) => { const next = String(v || 'selected'); if (sourceDraft.value !== next) sourceDraft.value = next }, { immediate: true })

function setInputValue(id: string, nextValue: string): void {
  execStore.setInputField(id, nextValue)
}

function setSelectValue(id: string, nextValue: string): void {
  execStore.setSelectField(id, nextValue)
}

function setCheckboxValue(id: string, checked: boolean): void {
  execStore.setCheckboxField(id, checked)
}

function resolveMount(): void {
  const pane = document.getElementById('executionShellPane')
  const mount = document.getElementById('executionVueShellMount')
  if (!pane || !mount) return
  paneRoot.value = pane
  pane.dataset.vueExecution = '1'
  mountReady.value = true
}

onMounted(async () => {
  await nextTick()
  resolveMount()
})

onBeforeUnmount(() => {
  if (paneRoot.value) {
    delete paneRoot.value.dataset.vueExecution
    paneRoot.value = null
  }
})
</script>

<template>
  <Teleport v-if="mountReady" to="#executionVueShellMount">
    <section class="execPanelVueRoot execVueMountHost" v-show="visible">
      <header class="head">
        <div class="title">Paper/Live Execution</div>
        <div class="meta mono">{{ orderCount }} orders · {{ stamp }}</div>
      </header>

      <section class="pane">
        <div class="grid">
          <div>
            <label for="vueExecMode">Mode</label>
            <select id="vueExecMode" :value="modeDraft" @change="modeDraft = String(($event.target as HTMLSelectElement).value || 'paper'); setSelectValue('execMode', modeDraft)">
              <option value="paper">paper</option>
              <option value="live">live</option>
            </select>
          </div>
          <div>
            <label for="vueExecSymbol">Symbol</label>
            <input id="vueExecSymbol" type="text" :value="symbolDraft" placeholder="AAPL" @input="symbolDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('execSymbol', symbolDraft)" />
          </div>
          <div>
            <label for="vueExecQty">Quantity</label>
            <input id="vueExecQty" type="number" min="0.0001" step="0.0001" :value="qtyDraft" @input="qtyDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('execQty', qtyDraft)" />
          </div>
          <div>
            <label for="vueExecSource">Parameter source</label>
            <select id="vueExecSource" :value="sourceDraft" @change="sourceDraft = String(($event.target as HTMLSelectElement).value || 'selected'); setSelectValue('execParamSource', sourceDraft)">
              <option value="selected">selected</option>
              <option value="best">best</option>
              <option value="manual">manual</option>
            </select>
          </div>
          <div>
            <label for="vueExecRiskSl">Risk SL % (optional)</label>
            <input id="vueExecRiskSl" type="number" min="0" step="0.0001" :value="riskSlDraft" @input="riskSlDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('execRiskStopLossPct', riskSlDraft)" />
          </div>
          <div>
            <label for="vueExecRiskTp">Risk TP % (optional)</label>
            <input id="vueExecRiskTp" type="number" min="0" step="0.0001" :value="riskTpDraft" @input="riskTpDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('execRiskTakeProfitPct', riskTpDraft)" />
          </div>
          <div>
            <label for="vueExecManualFast">Manual fast len</label>
            <input id="vueExecManualFast" type="number" min="1" step="1" :value="manualFastDraft" @input="manualFastDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('execManualFastLen', manualFastDraft)" />
          </div>
          <div>
            <label for="vueExecManualSlow">Manual slow len</label>
            <input id="vueExecManualSlow" type="number" min="1" step="1" :value="manualSlowDraft" @input="manualSlowDraft = String(($event.target as HTMLInputElement).value || ''); setInputValue('execManualSlowLen', manualSlowDraft)" />
          </div>
        </div>

        <div class="checks">
          <label>
            <input
              type="checkbox"
              :checked="dryRun"
              @change="setCheckboxValue('execDryRun', ($event.target as HTMLInputElement).checked)"
            />
            Dry-run
          </label>
          <label>
            <input
              type="checkbox"
              :checked="rememberKeys"
              @change="setCheckboxValue('execRememberKeys', ($event.target as HTMLInputElement).checked)"
            />
            Remember keys
          </label>
          <label>
            <input
              type="checkbox"
              :checked="liveUseDelayed"
              @change="setCheckboxValue('execLiveUseDelayed', ($event.target as HTMLInputElement).checked)"
            />
            Use delayed feed
          </label>
        </div>

        <div class="actions">
          <button type="button" :disabled="!canTestConnection" @click="execStore.testConnection()">Test account</button>
          <button type="button" :disabled="!canClearKeys" @click="execStore.clearKeys()">Clear keys</button>
          <button type="button" :disabled="!canRunOnce" @click="execStore.runOnce()">Execute once</button>
          <button type="button" :disabled="!canRefreshOrders" @click="execStore.refreshOrders()">Refresh orders</button>
          <button type="button" :disabled="!canStartLive" @click="execStore.startLive()">Start live</button>
          <button type="button" :disabled="!canStopLive" @click="execStore.stopLive()">Stop live</button>
          <button type="button" :disabled="!canStartRunner" @click="execStore.startRunner()">Start runner</button>
          <button type="button" :disabled="!canStopRunner" @click="execStore.stopRunner()">Stop runner</button>
        </div>
      </section>

      <section class="pane">
        <div class="statusGrid">
          <div><span class="k">Live</span><span class="v mono">{{ liveStatusHint }}</span></div>
          <div><span class="k">Runner</span><span class="v mono">{{ runnerStatusHint }}</span></div>
          <div><span class="k">Execution</span><span class="v mono">{{ statusHint }}</span></div>
          <div><span class="k">Position</span><span class="v mono">{{ positionMeta }}</span></div>
          <div><span class="k">Chart</span><span class="v mono">{{ chartMeta }}</span></div>
          <div><span class="k">App</span><span class="v mono">{{ statusLine }}</span></div>
        </div>

        <div class="chartBlock">
          <div class="muted">Execution chart (loaded dataset + broker orders)</div>
          <div class="tvChartFrame">
            <div id="execTvChartVue" class="tvChartHost" role="img" aria-label="Execution chart"></div>
          </div>
          <div id="execTvChartMetaVue" class="muted mono">{{ chartMeta }}</div>
        </div>

        <div class="tableWrap">
          <table class="execOrdersTableVue">
            <thead>
              <tr>
                <th class="left">Time (UTC)</th>
                <th class="left">Side</th>
                <th class="left">Status</th>
                <th>Qty</th>
                <th>Filled</th>
                <th>Avg Px</th>
                <th class="left">Order ID</th>
              </tr>
            </thead>
            <tbody>
              <tr v-if="!orderRows.length">
                <td class="left muted" colspan="7">No broker orders for current symbol.</td>
              </tr>
              <tr v-for="(row, rowIndex) in orderRows" :key="`exec_order_${rowIndex}_${row.cells[6] || row.cells[0]}`">
                <td class="left mono">{{ row.cells[0] || '—' }}</td>
                <td class="left">{{ row.cells[1] || '—' }}</td>
                <td class="left">{{ row.cells[2] || '—' }}</td>
                <td class="mono">{{ row.cells[3] || '—' }}</td>
                <td class="mono">{{ row.cells[4] || '—' }}</td>
                <td class="mono">{{ row.cells[5] || '—' }}</td>
                <td class="left mono" :title="row.cells[6] || undefined">{{ row.cells[6] || '—' }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <pre class="mono output">{{ outputPreview }}</pre>
      </section>
    </section>
  </Teleport>
</template>

<style scoped>
.execPanelVueRoot {
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
}

.grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.7rem;
}

.checks {
  margin-top: 0.65rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.85rem;
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

.statusGrid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.4rem 0.75rem;
}

.chartBlock {
  margin-top: 0.75rem;
  display: grid;
  gap: 0.5rem;
}

.tableWrap {
  margin-top: 0.75rem;
  max-height: 220px;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 10px;
}

.execOrdersTableVue {
  width: 100%;
  border-collapse: collapse;
  min-width: 780px;
}

.execOrdersTableVue th,
.execOrdersTableVue td {
  border-bottom: 1px solid var(--border);
  padding: 0.35rem 0.44rem;
  text-align: right;
  vertical-align: top;
}

.execOrdersTableVue th.left,
.execOrdersTableVue td.left {
  text-align: left;
}

.k {
  display: block;
  font-size: 0.68rem;
  letter-spacing: 0.03em;
  text-transform: uppercase;
  color: var(--muted);
}

.v {
  display: block;
  margin-top: 0.1rem;
  font-size: 0.78rem;
  line-height: 1.15rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.output {
  margin-top: 0.55rem;
  max-height: 200px;
  overflow: auto;
}

.mono {
  font-family: var(--mono);
}

.muted {
  color: var(--muted);
  font-size: 0.8rem;
}

@media (max-width: 1180px) {
  .grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 760px) {
  .grid {
    grid-template-columns: 1fr;
  }

  .statusGrid {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
#executionShellPane[data-vue-execution='1'] > :not(#executionVueShellMount) {
  display: none !important;
}
</style>


