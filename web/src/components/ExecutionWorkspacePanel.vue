<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { type Cell, type Header } from '@tanstack/vue-table'
import {
  useHeadlessMatrixTable,
  type MatrixColumnMeta,
} from '../composables/useHeadlessMatrixTable'
import { useShellMount } from '../composables/useShellMount'
import { useQueuedInputWriter } from '../composables/useQueuedInputWriter'
import { useVirtualTableRows } from '../composables/useVirtualTableRows'
import {
  disposeExecutionChart,
  syncExecutionChart,
  type ExecutionChartSurfaceResult,
} from '../services/executionChartService'
import type { ExecutionChartReview } from '../services/executionService'
import { useDataStore } from '../stores/dataStore'
import { useExecutionStore } from '../stores/executionStore'
import { useWorkflowStore } from '../stores/workflowStore'
import { formatUiTimeFromUnixMs } from '../utils/uiTime'
import ConfirmationDialog from './ConfirmationDialog.vue'
import ContextHelpTip from './ContextHelpTip.vue'
import AppDataTableShell from './ui/AppDataTableShell.vue'
import PriceChartPanel from './visualizations/PriceChartPanel.vue'

const dataStore = useDataStore()
const execStore = useExecutionStore()
const workflowStore = useWorkflowStore()
const {
  mode,
  executionTarget,
  symbol,
  qty,
  paramSource,
  riskStopLossPct,
  riskTakeProfitPct,
  riskMaxOrderNotional,
  riskMaxPositionNotional,
  riskAllowShort,
  riskRequireBracket,
  riskPolicyStatusHint,
  manualFastLen,
  manualSlowLen,
  apiKey,
  apiSecret,
  rememberKeys,
  storedRememberAvailable,
  liveUseDelayed,
  liveStatusHint,
  runnerStatusHint,
  statusHint,
  accountReview,
  brokerReview,
  outputPreviewMeta,
  reconciliationState,
  reconciliationOpenOrderCount,
  reconciliationOptimisticOrderCount,
  positionMeta,
  chartMeta,
  accountMeta,
  reviewMeta,
  orderCount,
  orderRows,
  savedSessions,
  selectedSavedSessionPath,
  savedSessionDetail,
  savedSessionStatusHint,
  outputPreview,
  canTestConnection,
  canRunOnce,
  canRefreshOrders,
  canResetLocalPaper,
  canCancelOpenOrders,
  canFlattenPosition,
  canEmergencyHalt,
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
  liveLifecycleLabel,
  runnerLifecycleLabel,
  recoveryStatusLabel,
  brokerReviewRefreshRequired,
  refreshOrdersBlockedReasons,
  runOnceActionLabel,
  runOnceActionDetail,
  runOnceBlockedReasons,
  startLiveBlockedReasons,
  startRunnerBlockedReasons,
  cancelOpenOrdersBlockedReasons,
  flattenPositionBlockedReasons,
  resetLocalPaperBlockedReasons,
  emergencyHaltBlockedReasons,
  reconciliationAdvisories,
  statusLine,
  updatedAtUnixMs,
} = storeToRefs(execStore)
const { dataId: loadedDataId, datasetLabel: loadedDatasetLabel } = storeToRefs(dataStore)
const { workspace } = storeToRefs(workflowStore)

const { mountReady } = useShellMount('#executionVueShellMount', {
  paneSelector: '#executionShellPane',
  paneDatasetKey: 'vueExecution',
})

type PendingExecutionAction =
  | 'clearKeys'
  | 'runOnce'
  | 'startLive'
  | 'startRunner'
  | 'resetLocalPaper'
  | 'cancelOpenOrders'
  | 'flattenPosition'
  | 'emergencyHalt'

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

interface ReviewDetailItem {
  label: string
  value: string
}

interface RecoveryChecklistItem {
  label: string
  done: boolean
  detail: string
}

interface SafetyMatrixItem {
  label: string
  does: string
  doesNot: string
  scope: string
}

const symbolDraft = ref('')
const qtyDraft = ref('')
const riskSlDraft = ref('')
const riskTpDraft = ref('')
const riskMaxOrderDraft = ref('')
const riskMaxPositionDraft = ref('')
const manualFastDraft = ref('')
const manualSlowDraft = ref('')
const apiKeyDraft = ref('')
const apiSecretDraft = ref('')
const modeDraft = ref('paper')
const targetDraft = ref('dry_run')
const sourceDraft = ref('selected')
const pendingAction = ref<PendingExecutionAction | null>(null)
const pendingActionOpenedAtUnixMs = ref(0)
const confirmationNowUnixMs = ref(Date.now())
const execTvChartElementRef = ref<HTMLElement | null>(null)
const chartSyncToken = ref(0)
const chartSurface = ref<ExecutionChartSurfaceResult>({
  state: 'no_dataset',
  summary: 'Execution chart unavailable: load a workflow dataset first.',
  symbol: '',
  datasetId: '',
  datasetLabel: '-',
  datasetSymbol: '',
  fetchedAtUnixMs: null,
  markerCount: 0,
})

