<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { type Cell, type Header } from '@tanstack/vue-table'
import {
  useHeadlessMatrixTable,
  type MatrixColumnMeta,
} from '../composables/useHeadlessMatrixTable'
import { useShellMount } from '../composables/useShellMount'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useVirtualTableRows } from '../composables/useVirtualTableRows'
import { useExecutionStore } from '../stores/executionStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'
import ConfirmationDialog from './ConfirmationDialog.vue'
import ContextHelpTip from './ContextHelpTip.vue'

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
  brokerModeLabel,
  submissionModeLabel,
  feedStateLabel,
  runnerStateLabel,
  requestedSymbolLabel,
  liveSymbolMatchesRequested,
  hasExecutionCredentials,
  keyStateLabel,
  keyStateDetail,
  runOnceActionLabel,
  runOnceActionDetail,
  runOnceBlockedReasons,
  startLiveBlockedReasons,
  startRunnerBlockedReasons,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(execStore)
const { workspace } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#executionVueShellMount', {
  paneSelector: '#executionShellPane',
  paneDatasetKey: 'vueExecution',
})

type PendingExecutionAction = 'clearKeys' | 'runOnce' | 'startLive' | 'startRunner'

interface ExecutionOrderColumn {
  label: string
  alignLeft: boolean
}

interface ExecutionOrderCell {
  text: string
  className: string
  title: string
}

interface ExecutionOrderTableRow {
  cells: ExecutionOrderCell[]
}

interface ExecutionPostureItem {
  label: string
  value: string
  tone: 'neutral' | 'info' | 'warn' | 'danger' | 'success'
}

const symbolDraft = ref('')
const qtyDraft = ref('')
const riskSlDraft = ref('')
const riskTpDraft = ref('')
const manualFastDraft = ref('')
const manualSlowDraft = ref('')
const apiKeyDraft = ref('')
const apiSecretDraft = ref('')
const modeDraft = ref('paper')
const sourceDraft = ref('selected')
const pendingAction = ref<PendingExecutionAction | null>(null)

const visible = computed(() => String(workspace.value || '').trim() === 'settings')
const liveModeArmed = computed(() => String(mode.value || modeDraft.value || 'paper').trim() === 'live')
const executionOrderColumns = computed<ExecutionOrderColumn[]>(() => [
  { label: 'Time (UTC)', alignLeft: true },
  { label: 'Side', alignLeft: true },
  { label: 'Status', alignLeft: true },
  { label: 'Qty', alignLeft: false },
  { label: 'Filled', alignLeft: false },
  { label: 'Avg Px', alignLeft: false },
  { label: 'Order ID', alignLeft: true },
])
const executionOrderTableRows = computed<ExecutionOrderTableRow[]>(() =>
  orderRows.value.map((row) => ({
    cells: [
      { text: row.cells[0] || '—', className: 'left mono', title: row.cells[0] || '' },
      { text: row.cells[1] || '—', className: 'left', title: row.cells[1] || '' },
      { text: row.cells[2] || '—', className: 'left', title: row.cells[2] || '' },
      { text: row.cells[3] || '—', className: 'mono', title: row.cells[3] || '' },
      { text: row.cells[4] || '—', className: 'mono', title: row.cells[4] || '' },
      { text: row.cells[5] || '—', className: 'mono', title: row.cells[5] || '' },
      { text: row.cells[6] || '—', className: 'left mono', title: row.cells[6] || '' },
    ],
  })),
)
const { headerGroups: orderHeaderGroups, rowModelRows: executionOrderRowModelRows } =
  useHeadlessMatrixTable(executionOrderColumns, executionOrderTableRows)
