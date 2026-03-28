<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useExecutionStore } from '../stores/executionStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'

const execution = useExecutionStore()
const workflowState = useWorkflowStore()
const {
  mode,
  symbol,
  qty,
  paramSource,
  apiKey,
  apiSecret,
  dryRun,
  rememberKeys,
  liveUseDelayed,
  liveStatusHint,
  runnerStatusHint,
  statusHint,
  positionMeta,
  chartMeta,
  orderCount,
  outputPreview,
  canTestConnection,
  canRunOnce,
  canRefreshOrders,
  canStartLive,
  canStopLive,
  canStartRunner,
  canStopRunner,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(execution)
const { workspace } = storeToRefs(workflowState)

const visible = computed(() => String(workspace.value || '').trim() === 'settings')

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

function setSelectValue(id: string, nextValue: string): void {
  execution.setSelectField(id, nextValue)
}

function setInputValue(id: string, nextValue: string): void {
  execution.setInputField(id, nextValue)
}

function setCheckboxValue(id: string, checked: boolean): void {
  execution.setCheckboxField(id, checked)
}
</script>

<template>
  <aside v-if="visible" class="execDock">
    <header class="dockHeader">
      <h2 class="title" style="margin: 0;">Execution Context</h2>
      <div class="stamp">{{ stamp }}</div>
    </header>
    <div class="rows">
      <div class="row">
        <span>Mode</span>
        <select
          :value="mode"
          aria-label="Execution mode"
          @change="setSelectValue('execMode', String(($event.target as HTMLSelectElement).value || 'paper'))"
        >
          <option value="paper">paper</option>
          <option value="live">live</option>
        </select>
      </div>
      <div class="row"><span>Symbol</span><input :value="symbol" aria-label="Execution symbol" type="text" @change="setInputValue('execSymbol', String(($event.target as HTMLInputElement).value || ''))" /></div>
      <div class="row"><span>Qty</span><input :value="qty" aria-label="Execution quantity" type="text" @change="setInputValue('execQty', String(($event.target as HTMLInputElement).value || ''))" /></div>
      <div class="row"><span>API key</span><input :value="apiKey" aria-label="Execution API key" type="text" @change="setInputValue('execApiKey', String(($event.target as HTMLInputElement).value || ''))" /></div>
      <div class="row"><span>API secret</span><input :value="apiSecret" aria-label="Execution API secret" type="password" @change="setInputValue('execApiSecret', String(($event.target as HTMLInputElement).value || ''))" /></div>
      <div class="row">
        <span>Source</span>
        <select
          :value="paramSource"
          aria-label="Execution parameter source"
          @change="setSelectValue('execParamSource', String(($event.target as HTMLSelectElement).value || 'selected'))"
        >
          <option value="selected">selected</option>
          <option value="best">best</option>
          <option value="manual">manual</option>
        </select>
      </div>
      <div class="row"><span>Dry run</span><strong class="mono">{{ dryRun ? 'on' : 'off' }}</strong></div>
      <div class="row"><span>Remember</span><strong class="mono">{{ rememberKeys ? 'on' : 'off' }}</strong></div>
      <div class="row"><span>Delayed</span><strong class="mono">{{ liveUseDelayed ? 'on' : 'off' }}</strong></div>
      <div class="row"><span>Orders</span><strong class="mono">{{ orderCount }}</strong></div>
      <div class="row"><span>Live</span><strong class="mono">{{ liveStatusHint }}</strong></div>
      <div class="row"><span>Runner</span><strong class="mono">{{ runnerStatusHint }}</strong></div>
      <div class="row"><span>Status</span><strong class="mono">{{ statusHint }}</strong></div>
      <div class="row"><span>Pos</span><strong class="mono">{{ positionMeta }}</strong></div>
      <div class="row"><span>Chart</span><strong class="mono">{{ chartMeta }}</strong></div>
      <div class="row"><span>Output</span><strong class="mono">{{ outputPreview }}</strong></div>
      <div class="row"><span>App</span><strong class="mono">{{ statusLine }}</strong></div>
    </div>
    <div class="actions">
      <button type="button" @click="setCheckboxValue('execDryRun', !dryRun)">Toggle dry run</button>
      <button type="button" @click="setCheckboxValue('execRememberKeys', !rememberKeys)">Toggle remember</button>
      <button type="button" @click="setCheckboxValue('execLiveUseDelayed', !liveUseDelayed)">Toggle delayed</button>
      <button type="button" :disabled="!canTestConnection" @click="execution.testConnection()">Test account</button>
      <button type="button" :disabled="!canRunOnce" @click="execution.runOnce()">Execute once</button>
      <button type="button" :disabled="!canRefreshOrders" @click="execution.refreshOrders()">Refresh orders</button>
      <button type="button" :disabled="!canStartLive" @click="execution.startLive()">Start live</button>
      <button type="button" :disabled="!canStopLive" @click="execution.stopLive()">Stop live</button>
      <button type="button" :disabled="!canStartRunner" @click="execution.startRunner()">Start runner</button>
      <button type="button" :disabled="!canStopRunner" @click="execution.stopRunner()">Stop runner</button>
      <button type="button" @click="execution.clearKeys()">Clear keys</button>
    </div>
  </aside>
</template>

<style scoped>
.execDock {
  position: fixed;
  right: 10px;
  top: 134px;
  z-index: 4037;
  width: min(460px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid rgba(34, 197, 94, 0.42);
  background: linear-gradient(158deg, rgba(15, 23, 42, 0.94), rgba(20, 83, 45, 0.9));
  color: #e2e8f0;
  box-shadow: 0 18px 40px rgba(2, 6, 23, 0.45);
  padding: 10px;
  display: grid;
  gap: 8px;
  pointer-events: auto;
  max-height: calc(100vh - 144px);
  overflow: auto;
}

.dockHeader {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.title {
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #86efac;
}

.stamp {
  font-size: 0.72rem;
  color: #bbf7d0;
}

.rows {
  display: grid;
  gap: 4px;
}

.row {
  display: grid;
  grid-template-columns: 66px 1fr;
  gap: 6px;
  align-items: baseline;
}

.row span {
  font-size: 0.72rem;
  color: #4ade80;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.row strong {
  font-size: 0.78rem;
  color: #e2e8f0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row input,
.row select {
  width: 100%;
  border: 1px solid rgba(34, 197, 94, 0.38);
  border-radius: 8px;
  padding: 3px 6px;
  font-size: 0.75rem;
  background: rgba(20, 83, 45, 0.82);
  color: #dcfce7;
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.actions button {
  border: 1px solid rgba(74, 222, 128, 0.4);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: rgba(20, 83, 45, 0.82);
  color: #dcfce7;
}

.actions button:disabled {
  opacity: 0.55;
}

@media (max-width: 760px) {
  .execDock {
    left: 8px;
    right: 8px;
    width: auto;
    top: auto;
    bottom: 8px;
    max-height: 62vh;
  }
}

@media (max-width: 1180px) {
  .execDock {
    display: none;
  }
}
</style>