const visible = computed(() => String(workspace.value || '').trim() === 'live')
const currentTarget = computed(() => String(executionTarget.value || targetDraft.value || 'dry_run').trim() || 'dry_run')
const liveModeArmed = computed(() => currentTarget.value === 'alpaca_live')
const brokerArmed = computed(() => currentTarget.value !== 'dry_run')
const LIVE_ARMING_CONFIRMATION_TTL_MS = 30_000
let confirmationTimerId: number | null = null
const confirmationExpiresAtUnixMs = computed(() => {
  if (!pendingActionOpenedAtUnixMs.value) return 0
  if (pendingAction.value === 'startRunner') return pendingActionOpenedAtUnixMs.value + LIVE_ARMING_CONFIRMATION_TTL_MS
  if (pendingAction.value === 'runOnce' && currentTarget.value === 'alpaca_live') {
    return pendingActionOpenedAtUnixMs.value + LIVE_ARMING_CONFIRMATION_TTL_MS
  }
  return 0
})
const confirmationExpired = computed(() => {
  const expiresAt = confirmationExpiresAtUnixMs.value
  return expiresAt > 0 && confirmationNowUnixMs.value >= expiresAt
})
const confirmationCountdownLabel = computed(() => {
  const expiresAt = confirmationExpiresAtUnixMs.value
  if (!expiresAt) return ''
  const remaining = Math.max(0, Math.ceil((expiresAt - confirmationNowUnixMs.value) / 1000))
  return confirmationExpired.value
    ? 'Confirmation expired. Reopen this action after reviewing the current broker and feed state.'
    : `Confirmation expires in ${remaining}s.`
})
const selectedSymbolLabel = computed(() => String(requestedSymbolLabel.value || 'No symbol').trim())
const executionOrderColumns = computed<ExecutionOrderColumn[]>(() => [
  { label: 'Time (UTC)', alignLeft: true },
  { label: 'Side', alignLeft: true },
  { label: 'Status', alignLeft: true },
  { label: 'Qty', alignLeft: false },
  { label: 'Filled', alignLeft: false },
  { label: 'Avg Px', alignLeft: false },
  { label: 'Order ID', alignLeft: true },
])
function orderStatusClass(status: string): string {
  const normalized = String(status || '').trim().toLowerCase()
  if (!normalized || normalized === '—' || normalized === '-') return 'statusUnknown'
  if (normalized === 'partially_filled') return 'statusPartial'
  if (normalized === 'filled') return 'statusFilled'
  if (['rejected', 'canceled', 'expired'].includes(normalized)) return 'statusRejected'
  if (['new', 'accepted', 'pending_new', 'accepted_for_bidding', 'pending_replace', 'held', 'stopped', 'calculated', 'suspended', 'pending_cancel'].includes(normalized)) return 'statusOpen'
  if (normalized.endsWith('*')) return 'statusOptimistic'
  return 'statusUnknown'
}
const executionOrderTableRows = computed<ExecutionOrderTableRow[]>(() =>
  orderRows.value.map((row) => ({
    cells: [
      { text: row.cells[0] || '—', className: 'left mono', title: row.cells[0] || '' },
      { text: row.cells[1] || '—', className: 'left', title: row.cells[1] || '' },
      { text: row.cells[2] || '—', className: `left orderStatusCell ${orderStatusClass(row.cells[2] || '')}`, title: row.cells[2] || '' },
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
const modeBadgeLabel = computed(() => {
  if (currentTarget.value === 'alpaca_live') return 'Alpaca live'
  if (currentTarget.value === 'alpaca_paper') return 'Alpaca paper'
  if (currentTarget.value === 'local_paper') return 'Local paper'
  return 'Dry-run'
})
const feedPostureTone = computed<ExecutionPostureItem['tone']>(() => {
  if (liveLifecycleLabel.value === 'Error') return 'danger'
  if (liveLifecycleLabel.value === 'Disconnected') return 'warn'
  if (liveLifecycleLabel.value === 'Connecting' || liveLifecycleLabel.value === 'Subscribed') return 'info'
  if (!String(feedStateLabel.value || '').trim() || feedStateLabel.value === 'Off') return 'neutral'
  return liveSymbolMatchesRequested.value ? 'success' : 'warn'
})
const runnerPostureTone = computed<ExecutionPostureItem['tone']>(() => {
  if (runnerStateLabel.value === 'Error' || runnerStateLabel.value === 'Active · recent error') return 'danger'
  if (
    runnerStateLabel.value === 'Review required' ||
    runnerStateLabel.value === 'Stopped · review required'
  ) return 'warn'
  if (runnerStateLabel.value === 'Active') return brokerArmed.value ? 'warn' : 'info'
  if (
    runnerStateLabel.value === 'Triggered' ||
    runnerStateLabel.value === 'Queued latest' ||
    runnerStateLabel.value === 'Order submitted'
  ) return brokerArmed.value ? 'warn' : 'info'
  if (runnerStateLabel.value === 'Signal processed') return 'info'
  return 'neutral'
})
const executionPostureItems = computed<ExecutionPostureItem[]>(() => [
  {
    label: 'Broker mode',
    value: brokerModeLabel.value,
    tone: liveModeArmed.value ? 'warn' : 'info',
  },
  {
    label: 'Submission',
    value: submissionModeLabel.value,
    tone: brokerArmed.value ? 'danger' : 'info',
  },
  {
    label: 'Feed',
    value: feedStateLabel.value,
    tone: feedPostureTone.value,
  },
  {
    label: 'Runner',
    value: runnerStateLabel.value,
    tone: runnerPostureTone.value,
  },
  {
    label: 'Key state',
    value: keyStateLabel.value,
    tone: keyStateLabel.value === 'Stored securely'
      ? 'success'
      : keyStateLabel.value === 'Session only'
        ? 'info'
        : keyStateLabel.value === 'Will store on next successful broker action' || keyStateLabel.value === 'Will update stored credentials'
          ? 'warn'
          : 'danger',
  },
])
const brokerReviewStale = computed(() => {
  const review = brokerReview.value
  if (!review?.fetchedAtUnixMs) return false
  return Date.now() - review.fetchedAtUnixMs > review.staleThresholdMs
})
const brokerReviewFreshnessLabel = computed(() => {
  const review = brokerReview.value
  const asOf = review?.fetchedAtUnixMs ? formatUiTimeFromUnixMs(review.fetchedAtUnixMs) : 'Unknown'
  if (brokerReviewRefreshRequired.value) return `Pending manual refresh · ${asOf}`
  return brokerReviewStale.value ? `Stale · ${asOf}` : `Fresh · ${asOf}`
})
const reconciliationLabel = computed(() => {
  switch (reconciliationState.value) {
    case 'disabled':
      return 'Disabled until one symbol is selected.'
    case 'unknown':
      return 'Broker refresh required.'
    case 'optimistic_local_pending':
      return `Waiting for broker confirmation (${reconciliationOptimisticOrderCount.value} optimistic local order${reconciliationOptimisticOrderCount.value === 1 ? '' : 's'}).`
    case 'broker_stale':
      return `Broker snapshot stale (${reconciliationOpenOrderCount.value} open orders).`
    case 'broker_confirmed_open':
      return `Broker-confirmed open (${reconciliationOpenOrderCount.value} open orders).`
    case 'broker_confirmed_flat':
      return 'Broker-confirmed flat.'
    default:
      return 'Broker review unavailable.'
  }
})
const executionOrderTableShellSummary = computed(() => {
  const rowCount = executionOrderTableRows.value.length
  return `${rowCount} order${rowCount === 1 ? '' : 's'} · ${reconciliationLabel.value}`
})
const accountReviewDetails = computed<ReviewDetailItem[]>(() => {
  const review = accountReview.value
  if (review.state === 'not_verified') {
    return [{ label: 'State', value: 'Not verified' }]
  }
  const asOf = review.fetchedAtUnixMs ? formatUiTimeFromUnixMs(review.fetchedAtUnixMs) : 'Unknown'
  const items: ReviewDetailItem[] = [
    { label: 'State', value: review.state === 'broker_confirmed' ? 'Broker confirmed' : 'Unavailable' },
    { label: 'Mode', value: review.mode },
    { label: 'Source', value: review.sourceCommand.replace(/_/g, ' ') },
    { label: 'Account', value: review.accountNumber || review.accountId || '—' },
    { label: 'Status', value: review.status || '—' },
    { label: 'As of', value: asOf },
  ]
  if (review.currency) items.push({ label: 'Currency', value: review.currency })
  if (review.buyingPower) items.push({ label: 'Buying power', value: review.buyingPower })
  if (review.cash) items.push({ label: 'Cash', value: review.cash })
  if (review.portfolioValue) items.push({ label: 'Portfolio', value: review.portfolioValue })
  return items
})
const localPaperAccountDetails = computed<ReviewDetailItem[]>(() => {
  if (currentTarget.value !== 'local_paper') return []
  const review = accountReview.value
  const broker = brokerReview.value
  const asOf = review.fetchedAtUnixMs ? formatUiTimeFromUnixMs(review.fetchedAtUnixMs) : 'Refresh local paper to load state'
  return [
    { label: 'Account', value: review.accountNumber || review.accountId || 'local-paper' },
    { label: 'Status', value: review.status || 'simulated' },
    { label: 'Cash', value: review.cash || '—' },
    { label: 'Buying power', value: review.buyingPower || '—' },
    { label: 'Portfolio', value: review.portfolioValue || '—' },
    { label: 'Position', value: positionMeta.value },
    { label: 'Orders', value: String(broker?.orders.totalCount ?? orderCount.value) },
    { label: 'As of', value: asOf },
  ]
})
const brokerReviewDetails = computed<ReviewDetailItem[]>(() => {
  const review = brokerReview.value
  const advisories = reconciliationAdvisories.value
  if (!review) {
    const items: ReviewDetailItem[] = [
      { label: 'Scope', value: 'No broker snapshot loaded' },
      { label: 'Reconciliation', value: reconciliationLabel.value },
    ]
    if (advisories.length > 0) {
      items.push({
        label: advisories.length === 1 ? 'Advisory' : 'Advisories',
        value: advisories.join(' '),
      })
    }
    return items
  }
  const items: ReviewDetailItem[] = [
    { label: 'Scope', value: `symbol-scoped ${review.symbol || '—'}` },
    { label: 'Reconciliation', value: reconciliationLabel.value },
    { label: 'Freshness', value: brokerReviewFreshnessLabel.value },
    { label: 'Open orders', value: String(review.orders.openCount) },
    { label: 'Review rows', value: String(review.orders.totalCount) },
    { label: 'Position', value: positionMeta.value },
  ]
  if (advisories.length > 0) {
    items.push({
      label: advisories.length === 1 ? 'Advisory' : 'Advisories',
      value: advisories.join(' '),
    })
  }
  return items
})
const chartReviewDetails = computed<ReviewDetailItem[]>(() => {
  const chart = effectiveChartSurface.value
  const asOf = chart.fetchedAtUnixMs ? formatUiTimeFromUnixMs(chart.fetchedAtUnixMs) : 'Unknown'
  return [
    { label: 'State', value: chart.state.replace(/_/g, ' ') },
    { label: 'Dataset', value: chart.datasetLabel || chart.datasetId || loadedDatasetLabel.value || '—' },
    { label: 'Symbol', value: chart.symbol || '—' },
    { label: 'Markers', value: String(chart.markerCount) },
    { label: 'Source', value: brokerReview.value ? 'get live execution snapshot' : 'dataset only' },
    { label: 'As of', value: asOf },
    { label: 'Summary', value: chart.summary || chartMeta.value },
  ]
})
const recoveryChecklistItems = computed<RecoveryChecklistItem[]>(() => {
  const chartState = effectiveChartSurface.value.state
  return [
    {
      label: 'Symbol selected',
      done: requestedSymbolLabel.value !== 'No symbol',
      detail: requestedSymbolLabel.value,
    },
    {
      label: 'Broker review refreshed',
      done: !brokerReviewRefreshRequired.value && reconciliationState.value !== 'optimistic_local_pending',
      detail: reconciliationLabel.value,
    },
    {
      label: 'Chart context valid',
      done: !['no_dataset', 'symbol_mismatch', 'load_error'].includes(chartState),
      detail: chartMeta.value,
    },
    {
      label: 'Runner stopped',
      done: !execStore.runnerActive,
      detail: runnerStateLabel.value,
    },
    {
      label: 'Recovery cleared',
      done: !execStore.recoveryReviewRequired,
      detail: recoveryStatusLabel.value,
    },
  ]
})
const previewMetaText = computed(() => {
  const meta = outputPreviewMeta.value
  const generatedAt = meta.generatedAtUnixMs ? formatUiTimeFromUnixMs(meta.generatedAtUnixMs) : 'Unknown'
  return `Source: ${meta.sourceCommand || 'none'} · Scope: ${meta.scopeLabel || 'unscoped'} · Generated: ${generatedAt}`
})
const savedSessionDetailItems = computed<ReviewDetailItem[]>(() => {
  const detail = savedSessionDetail.value
  if (!detail) {
    return [
      { label: 'State', value: 'No saved live session selected' },
      { label: 'Hint', value: savedSessionStatusHint.value },
    ]
  }
  return [
    { label: 'Session', value: detail.sessionId || '—' },
    { label: 'State', value: detail.sessionState || '—' },
    { label: 'Mode', value: detail.mode },
    { label: 'Symbol', value: detail.symbol || '—' },
    { label: 'Strategy', value: detail.strategyKind || '—' },
    { label: 'Dataset', value: detail.dataId || '—' },
    { label: 'Param source', value: detail.paramSource || '—' },
    { label: 'Created', value: detail.createdAtUnixMs ? formatUiTimeFromUnixMs(detail.createdAtUnixMs) : 'Unknown' },
    { label: 'Finalized', value: detail.finalizedAtUnixMs ? formatUiTimeFromUnixMs(detail.finalizedAtUnixMs) : 'Active / unknown' },
    { label: 'Events', value: String(detail.eventCount) },
    { label: 'Snapshots', value: String(detail.brokerSnapshotCount) },
    { label: 'Source run', value: detail.sourceRunArtifactPath || '—' },
  ]
})
const savedSessionEventLines = computed(() =>
  (savedSessionDetail.value?.events ?? []).map((event) => {
    const when = event.atUnixMs ? formatUiTimeFromUnixMs(event.atUnixMs) : 'Unknown'
    const action = event.action ? ` · action=${event.action}` : ''
    const orderId = event.orderId ? ` · order=${event.orderId}` : ''
    return `${when} · ${event.provenance || '-'} / ${event.status || '-'}${action}${orderId} · ${event.message || '-'}`
  }),
)
const savedSessionSnapshotLines = computed(() =>
  (savedSessionDetail.value?.brokerSnapshots ?? []).map((snapshot) => {
    const when = snapshot.atUnixMs ? formatUiTimeFromUnixMs(snapshot.atUnixMs) : 'Unknown'
    return `${when} · ${snapshot.sourceCommand || '-'} · ${snapshot.mode}/${snapshot.symbol || '-'} · position=${snapshot.positionState || '-'} qty=${snapshot.positionQty ?? '-'} · open_orders=${snapshot.openOrderCount} total_orders=${snapshot.orderCount}`
  }),
)
const savedSessionComparisonItems = computed<ReviewDetailItem[]>(() => {
  const detail = savedSessionDetail.value
  if (!detail) return [{ label: 'State', value: 'No saved session selected for comparison' }]
  const lastSnapshot = detail.brokerSnapshots[detail.brokerSnapshots.length - 1]
  const currentMode = currentTarget.value === 'alpaca_live' ? 'live' : 'paper'
  const items: ReviewDetailItem[] = [
    { label: 'Mode', value: detail.mode === currentMode ? `matches ${currentMode}` : `saved=${detail.mode} current=${currentMode}` },
    { label: 'Symbol', value: detail.symbol === selectedSymbolLabel.value ? `matches ${detail.symbol}` : `saved=${detail.symbol || '—'} current=${selectedSymbolLabel.value}` },
    { label: 'Dataset', value: detail.dataId === loadedDataId.value ? `matches ${detail.dataId}` : `saved=${detail.dataId || '—'} current=${loadedDataId.value || '—'}` },
    { label: 'Current broker review', value: reconciliationLabel.value },
  ]
  if (lastSnapshot) {
    items.push(
      { label: 'Saved last position', value: `${lastSnapshot.positionState || '-'} qty=${lastSnapshot.positionQty ?? '-'}` },
      { label: 'Saved open orders', value: String(lastSnapshot.openOrderCount) },
    )
  }
  return items
})
const safetyModeLabel = computed(() => {
  if (currentTarget.value === 'dry_run') {
    return 'Dry-run is on. Manual execution and auto-runner actions stay in simulation until broker submission is armed.'
  }
  if (currentTarget.value === 'local_paper') {
    return 'Local paper is armed. Manual execution and auto-runner actions fill a simulated local account without broker credentials.'
  }
  return currentTarget.value === 'alpaca_live'
    ? 'Broker submission is armed in live mode. Review symbol, key state, and confirmation prompts before sending any live order.'
    : 'Broker submission is armed in Alpaca paper mode. Manual execution and auto-runner actions can place paper orders once confirmed.'
})
const accountReviewNote = computed(() => {
  if (refreshOrdersBlockedReasons.value.length > 0) {
    return refreshOrdersBlockedReasons.value.join(' ')
  }
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
    currentTarget.value === 'dry_run'
      ? 'Runner: Ready to start in dry-run mode once live candles are active for the selected symbol.'
      : currentTarget.value === 'local_paper'
        ? 'Runner: Ready to start with local paper fills once confirmed.'
        : 'Runner: Ready to start broker-armed once confirmed.',
  ]
})
const safetyMatrixItems = computed<SafetyMatrixItem[]>(() => [
  {
    label: 'Stop runner',
    does: 'Stops the auto-runner loop.',
    doesNot: 'Does not stop the live feed, cancel orders, or flatten exposure.',
    scope: 'Current runner only.',
  },
  {
    label: 'Stop live feed',
    does: 'Stops Polygon live candles and disarms the runner if one is active.',
    doesNot: 'Does not cancel broker orders or flatten positions.',
    scope: 'Current live stream only.',
  },
  {
    label: 'Cancel open orders',
    does: `Cancels broker-open orders for ${selectedSymbolLabel.value}.`,
    doesNot: 'Does not flatten an existing broker position.',
    scope: 'Current symbol only. Runner must already be off.',
  },
  {
    label: 'Flatten position',
    does: `Submits one market order to flatten the broker position for ${selectedSymbolLabel.value}.`,
    doesNot: 'Does not cancel existing open orders or stop the live feed.',
    scope: 'Current symbol only. Runner must already be off.',
  },
  {
    label: 'Emergency halt',
    does: 'Stops the live feed and auto-runner, then requires a fresh broker review before re-arm.',
    doesNot: 'Does not cancel broker orders or flatten positions.',
    scope: 'Current local live-automation session only.',
  },
])
const safetyControlNotes = computed(() => {
  const notes = [
    ...cancelOpenOrdersBlockedReasons.value.map((reason) => `Cancel open orders: ${reason}`),
    ...flattenPositionBlockedReasons.value.map((reason) => `Flatten position: ${reason}`),
    ...resetLocalPaperBlockedReasons.value.map((reason) => `Reset local paper: ${reason}`),
    ...emergencyHaltBlockedReasons.value.map((reason) => `Emergency halt: ${reason}`),
  ]
  if (notes.length > 0) return Array.from(new Set(notes))
  return [
    `Cancel open orders: current-symbol only for ${selectedSymbolLabel.value}. Does not flatten the position.`,
    `Flatten position: current-symbol only for ${selectedSymbolLabel.value}. Cancel open orders first if Alpaca still shows them open.`,
    'Emergency halt: stops the live feed and auto-runner only. Refresh broker review before re-arming automation.',
  ]
})
function formatConfirmationContextLines(lines: Array<string | null | undefined>): string {
  return lines
    .map((line) => String(line || '').trim())
    .filter((line) => line.length > 0)
    .join('\n')
}
function startConfirmationTimer(): void {
  confirmationNowUnixMs.value = Date.now()
  if (confirmationTimerId != null) return
  confirmationTimerId = window.setInterval(() => {
    confirmationNowUnixMs.value = Date.now()
  }, 1000)
}
function stopConfirmationTimer(): void {
  if (confirmationTimerId == null) return
  window.clearInterval(confirmationTimerId)
  confirmationTimerId = null
}
const confirmationTitle = computed(() => {
  if (pendingAction.value === 'clearKeys') return 'Clear stored execution credentials?'
  if (pendingAction.value === 'runOnce') {
    return currentTarget.value === 'dry_run' ? 'Run one dry-run execution?' : `Confirm ${runOnceActionLabel.value.toLowerCase()}?`
  }
  if (pendingAction.value === 'startLive') return 'Start live candle feed?'
  if (pendingAction.value === 'startRunner') return 'Start the auto-runner?'
  if (pendingAction.value === 'resetLocalPaper') return 'Reset local paper broker?'
  if (pendingAction.value === 'cancelOpenOrders') return 'Cancel current-symbol open orders?'
  if (pendingAction.value === 'flattenPosition') return 'Flatten the current broker position?'
  if (pendingAction.value === 'emergencyHalt') return 'Emergency halt live automation?'
  return ''
})
const confirmationBody = computed(() => {
  if (pendingAction.value === 'clearKeys') {
    return 'This removes any typed execution override from the current session and clears stored secure execution credentials plus any legacy remembered local keys.'
  }
  if (pendingAction.value === 'runOnce') {
    return formatConfirmationContextLines([
      `${runOnceActionDetail.value} Review ${requestedSymbolLabel.value}, ${brokerModeLabel.value.toLowerCase()} mode, and ${keyStateLabel.value.toLowerCase()} credentials before continuing.`,
      `Submission: ${submissionModeLabel.value}.`,
      `Broker review: ${reconciliationLabel.value}.`,
      confirmationCountdownLabel.value,
      ...reconciliationAdvisories.value.map((note) => `Advisory: ${note}`),
    ])
  }
  if (pendingAction.value === 'startLive') {
    return formatConfirmationContextLines([
      'The app will begin consuming live candle updates for the current symbol. Review symbol, mode, and delayed-feed settings first.',
      `Feed: ${feedStateLabel.value}.`,
      `Recovery: ${recoveryStatusLabel.value}.`,
    ])
  }
  if (pendingAction.value === 'startRunner') {
    return formatConfirmationContextLines([
      'The auto-runner can issue repeated execution decisions as new bars arrive. Confirm mode, parameter source, and risk settings first.',
      `Feed: ${feedStateLabel.value}.`,
      `Recovery: ${recoveryStatusLabel.value}.`,
      `Broker review: ${reconciliationLabel.value}.`,
      confirmationCountdownLabel.value,
      ...reconciliationAdvisories.value.map((note) => `Advisory: ${note}`),
    ])
  }
  if (pendingAction.value === 'resetLocalPaper') {
    return formatConfirmationContextLines([
      'This resets the local simulated paper account, orders, fills, and positions. It does not touch Alpaca paper or live broker state.',
      `Current local paper review: ${accountMeta.value}.`,
      `Broker review: ${reconciliationLabel.value}.`,
      ...resetLocalPaperBlockedReasons.value.map((note) => `Advisory: ${note}`),
    ])
  }
  if (pendingAction.value === 'cancelOpenOrders') {
    return formatConfirmationContextLines([
      `This cancels currently open broker orders for ${selectedSymbolLabel.value}. It does not flatten any existing broker position, and the auto-runner must already be off.`,
      `Broker review: ${reconciliationLabel.value}.`,
      ...reconciliationAdvisories.value.map((note) => `Advisory: ${note}`),
    ])
  }
  if (pendingAction.value === 'flattenPosition') {
    return formatConfirmationContextLines([
      `This submits a market order to flatten the broker position for ${selectedSymbolLabel.value}. It does not cancel open orders or stop the live feed.`,
      `Broker review: ${reconciliationLabel.value}.`,
      ...reconciliationAdvisories.value.map((note) => `Advisory: ${note}`),
    ])
  }
  if (pendingAction.value === 'emergencyHalt') {
    return formatConfirmationContextLines([
      'Emergency halt stops the live feed and auto-runner, then requires a fresh broker review before re-arming. It does not cancel broker orders or flatten positions.',
      `Feed: ${feedStateLabel.value}.`,
      `Recovery: ${recoveryStatusLabel.value}.`,
      `Broker review: ${reconciliationLabel.value}.`,
    ])
  }
  return ''
})
const confirmationConfirmLabel = computed(() => {
  if (pendingAction.value === 'clearKeys') return 'Clear keys'
  if (pendingAction.value === 'runOnce') return runOnceActionLabel.value
  if (pendingAction.value === 'startLive') return 'Start live feed'
  if (pendingAction.value === 'startRunner') return 'Start runner'
  if (pendingAction.value === 'resetLocalPaper') return 'Reset local paper'
  if (pendingAction.value === 'cancelOpenOrders') return 'Cancel open orders'
  if (pendingAction.value === 'flattenPosition') return 'Flatten position'
  if (pendingAction.value === 'emergencyHalt') return 'Emergency halt'
  return 'Confirm'
})
const canConfirmPendingAction = computed(() => {
  if (pendingAction.value === 'clearKeys') return !!canClearKeys.value
  if (pendingAction.value === 'runOnce') return !!canRunOnce.value && !confirmationExpired.value
  if (pendingAction.value === 'startLive') return !!canStartLive.value
  if (pendingAction.value === 'startRunner') return !!canStartRunner.value && !confirmationExpired.value
  if (pendingAction.value === 'resetLocalPaper') return !!canResetLocalPaper.value
  if (pendingAction.value === 'cancelOpenOrders') return !!canCancelOpenOrders.value
  if (pendingAction.value === 'flattenPosition') return !!canFlattenPosition.value
  if (pendingAction.value === 'emergencyHalt') return !!canEmergencyHalt.value
  return false
})
const chartSurfaceMeta = computed(() => effectiveChartSurface.value.summary || chartMeta.value)

function surfaceFromBrokerReviewChart(chart: ExecutionChartReview): ExecutionChartSurfaceResult {
  return {
    state: chart.state === 'sync_pending' ? 'provenance_suppressed' : chart.state,
    summary: chartMeta.value,
    symbol: chart.symbol || '',
    datasetId: chart.datasetId || '',
    datasetLabel: chart.datasetLabel || chart.datasetId || '-',
    datasetSymbol: chart.datasetSymbol || '',
    fetchedAtUnixMs: chart.fetchedAtUnixMs,
    markerCount: chart.orderCount,
  }
}

function normalizeDatasetToken(raw: unknown): string {
  const value = String(raw ?? '').trim()
  return value === '-' || value === '—' ? '' : value
}

function surfaceFromMissingDataset(symbolValue: string, fetchedAtUnixMs: number | null): ExecutionChartSurfaceResult {
  return {
    state: 'no_dataset',
    summary: 'Execution chart unavailable: load a workflow dataset first.',
    symbol: String(symbolValue || '').trim(),
    datasetId: '',
    datasetLabel: '-',
    datasetSymbol: '',
    fetchedAtUnixMs,
    markerCount: 0,
  }
}

const effectiveChartSurface = computed<ExecutionChartSurfaceResult>(() => {
  const local = chartSurface.value
  const review = brokerReview.value
  const selectedSymbol = String(symbol.value || symbolDraft.value || review?.symbol || '').trim()
  const datasetId = normalizeDatasetToken(loadedDataId.value || review?.chart.datasetId || '')
  if (local.state === 'no_symbol' && !datasetId && selectedSymbol) {
    return surfaceFromMissingDataset(selectedSymbol, review?.fetchedAtUnixMs ?? null)
  }
  if (local.state === 'no_symbol' && review?.chart && review.chart.state !== 'no_symbol') {
    return surfaceFromBrokerReviewChart(review.chart)
  }
  return local
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
    String(riskMaxOrderNotional.value || ''),
    String(riskMaxPositionNotional.value || ''),
    String(manualFastLen.value || ''),
    String(manualSlowLen.value || ''),
    String(apiKey.value || ''),
    String(apiSecret.value || ''),
    String(mode.value || 'paper'),
    String(executionTarget.value || 'dry_run'),
    String(paramSource.value || 'selected'),
  ] as const,
  ([
    nextSymbol,
    nextQty,
    nextRiskStopLossPct,
    nextRiskTakeProfitPct,
    nextRiskMaxOrderNotional,
    nextRiskMaxPositionNotional,
    nextManualFastLen,
    nextManualSlowLen,
    nextApiKey,
    nextApiSecret,
    nextMode,
    nextTarget,
    nextParamSource,
  ]) => {
    if (symbolDraft.value !== nextSymbol) symbolDraft.value = nextSymbol
    if (qtyDraft.value !== nextQty) qtyDraft.value = nextQty
    if (riskSlDraft.value !== nextRiskStopLossPct) riskSlDraft.value = nextRiskStopLossPct
    if (riskTpDraft.value !== nextRiskTakeProfitPct) riskTpDraft.value = nextRiskTakeProfitPct
    if (riskMaxOrderDraft.value !== nextRiskMaxOrderNotional) riskMaxOrderDraft.value = nextRiskMaxOrderNotional
    if (riskMaxPositionDraft.value !== nextRiskMaxPositionNotional) riskMaxPositionDraft.value = nextRiskMaxPositionNotional
    if (manualFastDraft.value !== nextManualFastLen) manualFastDraft.value = nextManualFastLen
    if (manualSlowDraft.value !== nextManualSlowLen) manualSlowDraft.value = nextManualSlowLen
    if (apiKeyDraft.value !== nextApiKey) apiKeyDraft.value = nextApiKey
    if (apiSecretDraft.value !== nextApiSecret) apiSecretDraft.value = nextApiSecret
    if (modeDraft.value !== nextMode) modeDraft.value = nextMode
    if (targetDraft.value !== nextTarget) targetDraft.value = nextTarget
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

function refreshSavedSessionsAction(): void {
  void execStore.refreshSavedSessions()
}

function selectSavedSessionAction(nextPath: string): void {
  void execStore.selectSavedSession(nextPath)
}

function runOnceAction(): void {
  flushQueuedInputValuesNow()
  execStore.runOnce()
}

function previewRiskPolicyAction(): void {
  flushQueuedInputValuesNow()
  execStore.previewRiskPolicy()
}

function saveRiskPolicyAction(): void {
  flushQueuedInputValuesNow()
  void execStore.saveRiskPolicy()
}

function queueRunOnceAction(): void {
  flushQueuedInputValuesNow()
  if (currentTarget.value === 'dry_run') {
    runOnceAction()
    return
  }
  pendingAction.value = 'runOnce'
  pendingActionOpenedAtUnixMs.value = Date.now()
  startConfirmationTimer()
}

function refreshOrdersAction(): void {
  flushQueuedInputValuesNow()
  execStore.refreshOrders()
}

function resetLocalPaperAction(): void {
  flushQueuedInputValuesNow()
  execStore.resetLocalPaper()
}

function cancelOpenOrdersAction(): void {
  flushQueuedInputValuesNow()
  execStore.cancelOpenOrders()
}

function flattenPositionAction(): void {
  flushQueuedInputValuesNow()
  execStore.flattenPosition()
}

function emergencyHaltAction(): void {
  flushQueuedInputValuesNow()
  execStore.emergencyHalt()
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
  pendingActionOpenedAtUnixMs.value = Date.now()
  startConfirmationTimer()
}

function cancelPendingAction(): void {
  pendingAction.value = null
  pendingActionOpenedAtUnixMs.value = 0
  stopConfirmationTimer()
}

function confirmPendingAction(): void {
  const nextAction = pendingAction.value
  if (!nextAction) return
  if (confirmationExpired.value) return
  pendingAction.value = null
  pendingActionOpenedAtUnixMs.value = 0
  stopConfirmationTimer()
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
    return
  }
  if (nextAction === 'resetLocalPaper') {
    resetLocalPaperAction()
    return
  }
  if (nextAction === 'cancelOpenOrders') {
    cancelOpenOrdersAction()
    return
  }
  if (nextAction === 'flattenPosition') {
    flattenPositionAction()
    return
  }
  if (nextAction === 'emergencyHalt') {
    emergencyHaltAction()
  }
}

function setExecutionChartHost(element: HTMLElement | null): void {
  const previous = execTvChartElementRef.value
  execTvChartElementRef.value = element
  if (!element || previous === element || !visible.value) return
  void syncExecutionChartSurface()
}

async function syncExecutionChartSurface(): Promise<void> {
  const token = chartSyncToken.value + 1
  chartSyncToken.value = token
  if (!visible.value || !mountReady.value) {
    disposeExecutionChart()
    return
  }
  await nextTick()
  const review = brokerReview.value
  const selectedSymbol = String(symbol.value || symbolDraft.value || review?.symbol || '').trim()
  const requestedDatasetId = normalizeDatasetToken(loadedDataId.value || review?.chart.datasetId || '')
  const requestedDatasetLabel = normalizeDatasetToken(loadedDatasetLabel.value || review?.chart.datasetLabel || '')
  const brokerReviewLoaded =
    !!review &&
    (!!review.symbol ? String(review.symbol).trim().toUpperCase() === selectedSymbol.trim().toUpperCase() : !!selectedSymbol)
  const result = await syncExecutionChart(execTvChartElementRef.value, {
    dataId: requestedDatasetId,
    datasetLabel: requestedDatasetLabel,
    datasetSymbol: '',
    symbol: selectedSymbol,
    brokerReviewLoaded,
    fetchedAtUnixMs: brokerReviewLoaded ? (review?.fetchedAtUnixMs ?? null) : null,
    staleThresholdMs: review?.staleThresholdMs ?? 90_000,
    orderRows: brokerReviewLoaded ? orderRows.value : [],
  })
  if (chartSyncToken.value !== token) return
  if (result.state === 'no_symbol' && !requestedDatasetId) {
    const fallbackSymbol = String(review?.symbol || selectedSymbol || '').trim()
    if (fallbackSymbol) {
      chartSurface.value = {
        state: 'no_dataset',
        summary: 'Execution chart unavailable: load a workflow dataset first.',
        symbol: fallbackSymbol,
        datasetId: '',
        datasetLabel: '-',
        datasetSymbol: '',
        fetchedAtUnixMs: review?.fetchedAtUnixMs ?? null,
        markerCount: 0,
      }
      return
    }
  }
  if (result.state === 'no_symbol' && review?.chart && review.chart.state !== 'no_symbol') {
    chartSurface.value = surfaceFromBrokerReviewChart(review.chart)
    return
  }
  chartSurface.value = result
}

onBeforeUnmount(() => {
  flushQueuedInputValuesNow()
  pendingAction.value = null
  pendingActionOpenedAtUnixMs.value = 0
  stopConfirmationTimer()
  chartSyncToken.value += 1
  disposeExecutionChart()
})

onMounted(() => {
  void execStore.hydrateStoredCredentialStatus().catch(() => {})
  void execStore.loadRiskPolicy().catch(() => {})
})

watch(visible, (isVisible) => {
  if (!isVisible) cancelPendingAction()
})

watch(
  () => [
    visible.value,
    mountReady.value,
    String(symbol.value || ''),
    String(symbolDraft.value || ''),
    String(brokerReview.value?.symbol || ''),
    brokerReview.value?.fetchedAtUnixMs ?? null,
    orderRows.value.length,
    String(loadedDataId.value || ''),
    String(loadedDatasetLabel.value || ''),
    updatedAtUnixMs.value,
  ] as const,
  () => {
    void syncExecutionChartSurface()
  },
  { immediate: true, flush: 'post' },
)
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
          :data-label="item.label"
          :data-tone="item.tone"
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
            <label for="vueExecTarget">Execution target</label>
            <select id="vueExecTarget" :value="targetDraft" @change="targetDraft = String(($event.target as HTMLSelectElement).value || 'dry_run'); setSelectValue('execExecutionTarget', targetDraft)">
              <option value="dry_run">Dry-run</option>
              <option value="local_paper">Local paper</option>
              <option value="alpaca_paper">Alpaca paper</option>
              <option value="alpaca_live">Alpaca live</option>
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
            <label for="vueExecRiskMaxOrder">Max order notional</label>
            <input
              id="vueExecRiskMaxOrder"
              type="number"
              min="0"
              step="1"
              :value="riskMaxOrderDraft"
              @input="riskMaxOrderDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('execRiskMaxOrderNotional', riskMaxOrderDraft)"
              @change="commitInputValue('execRiskMaxOrderNotional', riskMaxOrderDraft)"
            />
          </div>
          <div>
            <label for="vueExecRiskMaxPosition">Max position notional</label>
            <input
              id="vueExecRiskMaxPosition"
              type="number"
              min="0"
              step="1"
              :value="riskMaxPositionDraft"
              @input="riskMaxPositionDraft = String(($event.target as HTMLInputElement).value || ''); queueInputValue('execRiskMaxPositionNotional', riskMaxPositionDraft)"
              @change="commitInputValue('execRiskMaxPositionNotional', riskMaxPositionDraft)"
            />
          </div>
          <div class="inlineCheck">
            <input
              id="vueExecRiskAllowShort"
              type="checkbox"
              :checked="riskAllowShort"
              @change="setCheckboxValue('execRiskAllowShort', ($event.target as HTMLInputElement).checked)"
            />
            <label for="vueExecRiskAllowShort">Allow short exposure</label>
          </div>
          <div class="inlineCheck">
            <input
              id="vueExecRiskRequireBracket"
              type="checkbox"
              :checked="riskRequireBracket"
              @change="setCheckboxValue('execRiskRequireBracket', ($event.target as HTMLInputElement).checked)"
            />
            <label for="vueExecRiskRequireBracket">Require brackets</label>
          </div>
          <div class="policyControls">
            <button type="button" @click="previewRiskPolicyAction">Preview policy</button>
            <button type="button" @click="saveRiskPolicyAction">Save policy</button>
          </div>
          <div class="policyStatus">{{ riskPolicyStatusHint }}</div>
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
              :checked="rememberKeys"
              :disabled="!storedRememberAvailable"
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

        <section class="safetyMatrix" aria-label="Live execution safety control matrix">
          <div class="safetyMatrixHeading panelMetricKey">Live execution safety control matrix</div>
          <div
            v-for="item in safetyMatrixItems"
            :key="`safety_matrix_${item.label}`"
            class="safetyMatrixCard"
          >
            <div class="safetyMatrixLabel">{{ item.label }}</div>
            <div class="safetyMatrixLine"><span class="panelMetricKey">Does</span><span>{{ item.does }}</span></div>
            <div class="safetyMatrixLine"><span class="panelMetricKey">Does not</span><span>{{ item.doesNot }}</span></div>
            <div class="safetyMatrixLine"><span class="panelMetricKey">Scope</span><span>{{ item.scope }}</span></div>
          </div>
        </section>

        <div class="actionGroups">
          <section class="actionGroup">
            <div class="actionGroupLabel">Account and review</div>
            <div class="actions">
              <button type="button" data-qa-action="live.account.test" :disabled="!canTestConnection" @click="testConnectionAction">Test account</button>
              <button type="button" data-qa-action="live.account.refreshOrders" :disabled="!canRefreshOrders" @click="refreshOrdersAction">Refresh orders</button>
              <button type="button" class="danger-btn" data-qa-action="live.paper.reset" :disabled="!canResetLocalPaper" @click="queueConfirmation('resetLocalPaper')">Reset local paper</button>
            </div>
            <div class="actionHelp panelMuted">{{ accountReviewNote }}</div>
            <div v-if="localPaperAccountDetails.length > 0" class="localPaperAccountPanel" aria-label="Local paper account">
              <div class="reviewDetailTitle">Local paper account</div>
              <dl class="reviewDetailList">
                <template v-for="item in localPaperAccountDetails" :key="`local_paper_account_${item.label}`">
                  <dt>{{ item.label }}</dt>
                  <dd class="mono">{{ item.value }}</dd>
                </template>
              </dl>
            </div>
          </section>

          <section class="actionGroup">
            <div class="actionGroupLabel">Manual execution</div>
            <div class="actions">
              <button type="button" data-qa-action="live.runner.runOnce" :disabled="!canRunOnce" @click="queueRunOnceAction">{{ runOnceActionLabel }}</button>
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
              <button type="button" data-qa-action="live.stream.start" :disabled="!canStartLive" @click="queueConfirmation('startLive')">Start live</button>
              <button type="button" data-qa-action="live.stream.stop" :disabled="!canStopLive" @click="stopLiveAction">Stop live</button>
              <button type="button" data-qa-action="live.runner.start" :disabled="!canStartRunner" @click="queueConfirmation('startRunner')">Start runner</button>
              <button type="button" data-qa-action="live.runner.stop" :disabled="!canStopRunner" @click="stopRunnerAction">Stop runner</button>
            </div>
            <div class="preflightBlock" role="status" aria-live="polite">
              <div class="preflightLabel">Preflight</div>
              <ul class="preflightList">
                <li v-for="note in liveAutomationNotes" :key="`automation_preflight_${note}`">{{ note }}</li>
              </ul>
            </div>
          </section>

          <section class="actionGroup actionGroupWarn">
            <div class="actionGroupLabel">Safety controls</div>
            <div class="actions">
              <button type="button" data-qa-action="live.safety.cancelOpenOrders" :disabled="!canCancelOpenOrders" @click="queueConfirmation('cancelOpenOrders')">Cancel open orders</button>
              <button type="button" data-qa-action="live.safety.flattenPosition" :disabled="!canFlattenPosition" @click="queueConfirmation('flattenPosition')">Flatten position</button>
              <button type="button" class="danger-btn" data-qa-action="live.safety.emergencyHalt" :disabled="!canEmergencyHalt" @click="queueConfirmation('emergencyHalt')">Emergency halt</button>
            </div>
            <div class="actionHelp panelMuted">
              Broker-facing safety controls stay backend-authoritative and current-symbol-only. Emergency halt is local disarm only.
            </div>
            <div class="preflightBlock" role="status" aria-live="polite">
              <div class="preflightLabel">Control notes</div>
              <ul class="preflightList">
                <li v-for="note in safetyControlNotes" :key="`safety_control_${note}`">{{ note }}</li>
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
            <div><span class="panelMetricKey">Live state</span><span class="panelMetricValue mono panelNumeric">{{ liveLifecycleLabel }}</span></div>
            <div><span class="panelMetricKey">Runner</span><span class="panelMetricValue mono panelNumeric">{{ runnerStatusHint }}</span></div>
            <div><span class="panelMetricKey">Runner state</span><span class="panelMetricValue mono panelNumeric">{{ runnerLifecycleLabel }}</span></div>
            <div><span class="panelMetricKey">Execution</span><span class="panelMetricValue mono panelNumeric">{{ statusHint }}</span></div>
            <div><span class="panelMetricKey">Account</span><span class="panelMetricValue mono panelNumeric">{{ accountMeta }}</span></div>
            <div><span class="panelMetricKey">Review</span><span class="panelMetricValue mono panelNumeric">{{ reviewMeta }}</span></div>
            <div><span class="panelMetricKey">Reconciliation</span><span class="panelMetricValue mono panelNumeric">{{ reconciliationLabel }}</span></div>
            <div><span class="panelMetricKey">Recovery</span><span class="panelMetricValue mono panelNumeric">{{ recoveryStatusLabel }}</span></div>
            <div><span class="panelMetricKey">Position</span><span class="panelMetricValue mono panelNumeric">{{ positionMeta }}</span></div>
            <div><span class="panelMetricKey">Chart</span><span class="panelMetricValue mono panelNumeric">{{ chartSurfaceMeta }}</span></div>
            <div><span class="panelMetricKey">App</span><span class="panelMetricValue mono panelNumeric" role="status" aria-live="polite">{{ statusLine }}</span></div>
          </div>
          <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">
            {{ liveStatusHint }} · {{ runnerStatusHint }} · {{ statusHint }}
          </div>
          <div class="recoveryChecklist" aria-label="Recovery checklist">
            <div
              v-for="item in recoveryChecklistItems"
              :key="`recovery_check_${item.label}`"
              class="recoveryChecklistItem"
              :class="{ done: item.done, pending: !item.done }"
            >
              <span class="recoveryChecklistMark" aria-hidden="true">{{ item.done ? 'OK' : '!' }}</span>
              <span>
                <strong>{{ item.label }}</strong>
                <span class="panelMuted">{{ item.detail }}</span>
              </span>
            </div>
          </div>
          <div class="reviewDetailGrid" aria-label="Structured execution review details">
            <div class="reviewDetailCard">
              <div class="reviewDetailTitle">Account review</div>
              <dl class="reviewDetailList">
                <template v-for="item in accountReviewDetails" :key="`account_review_${item.label}`">
                  <dt>{{ item.label }}</dt>
                  <dd class="mono">{{ item.value }}</dd>
                </template>
              </dl>
            </div>
            <div class="reviewDetailCard">
              <div class="reviewDetailTitle">Broker review</div>
              <dl class="reviewDetailList">
                <template v-for="item in brokerReviewDetails" :key="`broker_review_${item.label}`">
                  <dt>{{ item.label }}</dt>
                  <dd class="mono">{{ item.value }}</dd>
                </template>
              </dl>
            </div>
            <div class="reviewDetailCard">
              <div class="reviewDetailTitle">Chart review</div>
              <dl class="reviewDetailList">
                <template v-for="item in chartReviewDetails" :key="`chart_review_${item.label}`">
                  <dt>{{ item.label }}</dt>
                  <dd class="mono">{{ item.value }}</dd>
                </template>
              </dl>
            </div>
          </div>
        </section>

        <section class="executionSubsection" aria-label="Execution chart context">
          <div class="executionSubsectionLabel panelMetricKey">Chart context</div>
          <div class="chartBlock">
            <PriceChartPanel
              chart-id="execTvChartVue"
              meta-id="execTvChartMetaVue"
              title="Execution chart (loaded dataset + broker orders)"
              :meta-text="chartSurfaceMeta"
              advisory="If this chart remains blank, confirm the current dataset matches the selected symbol and refresh broker review before relying on marker overlays."
              frame-aria-label="Execution chart for the loaded dataset with broker order markers."
              host-aria-label="Execution chart"
              @host-ready="setExecutionChartHost"
            />
          </div>
        </section>

        <section class="executionSubsection" aria-label="Broker order activity">
          <div class="executionSubsectionLabel panelMetricKey">Broker orders</div>
          <AppDataTableShell
            title="Broker order activity"
            :summary="executionOrderTableShellSummary"
            :row-count="executionOrderTableRows.length"
            :viewport="false"
            aria-label="Broker order activity table shell"
            empty-title="No broker orders for current symbol."
            empty-body="Refresh broker review after selecting a symbol to populate broker-confirmed order activity."
            keyboard-hint="Broker order rows are virtualized; spacer rows are hidden from assistive technology."
          >
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
          </AppDataTableShell>
        </section>

        <section class="executionSubsection" aria-label="Saved live session evidence">
          <div class="executionSubsectionLabel panelMetricKey">Saved session evidence</div>
          <div class="savedSessionToolbar">
            <button type="button" :disabled="execStore.pending.refreshSavedSessions" @click="refreshSavedSessionsAction">
              Refresh saved sessions
            </button>
            <label class="srOnly" for="savedSessionEvidenceSelect">Saved live session evidence selection</label>
            <select
              id="savedSessionEvidenceSelect"
              aria-label="Saved live session evidence selection"
              :value="selectedSavedSessionPath"
              :disabled="!savedSessions.length || execStore.pending.refreshSavedSessions"
              @change="selectSavedSessionAction(($event.target as HTMLSelectElement).value)"
            >
              <option value="">Select saved session</option>
              <option v-for="session in savedSessions" :key="session.path" :value="session.path">
                {{ session.ts || 'Unknown time' }} · {{ session.mode }}/{{ session.symbol || '-' }} · {{ session.sessionState || '-' }}
              </option>
            </select>
          </div>
          <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ savedSessionStatusHint }}</div>
          <div class="reviewDetailCard">
            <div class="reviewDetailTitle">Saved session detail</div>
            <dl class="reviewDetailList">
              <template v-for="item in savedSessionDetailItems" :key="`saved_session_${item.label}`">
                <dt>{{ item.label }}</dt>
                <dd class="mono">{{ item.value }}</dd>
              </template>
            </dl>
          </div>
          <div class="reviewDetailCard">
            <div class="reviewDetailTitle">Current comparison</div>
            <dl class="reviewDetailList">
              <template v-for="item in savedSessionComparisonItems" :key="`saved_session_compare_${item.label}`">
                <dt>{{ item.label }}</dt>
                <dd class="mono">{{ item.value }}</dd>
              </template>
            </dl>
          </div>
          <div class="savedSessionLists">
            <div class="savedSessionListCard">
              <div class="reviewDetailTitle">Events</div>
              <ul class="savedSessionList mono">
                <li v-if="!savedSessionEventLines.length">No saved session events loaded.</li>
                <li v-for="line in savedSessionEventLines" :key="`saved_session_event_${line}`">{{ line }}</li>
              </ul>
            </div>
            <div class="savedSessionListCard">
              <div class="reviewDetailTitle">Broker snapshots</div>
              <ul class="savedSessionList mono">
                <li v-if="!savedSessionSnapshotLines.length">No saved broker snapshots loaded.</li>
                <li v-for="line in savedSessionSnapshotLines" :key="`saved_session_snapshot_${line}`">{{ line }}</li>
              </ul>
            </div>
          </div>
        </section>

        <details class="panelDetails">
          <summary>Execution output preview</summary>
          <div class="panelDetailsBody">
            <div class="panelMuted">
              Raw broker/runtime output is available here for troubleshooting and operator review without dominating the main execution surface.
            </div>
            <div class="panelStatus mono panelNumeric" role="status" aria-live="polite">{{ previewMetaText }}</div>
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

.inlineCheck {
  display: flex;
  align-items: center;
  gap: 0.42rem;
  min-height: 2.15rem;
}

.policyControls {
  display: flex;
  flex-wrap: wrap;
  align-items: end;
  gap: 0.45rem;
}

.policyControls button {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
}

.policyStatus {
  align-self: end;
  color: var(--muted);
  font-size: 0.76rem;
  line-height: 1.35;
}

.credentialStateCard {
  margin-top: 0.7rem;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-sunken) 92%, var(--panel));
  padding: 0.7rem 0.75rem;
}

.safetyMatrix {
  margin-top: 0.7rem;
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.55rem;
}

.safetyMatrixHeading {
  grid-column: 1 / -1;
}

.safetyMatrixCard {
  border: 1px solid color-mix(in oklab, var(--warning) 24%, var(--border));
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface-warning) 92%, var(--surface));
  padding: 0.7rem 0.75rem;
  display: grid;
  gap: 0.28rem;
}