const {
  scrollElementRef: orderTableScrollElementRef,
  virtualRows: virtualOrderRows,
  topSpacerPx: orderTableTopSpacerPx,
  bottomSpacerPx: orderTableBottomSpacerPx,
} = useVirtualTableRows(executionOrderRowModelRows, { rowHeightPx: 30, overscan: 8 })
const modeBadgeLabel = computed(() => liveModeArmed.value ? 'Live broker' : 'Paper broker')
const executionPostureItems = computed<ExecutionPostureItem[]>(() => [
  {
    label: 'Broker mode',
    value: brokerModeLabel.value,
    tone: liveModeArmed.value ? 'warn' : 'info',
  },
  {
    label: 'Submission',
    value: submissionModeLabel.value,
    tone: dryRun.value ? 'info' : 'danger',
  },
  {
    label: 'Feed',
    value: feedStateLabel.value,
    tone: !String(feedStateLabel.value || '').trim() || feedStateLabel.value === 'Off'
      ? 'neutral'
      : liveSymbolMatchesRequested.value
        ? 'success'
        : 'warn',
  },
  {
    label: 'Runner',
    value: runnerStateLabel.value,
    tone: runnerStateLabel.value === 'Active'
      ? (dryRun.value ? 'info' : 'warn')
      : 'neutral',
  },
  {
    label: 'Key state',
    value: keyStateLabel.value,
    tone: keyStateLabel.value === 'Persisted'
      ? 'success'
      : keyStateLabel.value === 'Session only'
        ? 'info'
        : keyStateLabel.value === 'Will persist next action' || keyStateLabel.value === 'Will update persisted'
          ? 'warn'
          : 'danger',
  },
])
const safetyModeLabel = computed(() => {
  if (dryRun.value) {
    return 'Dry-run is on. Manual execution and auto-runner actions stay in simulation until broker submission is armed.'
  }
  const nextMode = String(mode.value || modeDraft.value || 'paper').trim() || 'paper'
  return nextMode === 'live'
    ? 'Broker submission is armed in live mode. Review symbol, key state, and confirmation prompts before sending any live order.'
    : 'Broker submission is armed in paper mode. Manual execution and auto-runner actions can place paper orders once confirmed.'
})
const accountReviewNote = computed(() => {
  if (!hasExecutionCredentials.value) {
    return 'API key + secret are required before testing the selected broker account.'
  }
  return 'Use account test and order refresh to verify broker connectivity before higher-risk actions.'
})
const manualExecutionNotes = computed(() => {
  if (runOnceBlockedReasons.value.length > 0) return runOnceBlockedReasons.value
  return [runOnceActionDetail.value]
})
const liveAutomationNotes = computed(() => {
  const notes = [
    ...startLiveBlockedReasons.value.map((reason) => `Feed: ${reason}`),
    ...startRunnerBlockedReasons.value.map((reason) => `Runner: ${reason}`),
  ]
  if (notes.length > 0) return Array.from(new Set(notes))
  return [
    `Feed: Ready to start for ${requestedSymbolLabel.value}.`,
    dryRun.value
      ? 'Runner: Ready to start in dry-run mode once live candles are active for the selected symbol.'
      : 'Runner: Ready to start broker-armed once confirmed.',
  ]
})
const confirmationTitle = computed(() => {
  if (pendingAction.value === 'clearKeys') return 'Clear stored execution credentials?'
  if (pendingAction.value === 'runOnce') {
    return dryRun.value ? 'Run one dry-run execution?' : `Confirm ${runOnceActionLabel.value.toLowerCase()}?`
  }
  if (pendingAction.value === 'startLive') return 'Start live candle feed?'
  if (pendingAction.value === 'startRunner') return 'Start the auto-runner?'
  return ''
})
const confirmationBody = computed(() => {
  if (pendingAction.value === 'clearKeys') {
    return 'This removes the current API key and secret from the frontend state and clears persisted execution keys.'
  }
  if (pendingAction.value === 'runOnce') {
    return `${runOnceActionDetail.value} Review ${requestedSymbolLabel.value}, ${brokerModeLabel.value.toLowerCase()} mode, and ${keyStateLabel.value.toLowerCase()} credentials before continuing.`
  }
  if (pendingAction.value === 'startLive') {
    return 'The app will begin consuming live candle updates for the current symbol. Review symbol, mode, and delayed-feed settings first.'
  }
  if (pendingAction.value === 'startRunner') {
    return 'The auto-runner can issue repeated execution decisions as new bars arrive. Confirm mode, parameter source, and risk settings first.'
  }
  return ''
})
const confirmationConfirmLabel = computed(() => {
  if (pendingAction.value === 'clearKeys') return 'Clear keys'
  if (pendingAction.value === 'runOnce') return runOnceActionLabel.value
  if (pendingAction.value === 'startLive') return 'Start live feed'
  if (pendingAction.value === 'startRunner') return 'Start runner'
  return 'Confirm'
})
const canConfirmPendingAction = computed(() => {
  if (pendingAction.value === 'clearKeys') return !!canClearKeys.value
  if (pendingAction.value === 'runOnce') return !!canRunOnce.value
  if (pendingAction.value === 'startLive') return !!canStartLive.value
  if (pendingAction.value === 'startRunner') return !!canStartRunner.value
  return false
})

