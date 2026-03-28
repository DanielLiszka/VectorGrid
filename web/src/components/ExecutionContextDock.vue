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
  --dock-accent: var(--ok);
  --dock-accent-ink: var(--ok-ink);
  --dock-accent-soft: var(--ok-soft);
  --dock-surface: linear-gradient(
    158deg,
    color-mix(in oklab, var(--surface-focus) 90%, var(--panel-footer)),
    color-mix(in oklab, var(--ok-soft) 34%, var(--surface-sticky))
  );
  --dock-border: color-mix(in oklab, var(--dock-accent) 40%, var(--border));
  --dock-field-border: color-mix(in oklab, var(--dock-accent) 34%, var(--border));
  --dock-field-surface: color-mix(in oklab, var(--dock-accent-soft) 48%, var(--surface-inline));
  --dock-text: var(--text);
  --dock-title: color-mix(in oklab, var(--dock-accent) 74%, var(--dock-text));
  --dock-stamp: color-mix(in oklab, var(--dock-accent-ink) 76%, var(--dock-text));
  --dock-label: color-mix(in oklab, var(--dock-accent) 62%, var(--dock-text));
  --dock-action-text: color-mix(in oklab, var(--dock-accent-ink) 82%, var(--dock-text));
  position: fixed;
  right: 10px;
  top: 134px;
  z-index: 4037;
  width: min(460px, calc(100vw - 20px));
  border-radius: 12px;
  border: 1px solid var(--dock-border);
  background: var(--dock-surface);
  color: var(--dock-text);
  box-shadow: var(--shadow-focus);
  backdrop-filter: blur(12px);
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
  color: var(--dock-title);
}

.stamp {
  font-size: 0.72rem;
  color: var(--dock-stamp);
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
  color: var(--dock-label);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.row strong {
  font-size: 0.78rem;
  color: var(--dock-text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row input,
.row select {
  width: 100%;
  border: 1px solid var(--dock-field-border);
  border-radius: 8px;
  padding: 3px 6px;
  font-size: 0.75rem;
  background: var(--dock-field-surface);
  color: var(--dock-action-text);
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
  border: 1px solid var(--dock-field-border);
  border-radius: 8px;
  padding: 4px 8px;
  font-size: 0.72rem;
  background: var(--dock-field-surface);
  color: var(--dock-action-text);
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