.safetyMatrixLabel {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
}

.safetyMatrixLine {
  display: grid;
  gap: 0.08rem;
  color: var(--text);
  font-size: 0.76rem;
  line-height: 1.35;
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

.actionGroupWarn {
  border-color: color-mix(in oklab, var(--warning) 34%, var(--border));
}

.actionHelp {
  margin-top: 0.52rem;
  font-size: 0.78rem;
}

.localPaperAccountPanel {
  margin-top: 0.62rem;
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface) 95%, var(--panel));
  padding: 0.58rem 0.62rem;
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

.reviewDetailGrid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.55rem;
}

.recoveryChecklist {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.42rem;
}

.recoveryChecklistItem {
  min-width: 0;
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.38rem;
  align-items: start;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0.42rem 0.48rem;
  font-size: 0.72rem;
  line-height: 1.3;
}

.recoveryChecklistItem strong,
.recoveryChecklistItem span {
  display: block;
  min-width: 0;
}

.recoveryChecklistItem.done {
  border-color: color-mix(in oklab, var(--success) 38%, var(--border));
}

.recoveryChecklistItem.pending {
  border-color: color-mix(in oklab, var(--warning) 44%, var(--border));
}

.recoveryChecklistMark {
  font-weight: 800;
  color: var(--muted);
}