const stamp = computed(() => formatUiTimeFromUnixMs(updatedAtUnixMs.value))

function headerMeta(header: Header<ExecutionOrderTableRow, unknown>): MatrixColumnMeta {
  return header.column.columnDef.meta as MatrixColumnMeta
}

function executionOrderCellPayload(cell: Cell<ExecutionOrderTableRow, unknown>): ExecutionOrderCell {
  return cell.getValue() as ExecutionOrderCell
}

watch(
  () => [
    String(symbol.value || ''),
    String(qty.value || ''),
    String(riskStopLossPct.value || ''),
    String(riskTakeProfitPct.value || ''),
    String(manualFastLen.value || ''),
    String(manualSlowLen.value || ''),
    String(apiKey.value || ''),
    String(apiSecret.value || ''),
    String(mode.value || 'paper'),
    String(paramSource.value || 'selected'),
  ] as const,
  ([
    nextSymbol,
    nextQty,
    nextRiskStopLossPct,
    nextRiskTakeProfitPct,
    nextManualFastLen,
    nextManualSlowLen,
    nextApiKey,
    nextApiSecret,
    nextMode,
    nextParamSource,
  ]) => {
    if (symbolDraft.value !== nextSymbol) symbolDraft.value = nextSymbol
    if (qtyDraft.value !== nextQty) qtyDraft.value = nextQty
    if (riskSlDraft.value !== nextRiskStopLossPct) riskSlDraft.value = nextRiskStopLossPct
    if (riskTpDraft.value !== nextRiskTakeProfitPct) riskTpDraft.value = nextRiskTakeProfitPct
    if (manualFastDraft.value !== nextManualFastLen) manualFastDraft.value = nextManualFastLen
    if (manualSlowDraft.value !== nextManualSlowLen) manualSlowDraft.value = nextManualSlowLen
    if (apiKeyDraft.value !== nextApiKey) apiKeyDraft.value = nextApiKey
    if (apiSecretDraft.value !== nextApiSecret) apiSecretDraft.value = nextApiSecret
    if (modeDraft.value !== nextMode) modeDraft.value = nextMode
    if (sourceDraft.value !== nextParamSource) sourceDraft.value = nextParamSource
  },
  { immediate: true },
)

function setInputValue(id: string, nextValue: string): void {
  execStore.setInputField(id, nextValue)
}

const { queueInputValue, commitInputValue, flushQueuedInputValuesNow } = useQueuedInputWriter(setInputValue)

function setSelectValue(id: string, nextValue: string): void {
  execStore.setSelectField(id, nextValue)
}

function setCheckboxValue(id: string, checked: boolean): void {
  execStore.setCheckboxField(id, checked)
}

function testConnectionAction(): void {
  flushQueuedInputValuesNow()
  execStore.testConnection()
}

function clearKeysAction(): void {
  flushQueuedInputValuesNow()
  execStore.clearKeys()
}

function runOnceAction(): void {
  flushQueuedInputValuesNow()
  execStore.runOnce()
}

function queueRunOnceAction(): void {
  flushQueuedInputValuesNow()
  if (dryRun.value) {
    runOnceAction()
    return
  }
  pendingAction.value = 'runOnce'
}

function refreshOrdersAction(): void {
  flushQueuedInputValuesNow()
  execStore.refreshOrders()
}

function startLiveAction(): void {
  flushQueuedInputValuesNow()
  execStore.startLive()
}

function stopLiveAction(): void {
  flushQueuedInputValuesNow()
  execStore.stopLive()
}

function startRunnerAction(): void {
  flushQueuedInputValuesNow()
  execStore.startRunner()
}

function stopRunnerAction(): void {
  flushQueuedInputValuesNow()
  execStore.stopRunner()
}

function queueConfirmation(action: PendingExecutionAction): void {
  flushQueuedInputValuesNow()
  pendingAction.value = action
}