.recoveryChecklistItem.done .recoveryChecklistMark {
  color: var(--success);
}

.recoveryChecklistItem.pending .recoveryChecklistMark {
  color: var(--warning);
}

.reviewDetailCard {
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface) 95%, var(--panel));
  padding: 0.62rem 0.68rem;
}

.reviewDetailTitle {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted);
  margin-bottom: 0.4rem;
}

.reviewDetailList {
  margin: 0;
  display: grid;
  grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
  gap: 0.28rem 0.45rem;
}

.reviewDetailList dt {
  margin: 0;
  color: var(--muted);
  font-size: 0.74rem;
}

.reviewDetailList dd {
  margin: 0;
  color: var(--text);
  font-size: 0.76rem;
}

.chartBlock {
  margin-top: 0.75rem;
  display: grid;
  gap: 0.5rem;
}

.savedSessionToolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
}

.savedSessionToolbar select {
  min-width: 320px;
  max-width: 100%;
}

.savedSessionLists {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.55rem;
}

.savedSessionListCard {
  border: 1px solid color-mix(in oklab, var(--border) 86%, transparent);
  border-radius: 12px;
  background: color-mix(in oklab, var(--surface) 95%, var(--panel));
  padding: 0.62rem 0.68rem;
}

.savedSessionList {
  margin: 0;
  padding-left: 1rem;
  display: grid;
  gap: 0.28rem;
  font-size: 0.76rem;
  line-height: 1.4;
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

.orderStatusCell {
  font-weight: 700;
}

.statusOptimistic,
.statusOpen {
  color: var(--warning);
}

.statusPartial {
  color: var(--accent);
}

.statusFilled {
  color: var(--success);
}

.statusRejected {
  color: var(--danger);
}

.statusUnknown {
  color: var(--muted);
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

  .safetyMatrix {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .recoveryChecklist {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 760px) {
  .executionPostureGrid {
    grid-template-columns: 1fr;
  }

  .safetyMatrix {
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

  .recoveryChecklist {
    grid-template-columns: 1fr;
  }

  .reviewDetailGrid {
    grid-template-columns: 1fr;
  }

  .savedSessionLists {
    grid-template-columns: 1fr;
  }
}
</style>

<style>
#executionShellPane[data-vue-execution='1'] > :not(#executionVueShellMount) {
  display: none !important;
}
</style>