function cancelPendingAction(): void {
  pendingAction.value = null
}

function confirmPendingAction(): void {
  const nextAction = pendingAction.value
  if (!nextAction) return
  pendingAction.value = null
  if (nextAction === 'clearKeys') {
    clearKeysAction()
    return
  }
  if (nextAction === 'runOnce') {
    runOnceAction()
    return
  }
  if (nextAction === 'startLive') {
    startLiveAction()
    return
  }
  if (nextAction === 'startRunner') {
    startRunnerAction()
  }
}

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  pendingAction.value = null
})

watch(visible, (isVisible) => {
  if (!isVisible) pendingAction.value = null
})
</script>

<template>
  <Teleport v-if="mountReady" to="#executionVueShellMount">
    <section class="execPanelVueRoot execVueMountHost" v-show="visible">
      <header class="head">
        <div class="panelHeadCopy">
          <div class="panelEyebrow">Broker</div>
          <h2 class="panelTitle">Paper/Live Execution</h2>
          <div class="panelSubtitle">
            Review broker posture, live automation guardrails, and the current order surface before starting any broker-facing workflow.
          </div>
        </div>
        <div class="headMeta">
          <div class="modeBadge" :class="{ live: liveModeArmed }">{{ modeBadgeLabel }}</div>
          <div class="panelMeta mono panelNumeric">{{ orderCount }} orders · {{ stamp }}</div>
        </div>
      </header>
      <div class="executionPostureGrid" aria-label="Execution posture summary">
        <div
          v-for="item in executionPostureItems"
          :key="`execution_posture_${item.label}`"
          class="executionPostureCard"
          :class="item.tone"
        >
          <div class="executionPostureLabel">{{ item.label }}</div>
          <div class="executionPostureValue mono">{{ item.value }}</div>
        </div>
      </div>

      <section class="pane panelProminentPane">
        <div class="paneTopBar">
          <div class="safetyNotice">
            <div class="safetyTitle">Execution safety</div>
            <div class="safetyBody">{{ safetyModeLabel }}</div>
          </div>
          <ContextHelpTip
            title="Execution modes"
            summary="Keep broker posture, one-shot execution, and automation state grouped here; broker-armed actions still require deliberate review."
            detail="Use this surface to review account inputs, source selection, live feed state, runner state, and key posture together. Dry-run one-shot execution stays direct, while broker-armed one-shot execution, live feed start, runner start, and key clearing retain explicit confirmation."
            aria-label="Execution mode help"
          />
        </div>

        <div class="grid">
          <div>
            <label for="vueExecMode">Broker mode</label>
            <select id="vueExecMode" :value="modeDraft" @change="modeDraft = String(($event.target as HTMLSelectElement).value || 'paper'); setSelectValue('execMode', modeDraft)">
              <option value="paper">paper</option>
              <option value="live">live</option>
            </select>
          </div>
          <div>
            <label for="vueExecSymbol">Symbol</label>
            <input
              id="vueExecSymbol"
              type="text"
              :value="symbolDraft"
              placeholder="AAPL"
              @input="symbolDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('execSymbol', symbolDraft)"
              @change="commitInputValue('execSymbol', symbolDraft)"
            />
          </div>
          <div>
            <label for="vueExecQty">Quantity</label>
            <input
              id="vueExecQty"
              type="number"
              min="0.0001"
              step="0.0001"
              :value="qtyDraft"
              @input="qtyDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('execQty', qtyDraft)"
              @change="commitInputValue('execQty', qtyDraft)"
            />
          </div>
          <div>
            <label for="vueExecApiKey">API key</label>
            <input
              id="vueExecApiKey"
              type="text"
              :value="apiKeyDraft"
              autocomplete="off"
              @input="apiKeyDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('execApiKey', apiKeyDraft)"
              @change="commitInputValue('execApiKey', apiKeyDraft)"
            />
          </div>
          <div>
            <label for="vueExecApiSecret">API secret</label>
            <input
              id="vueExecApiSecret"
              type="password"
              :value="apiSecretDraft"
              autocomplete="off"
              @input="apiSecretDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('execApiSecret', apiSecretDraft)"
              @change="commitInputValue('execApiSecret', apiSecretDraft)"
            />
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
            <input
              id="vueExecRiskSl"
              type="number"
              min="0"
              step="0.0001"
              :value="riskSlDraft"
              @input="riskSlDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('execRiskStopLossPct', riskSlDraft)"
              @change="commitInputValue('execRiskStopLossPct', riskSlDraft)"
            />
          </div>
          <div>
            <label for="vueExecRiskTp">Risk TP % (optional)</label>
            <input
              id="vueExecRiskTp"
              type="number"
              min="0"
              step="0.0001"
              :value="riskTpDraft"
              @input="riskTpDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('execRiskTakeProfitPct', riskTpDraft)"
              @change="commitInputValue('execRiskTakeProfitPct', riskTpDraft)"
            />
          </div>
          <div>
            <label for="vueExecManualFast">Manual fast len</label>
            <input
              id="vueExecManualFast"
              type="number"
              min="1"
              step="1"
              :value="manualFastDraft"
              @input="manualFastDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('execManualFastLen', manualFastDraft)"
              @change="commitInputValue('execManualFastLen', manualFastDraft)"
            />
          </div>
          <div>
            <label for="vueExecManualSlow">Manual slow len</label>
            <input
              id="vueExecManualSlow"
              type="number"
              min="1"
              step="1"
              :value="manualSlowDraft"
              @input="manualSlowDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('execManualSlowLen', manualSlowDraft)"
              @change="commitInputValue('execManualSlowLen', manualSlowDraft)"
            />
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
        <div class="credentialStateCard" role="status" aria-live="polite">
          <div class="credentialStateLabel">Credential state</div>
          <div class="credentialStateValue mono">{{ keyStateLabel }}</div>
          <div class="panelMuted">
            {{ keyStateDetail }}
          </div>
        </div>

        <div class="actionGroups">
          <section class="actionGroup">
            <div class="actionGroupLabel">Account and review</div>
            <div class="actions">
              <button type="button" :disabled="!canTestConnection" @click="testConnectionAction">Test account</button>
              <button type="button" :disabled="!canRefreshOrders" @click="refreshOrdersAction">Refresh orders</button>
            </div>
            <div class="actionHelp panelMuted">{{ accountReviewNote }}</div>
          </section>

          <section class="actionGroup">
            <div class="actionGroupLabel">Manual execution</div>
            <div class="actions">
              <button type="button" :disabled="!canRunOnce" @click="queueRunOnceAction">{{ runOnceActionLabel }}</button>
            </div>
            <div class="actionHelp panelMuted">{{ runOnceActionDetail }}</div>
            <div class="preflightBlock" role="status" aria-live="polite">
              <div class="preflightLabel">Preflight</div>
              <ul class="preflightList">
                <li v-for="note in manualExecutionNotes" :key="`manual_preflight_${note}`">{{ note }}</li>
              </ul>
            </div>
          </section>

          <section class="actionGroup">
            <div class="actionGroupLabel">Live automation</div>
            <div class="actions">
              <button type="button" :disabled="!canStartLive" @click="queueConfirmation('startLive')">Start live</button>
              <button type="button" :disabled="!canStopLive" @click="stopLiveAction">Stop live</button>
              <button type="button" :disabled="!canStartRunner" @click="queueConfirmation('startRunner')">Start runner</button>
              <button type="button" :disabled="!canStopRunner" @click="stopRunnerAction">Stop runner</button>
            </div>
            <div class="preflightBlock" role="status" aria-live="polite">
              <div class="preflightLabel">Preflight</div>
              <ul class="preflightList">
                <li v-for="note in liveAutomationNotes" :key="`automation_preflight_${note}`">{{ note }}</li>
              </ul>
            </div>
          </section>

          <section class="actionGroup actionGroupDanger">
            <div class="actionGroupLabel">Destructive</div>
            <div class="actions">
              <button type="button" class="danger-btn" :disabled="!canClearKeys" @click="queueConfirmation('clearKeys')">Clear keys</button>
            </div>
          </section>
        </div>
      </section>

      <section class="pane">
        <div class="panelSectionHeader">
          <div class="panelHeadCopy">
            <div class="panelEyebrow">Monitor</div>
            <h3 class="panelPaneTitle">Execution review surface</h3>
            <div class="panelSectionLead">
              Review runtime state, chart context, and broker orders together before changing live automation state.
            </div>
          </div>
        </div>

        <section class="executionSubsection" aria-label="Execution runtime status">
          <div class="executionSubsectionLabel panelMetricKey">Runtime status</div>
          <div class="statusGrid">
            <div><span class="panelMetricKey">Live</span><span class="panelMetricValue mono panelNumeric">{{ liveStatusHint }}</span></div>
            <div><span class="panelMetricKey">Runner</span><span class="panelMetricValue mono panelNumeric">{{ runnerStatusHint }}</span></div>
            <div><span class="panelMetricKey">Execution</span><span class="panelMetricValue mono panelNumeric">{{ statusHint }}</span></div>
            <div><span class="panelMetricKey">Position</span><span class="panelMetricValue mono panelNumeric">{{ positionMeta }}</span></div>
            <div><span class="panelMetricKey">Chart</span><span class="panelMetricValue mono panelNumeric">{{ chartMeta }}</span></div>
            <div><span class="panelMetricKey">App</span><span class="panelMetricValue mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</span></div>
          </div>
          <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">
            {{ liveStatusHint }} · {{ runnerStatusHint }} · {{ statusHint }}
          </div>
        </section>

        <section class="executionSubsection" aria-label="Execution chart context">
          <div class="executionSubsectionLabel panelMetricKey">Chart context</div>
          <div class="chartBlock">
            <div class="panelMuted">Execution chart (loaded dataset + broker orders)</div>
            <div class="panelAdvisory">
              If this chart remains blank, confirm the symbol has chart data loaded and refresh broker orders to repopulate markers before reviewing the raw preview below.
            </div>
            <div class="tvChartFrame">
              <div id="execTvChartVue" class="tvChartHost" role="img" aria-label="Execution chart"></div>
            </div>
            <div id="execTvChartMetaVue" class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ chartMeta }}</div>
          </div>
        </section>

        <section class="executionSubsection" aria-label="Broker order activity">
          <div class="executionSubsectionLabel panelMetricKey">Broker orders</div>
          <div
            ref="orderTableScrollElementRef"
            class="tableWrap overflowShadowY overflowShadowX surfaceWorking"
            tabindex="0"
            aria-label="Broker order activity table"
          >
            <table class="execOrdersTableVue">
              <caption class="srOnly">
                Broker order activity for the current execution symbol, including side, status, filled quantity, average price, and order identifiers.
              </caption>
              <thead>
                <tr v-for="headerGroup in orderHeaderGroups" :key="headerGroup.id">
                  <th
                    v-for="header in headerGroup.headers"
                    :key="header.id"
                    :class="{ left: headerMeta(header).alignLeft }"
                    scope="col"
                  >
                    {{ headerMeta(header).columnLabel }}
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr v-if="!virtualOrderRows.length">
                  <td class="left panelMuted" colspan="7">No broker orders for current symbol.</td>
                </tr>
                <tr v-if="orderTableTopSpacerPx > 0" class="spacerRow" aria-hidden="true">
                  <td colspan="7" :style="{ height: `${orderTableTopSpacerPx}px` }"></td>
                </tr>
                <tr v-for="virtualRow in virtualOrderRows" :key="virtualRow.row.id">
                  <template v-for="cell in virtualRow.row.getVisibleCells()" :key="cell.id">
                    <td
                      :class="executionOrderCellPayload(cell).className"
                      :title="executionOrderCellPayload(cell).title || undefined"
                    >
                      {{ executionOrderCellPayload(cell).text }}
                    </td>
                  </template>
                </tr>
                <tr v-if="orderTableBottomSpacerPx > 0" class="spacerRow" aria-hidden="true">
                  <td colspan="7" :style="{ height: `${orderTableBottomSpacerPx}px` }"></td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <details class="panelDetails">
          <summary>Execution output preview</summary>
          <div class="panelDetailsBody">
            <div class="panelMuted">
              Raw broker/runtime output is available here for troubleshooting and operator review without dominating the main execution surface.
            </div>
            <pre class="mono output">{{ outputPreview }}</pre>
          </div>
        </details>
      </section>
      <ConfirmationDialog
        id-base="execution_confirmation"
        :open="pendingAction !== null"
        :title="confirmationTitle"
        :body="confirmationBody"
        :confirm-label="confirmationConfirmLabel"
        :confirm-disabled="!canConfirmPendingAction"
        danger
        @cancel="cancelPendingAction"
        @confirm="confirmPendingAction"
      />
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
  align-items: flex-start;
  gap: 0.75rem;
}

.headMeta {
  display: grid;
  justify-items: end;
  gap: 0.35rem;
}

.executionPostureGrid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.55rem;
}

.executionPostureCard {
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface) 94%, var(--panel));
  padding: 0.62rem 0.68rem;
  min-width: 0;
}

.executionPostureCard.info {
  border-color: color-mix(in oklab, var(--accent) 28%, var(--border));
  background: color-mix(in oklab, var(--selection-fill) 78%, var(--surface-inline));
}

.executionPostureCard.success {
  border-color: color-mix(in oklab, var(--ok) 30%, var(--border));
  background: color-mix(in oklab, var(--surface-success) 88%, var(--surface-inline));
}

.executionPostureCard.warn {
  border-color: color-mix(in oklab, var(--warning) 34%, var(--border));
  background: color-mix(in oklab, var(--surface-warning) 90%, var(--surface-inline));
}

.executionPostureCard.danger {
  border-color: color-mix(in oklab, var(--danger) 34%, var(--border));
  background: color-mix(in oklab, var(--surface-danger) 90%, var(--surface-inline));
}

.executionPostureLabel,
.credentialStateLabel,
.preflightLabel {
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.executionPostureValue,
.credentialStateValue {
  margin-top: 0.24rem;
  color: var(--text);
  font-size: 0.82rem;
  line-height: 1.35;
}

.paneTopBar {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
  margin-bottom: 0.7rem;
}

.modeBadge {
  border: 1px solid color-mix(in oklab, var(--border) 84%, transparent);
  border-radius: 999px;
  background: color-mix(in oklab, var(--surface2) 88%, transparent);
  padding: 0.24rem 0.56rem;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.modeBadge.live {
  border-color: color-mix(in oklab, var(--danger-ink) 34%, var(--border));
  background: color-mix(in oklab, var(--surface-danger) 88%, var(--surface));
  color: var(--danger-ink);
}

.pane {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--panel-footer);
  padding: 0.75rem 0.8rem;
}

.safetyNotice {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface) 94%, var(--panel));
  padding: 0.7rem 0.75rem;
}

.safetyTitle,
.actionGroupLabel {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.safetyBody {
  margin-top: 0.3rem;
  color: var(--text);
  font-size: 0.82rem;
  line-height: 1.4;
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

.credentialStateCard {
  margin-top: 0.7rem;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-sunken) 92%, var(--panel));
  padding: 0.7rem 0.75rem;
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

.actionGroups {
  margin-top: 0.75rem;
  display: grid;
  gap: 0.75rem;
}

.actionGroup {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface) 96%, var(--panel));
  padding: 0.7rem 0.75rem;
}

.actionGroupDanger {
  border-color: color-mix(in oklab, var(--danger) 34%, var(--border));
}

.actionHelp {
  margin-top: 0.52rem;
  font-size: 0.78rem;
}

.preflightBlock {
  margin-top: 0.58rem;
  display: grid;
  gap: 0.28rem;
}

.preflightList {
  margin: 0;
  padding-left: 1rem;
  display: grid;
  gap: 0.22rem;
  color: var(--text);
  font-size: 0.78rem;
  line-height: 1.4;
}

.executionSubsection {
  display: grid;
  gap: 0.5rem;
}

.executionSubsection + .executionSubsection {
  margin-top: 0.85rem;
}

.executionSubsectionLabel {
  margin-bottom: 0.05rem;
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

.execOrdersTableVue tbody tr.spacerRow {
  pointer-events: none;
}

.execOrdersTableVue tbody tr.spacerRow td {
  border-bottom: none;
  padding: 0;
}

.output {
  margin-top: 0.55rem;
  max-height: 200px;
  overflow: auto;
}

.mono {
  font-family: var(--mono);
}

@media (max-width: 1180px) {
  .executionPostureGrid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 760px) {
  .executionPostureGrid {
    grid-template-columns: 1fr;
  }

  .grid {
    grid-template-columns: 1fr;
  }

  .headMeta {
    justify-items: start;
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



