import { invokeCommand } from "../tauri/invoke";
import { listenEvent } from "../tauri/events";
import { useDataStore } from "../stores/dataStore";
import { resolveRuntimePinia } from "../stores/runtimePinia";
import { readLatestWorkflowRunSnapshot } from "../stores/workflowArtifactsStore";
import { EXEC_KEYS_STORAGE_KEY } from "../utils/core/storageKeys";
import { formatUtcDateTimeFromUnixMs } from "../utils/uiFormat";
import { readStorageString, removeStorageKey } from "../utils/storage";
import { storeErrorText } from "../utils/storeErrorText";
import {
  isBoundaryObject,
  parseBoundaryArray,
  parseBoundaryObject,
} from "./serviceBoundarySchemas";

let analysisServicePromise: Promise<typeof import("./analysisService")> | null =
  null;
let optimizeServicePromise: Promise<typeof import("./optimizeService")> | null =
  null;

function loadAnalysisService(): Promise<typeof import("./analysisService")> {
  if (!analysisServicePromise)
    analysisServicePromise = import("./analysisService");
  return analysisServicePromise;
}

function loadOptimizeService(): Promise<typeof import("./optimizeService")> {
  if (!optimizeServicePromise)
    optimizeServicePromise = import("./optimizeService");
  return optimizeServicePromise;
}

interface ExecutionStoredKeysPayload {
  api_key?: unknown;
  api_secret?: unknown;
}

interface ExecutionStoredCredentialStatusResponse {
  remember_available?: unknown;
  has_api_key?: unknown;
  has_api_secret?: unknown;
  has_credentials?: unknown;
  state?: unknown;
  storage_backend?: unknown;
  message?: unknown;
}

interface ExecutionRiskPayload {
  stop_loss_pct: number | null;
  take_profit_pct: number | null;
  max_order_notional: number | null;
  max_position_notional: number | null;
  max_daily_loss: number | null;
  max_daily_orders: number | null;
  allow_short: boolean | null;
  require_bracket: boolean | null;
}

export type ExecutionTarget =
  | "dry_run"
  | "local_paper"
  | "alpaca_paper"
  | "alpaca_live";

interface ExecutionActionOptions {
  mode: string;
  executionTarget?: string;
  symbol: string;
  qty: string | number;
  paramSource: string;
  riskStopLossPct: string;
  riskTakeProfitPct: string;
  riskMaxOrderNotional?: string;
  riskMaxPositionNotional?: string;
  riskMaxDailyLoss?: string;
  riskMaxDailyOrders?: string;
  riskAllowShort?: boolean | null;
  riskRequireBracket?: boolean | null;
  manualFastLen: string;
  manualSlowLen: string;
  dryRun: boolean;
  rememberKeys: boolean;
  apiKey: string;
  apiSecret: string;
  liveUseDelayed: boolean;
}

interface ExecutionRunnerStatusResponse {
  active?: unknown;
  inflight?: unknown;
  started_at_unix_ms?: unknown;
  last_bar_open_unix_ms?: unknown;
  last_triggered_at_unix_ms?: unknown;
  last_completed_at_unix_ms?: unknown;
  run_count?: unknown;
  skip_count?: unknown;
  error_count?: unknown;
  last_action?: unknown;
  last_order_id?: unknown;
  last_error?: unknown;
  config?: unknown;
}

interface ExecutionRunnerConfigResponse {
  symbol?: unknown;
  mode?: unknown;
  dry_run?: unknown;
  execution_target?: unknown;
}

interface ExecutionConnectionResponse {
  account_number?: unknown;
  account_id?: unknown;
  status?: unknown;
  currency?: unknown;
  buying_power?: unknown;
  cash?: unknown;
  portfolio_value?: unknown;
  fetched_at_unix_ms?: unknown;
}

export interface ExecutionRiskPolicy {
  max_order_notional: number | null;
  max_position_notional: number | null;
  max_daily_loss: number | null;
  max_daily_orders: number | null;
  allow_short: boolean | null;
  require_bracket: boolean | null;
}

interface ExecutionRiskPolicyResponse {
  policy?: unknown;
}

interface ExecutionSnapshotOrderResponse {
  id?: unknown;
  client_order_id?: unknown;
  parent_order_id?: unknown;
  bracket_group_id?: unknown;
  bracket_leg?: unknown;
  tie_break?: unknown;
  order_type?: unknown;
  type?: unknown;
  side?: unknown;
  status?: unknown;
  qty?: unknown;
  filled_qty?: unknown;
  filled_avg_price?: unknown;
  limit_price?: unknown;
  stop_price?: unknown;
  submitted_at?: unknown;
  filled_at?: unknown;
  updated_at?: unknown;
  canceled_at?: unknown;
  created_at?: unknown;
}

interface ExecutionSnapshotPositionResponse {
  qty?: unknown;
  side?: unknown;
  avg_entry_price?: unknown;
  current_price?: unknown;
  market_value?: unknown;
  unrealized_pl?: unknown;
  unrealized_plpc?: unknown;
}

interface ExecutionSnapshotResponse {
  orders?: unknown;
  symbol?: unknown;
  position?: unknown;
  account_id?: unknown;
  account_number?: unknown;
  account_status?: unknown;
  currency?: unknown;
  buying_power?: unknown;
  cash?: unknown;
  portfolio_value?: unknown;
  fetched_at_unix_ms?: unknown;
}

interface CancelLiveExecutionOpenOrdersResponse {
  mode?: unknown;
  symbol?: unknown;
  open_order_count_before?: unknown;
  cancelled_count?: unknown;
  cancelled_order_ids?: unknown;
}

interface ResetLocalPaperBrokerResponse {
  snapshot?: unknown;
}

interface FlattenLiveExecutionPositionResponse {
  mode?: unknown;
  symbol?: unknown;
  position_found?: unknown;
  open_order_count_before?: unknown;
  order_submitted?: unknown;
  order_id?: unknown;
  action?: unknown;
  qty?: unknown;
}

interface EmergencyHaltLiveExecutionResponse {
  runner_stopped?: unknown;
  live_stopped?: unknown;
  live_symbol?: unknown;
  requires_manual_review?: unknown;
  status_message?: unknown;
}

interface LiveCandleStreamInfoResponse {
  symbol?: unknown;
  delayed?: unknown;
  market?: unknown;
  active?: unknown;
  status?: unknown;
  last_message?: unknown;
  last_status_at_unix_ms?: unknown;
  last_accepted_bar_open_unix_ms?: unknown;
  last_accepted_bar_close_unix_ms?: unknown;
  last_accepted_bar_received_at_unix_ms?: unknown;
}

interface LiveCandleStreamStatusResponse {
  active?: unknown;
  stream?: unknown;
}

interface LiveCandleStreamResponse {
  stopped?: unknown;
  symbol?: unknown;
  delayed?: unknown;
  channel?: unknown;
}

interface ExecuteSignalResponse {
  action?: unknown;
  order_submitted?: unknown;
  order_id?: unknown;
  client_order_id?: unknown;
  delta_qty?: unknown;
  last_bar_ts?: unknown;
  bracket_preview?: unknown;
  broker_reconciliation_events?: unknown;
  warnings?: unknown;
}

interface ExecutionBracketPreviewLegResponse {
  leg?: unknown;
  side?: unknown;
  order_type?: unknown;
  qty?: unknown;
  limit_price?: unknown;
  stop_price?: unknown;
  status?: unknown;
  trigger?: unknown;
}

interface ExecutionBracketPreviewResponse {
  source?: unknown;
  preview_only?: unknown;
  entry_side?: unknown;
  entry_price?: unknown;
  qty?: unknown;
  time_in_force?: unknown;
  tie_break?: unknown;
  legs?: unknown;
}

interface PolygonLiveStatusEventPayload {
  status?: unknown;
  message?: unknown;
  market?: unknown;
  symbol?: unknown;
  delayed?: unknown;
  channel?: unknown;
  at_unix_ms?: unknown;
}

interface LiveExecutionRunnerEventPayload {
  status?: unknown;
  message?: unknown;
  symbol?: unknown;
  at_unix_ms?: unknown;
  bar_open_unix_ms?: unknown;
  execution?: unknown;
}

const RUNNER_ACTIVE_EVENT_STATUSES = new Set([
  "started",
  "triggered",
  "queued_latest",
  "signal_processed",
  "order_submitted",
]);

const RUNNER_INACTIVE_EVENT_STATUSES = new Set(["stopped", "cancelled"]);

const BROKER_OPEN_ORDER_STATUSES = new Set([
  "new",
  "accepted",
  "pending_new",
  "accepted_for_bidding",
  "partially_filled",
  "pending_replace",
  "held",
  "stopped",
  "calculated",
  "suspended",
]);

export type ExecutionLiveLifecycleState =
  | "off"
  | "active"
  | "connecting"
  | "subscribed"
  | "started"
  | "closed"
  | "error"
  | "stopped";

export type ExecutionRunnerLifecycleState =
  | "off"
  | "active"
  | "started"
  | "triggered"
  | "queued_latest"
  | "signal_processed"
  | "order_submitted"
  | "stopped"
  | "cancelled"
  | "error";

export interface ExecutionOrderRow {
  cells: string[];
}

export interface ExecutionPreviewMeta {
  sourceCommand:
    | "none"
    | "test_alpaca_connection"
    | "get_live_execution_snapshot"
    | "get_local_paper_execution_snapshot"
    | "execute_strategy_signal_once"
    | "preview_execution_policy_decision"
    | "alpaca_clear_api_credentials";
  scopeLabel: string;
  generatedAtUnixMs: number | null;
}

export interface ExecutionAccountReview {
  state: "not_verified" | "broker_confirmed" | "unavailable";
  mode: "paper" | "live";
  sourceCommand:
    | "none"
    | "test_alpaca_connection"
    | "get_live_execution_snapshot"
    | "get_local_paper_execution_snapshot";
  fetchedAtUnixMs: number | null;
  accountId: string;
  accountNumber: string;
  status: string;
  currency: string;
  buyingPower: string;
  cash: string;
  portfolioValue: string;
}

export interface ExecutionPositionReview {
  state: "suppressed" | "flat" | "open" | "unavailable";
  symbol: string;
  qty: number | null;
  side: string;
  avgEntryPrice: number | null;
  currentPrice: number | null;
  marketValue: number | null;
  unrealizedPl: number | null;
  unrealizedPlpc: number | null;
}

export interface ExecutionOrderReview {
  state: "disabled" | "broker_confirmed";
  scope: "symbol_scoped" | "all_symbols";
  sourceCommand:
    | "none"
    | "get_live_execution_snapshot"
    | "get_local_paper_execution_snapshot";
  fetchedAtUnixMs: number | null;
  totalCount: number;
  openCount: number;
}

export interface ExecutionChartReview {
  state:
    | "no_symbol"
    | "no_dataset"
    | "symbol_mismatch"
    | "stale_broker_review"
    | "provenance_suppressed"
    | "loaded_no_markers"
    | "loaded_broker_markers"
    | "broker_review_loaded_chart_unavailable"
    | "sync_pending";
  symbol: string;
  datasetId: string;
  datasetLabel: string;
  datasetSymbol: string;
  orderCount: number;
  sourceCommand:
    | "none"
    | "get_live_execution_snapshot"
    | "get_local_paper_execution_snapshot";
  fetchedAtUnixMs: number | null;
}

export interface ExecutionBrokerReview {
  sourceCommand:
    | "get_live_execution_snapshot"
    | "get_local_paper_execution_snapshot";
  mode: "paper" | "live";
  symbol: string;
  fetchedAtUnixMs: number | null;
  staleThresholdMs: number;
  account: ExecutionAccountReview;
  position: ExecutionPositionReview;
  orders: ExecutionOrderReview;
  chart: ExecutionChartReview;
}

export interface ExecutionActionResult {
  statusLine: string;
  statusHint?: string;
  liveStatusHint?: string;
  runnerStatusHint?: string;
  liveLifecycleState?: ExecutionLiveLifecycleState;
  runnerLifecycleState?: ExecutionRunnerLifecycleState;
  recoveryReviewRequired?: boolean;
  brokerReviewRefreshRequired?: boolean;
  recoveryCause?: string;
  prependOrderRows?: ExecutionOrderRow[];
  positionMeta?: string;
  chartMeta?: string;
  accountMeta?: string;
  reviewMeta?: string;
  accountReview?: ExecutionAccountReview;
  brokerReview?: ExecutionBrokerReview;
  outputPreviewMeta?: ExecutionPreviewMeta;
  outputPreview?: string;
  orderRows?: ExecutionOrderRow[];
  orderCount?: number;
  liveActive?: boolean;
  runnerActive?: boolean;
  liveSymbol?: string;
  liveDelayed?: boolean;
}

export interface ExecutionStoredCredentialStatus {
  rememberAvailable: boolean;
  hasApiKey: boolean;
  hasApiSecret: boolean;
  hasCredentials: boolean;
  state: "missing" | "stored" | "incomplete" | "unavailable";
  storageBackend: string;
  message: string;
}

export interface ExecutionSavedSessionSummary {
  ts: string;
  path: string;
  sessionId: string;
  mode: "paper" | "live";
  symbol: string;
  strategyKind: string;
  dataId: string;
  dryRun: boolean;
  paramSource: string;
  sessionState: string;
  createdAtUnixMs: number | null;
  finalizedAtUnixMs: number | null;
  eventCount: number;
  brokerSnapshotCount: number;
}

export interface ExecutionSavedSessionEvent {
  provenance: string;
  status: string;
  message: string;
  symbol: string;
  atUnixMs: number | null;
  barOpenUnixMs: number | null;
  action: string;
  orderId: string;
  deltaQty: number | null;
  orderSubmitted: boolean | null;
  warnings: string[];
}

export interface ExecutionSavedSessionBrokerSnapshot {
  sourceCommand: string;
  mode: "paper" | "live";
  symbol: string;
  atUnixMs: number | null;
  accountStatus: string;
  currency: string;
  positionState: string;
  positionQty: number | null;
  positionSide: string;
  openOrderCount: number;
  orderCount: number;
}

export interface ExecutionSavedSessionDetail {
  ts: string;
  artifactPath: string;
  sessionId: string;
  sessionState: string;
  mode: "paper" | "live";
  symbol: string;
  qty: number;
  dryRun: boolean;
  strategyKind: string;
  dataId: string;
  paramSource: string;
  createdAtUnixMs: number | null;
  finalizedAtUnixMs: number | null;
  sourceRunArtifactPath: string;
  sourceRunConfigFingerprint: string;
  sourceRunRouteSummary: string;
  eventCount: number;
  brokerSnapshotCount: number;
  events: ExecutionSavedSessionEvent[];
  brokerSnapshots: ExecutionSavedSessionBrokerSnapshot[];
}

export interface ParsedLiveStatus {
  active: boolean;
  symbol: string;
  delayed: boolean;
  hint: string;
  status: string;
  message: string;
  lastStatusAtUnixMs?: number | null;
  lastAcceptedBarOpenUnixMs?: number | null;
  lastAcceptedBarCloseUnixMs?: number | null;
  lastAcceptedBarReceivedAtUnixMs?: number | null;
}

export interface ParsedRunnerEvent {
  status: string;
  symbol: string;
  message: string;
  hint: string;
  active: boolean | null;
  execution: ExecuteSignalResponse | null;
}

interface ExecutionFollowupWarning {
  short: string;
  detail: string;
}

export interface ExecutionRuntimeEventHandlers {
  onPolygonLiveStatus?: (event: ParsedLiveStatus) => void;
  onRunnerEvent?: (event: ParsedRunnerEvent) => void;
}

export interface ExecutionStrategySupportRow {
  strategyKind: string;
  supported: boolean;
  adapter: string;
  supportLevel: string;
  notes: string;
}

function text(value: unknown): string {
  return String(value ?? "").trim();
}

export function canonicalSymbol(raw: unknown): string {
  return String(raw ?? "")
    .trim()
    .replace(/\s+/g, "")
    .toUpperCase();
}

function toNumber(value: unknown): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return Number.NaN;
  return parsed;
}

function ensureTrailingPeriod(value: string): string {
  const trimmed = text(value);
  if (!trimmed) return "";
  return /[.!?]$/.test(trimmed) ? trimmed : `${trimmed}.`;
}

function detailAfterPrefix(detail: string, prefix: string): string {
  const trimmed = text(detail);
  if (!trimmed) return "";
  return trimmed.toLowerCase().startsWith(prefix.toLowerCase())
    ? text(trimmed.slice(prefix.length))
    : trimmed;
}

function liveFeedSubject(symbolRaw: unknown, delayed: boolean): string {
  const symbol = canonicalSymbol(symbolRaw);
  return `Live feed${symbol ? ` for ${symbol}` : ""}${delayed ? " [delayed]" : ""}`;
}

function startsWithAutoRunnerLabel(value: string): boolean {
  return /^auto-runner\b/i.test(text(value));
}

function compactRunnerFailureForSummary(value: string): string {
  const trimmed = text(value);
  if (!trimmed) return "";
  const compact = trimmed.replace(
    /^auto-runner(?: for [A-Z0-9./_-]+)?\s+/i,
    "",
  );
  if (!compact) return "";
  return compact.charAt(0).toUpperCase() + compact.slice(1);
}

export function normalizeExecutionOperatorError(value: unknown): string {
  const detail = text(value);
  const lowered = detail.toLowerCase();
  if (!detail) return "";
  if (
    lowered.includes("failed to fetch alpaca account") ||
    lowered.includes("alpaca account request failed")
  ) {
    return "Alpaca account review failed. Check credentials and broker connectivity, then retry.";
  }
  if (
    lowered.includes("failed to fetch alpaca orders") ||
    lowered.includes("alpaca orders request failed")
  ) {
    return "Broker order review failed. Check Alpaca connectivity, then refresh again.";
  }
  if (
    lowered.includes("failed to fetch alpaca position") ||
    lowered.includes("failed to fetch alpaca positions") ||
    lowered.includes("alpaca position request failed") ||
    lowered.includes("alpaca positions request failed")
  ) {
    return "Broker position review failed. Check Alpaca connectivity, then refresh again.";
  }
  if (
    lowered.includes("failed to cancel alpaca order") ||
    lowered.includes("alpaca order cancel failed")
  ) {
    return "Broker order cancellation failed. Check Alpaca connectivity and retry.";
  }
  if (
    lowered.includes("failed to submit alpaca order") ||
    lowered.includes("alpaca order failed")
  ) {
    return "Broker order submission failed. Confirm credentials, buying power, and broker availability, then retry.";
  }
  if (lowered.includes("failed to decode alpaca")) {
    return "Broker response decoding failed. Retry the request.";
  }
  if (
    lowered.includes("failed to initialize alpaca http client") ||
    lowered.includes("failed to build alpaca http client")
  ) {
    return "Broker connection could not be initialized. Check credentials and network access, then retry.";
  }
  if (lowered.includes("polygon api key")) {
    return "Polygon credentials are missing. Configure the market-data API key before starting live candles.";
  }
  return detail;
}

function normalizeLiveRuntimeMessage(
  statusRaw: unknown,
  rawMessage: unknown,
  symbolRaw: unknown,
  delayed: boolean,
): string {
  const status = text(statusRaw).toLowerCase();
  const detail = text(rawMessage);
  const subject = liveFeedSubject(symbolRaw, delayed);
  if (status === "connecting") return `${subject} connecting.`;
  if (status === "subscribed")
    return `${subject} subscribed to minute aggregates.`;
  if (status === "started") return `${subject} started.`;
  if (status === "stopped") return `${subject} stopped.`;
  if (status === "closed") {
    if (!detail || detail === "remote close" || detail === "stream ended") {
      return `${subject} disconnected. Refresh broker review before re-arming automation.`;
    }
    return `${subject} closed. ${ensureTrailingPeriod(detail)} Refresh broker review before re-arming automation.`;
  }
  if (status === "error") {
    if (detail.includes("connect timed out")) {
      return `${subject} connection timed out. Check Polygon connectivity, then refresh broker review before re-arming automation.`;
    }
    if (detail.includes("connect failed")) {
      return `${subject} connection failed. Check Polygon connectivity, then refresh broker review before re-arming automation.`;
    }
    if (detail.includes("auth send")) {
      return `${subject} authentication failed. Check the Polygon API key, then refresh broker review before re-arming automation.`;
    }
    if (detail.includes("subscribe send")) {
      return `${subject} subscription failed. Check Polygon connectivity, then refresh broker review before re-arming automation.`;
    }
    if (detail.includes("stream read failed")) {
      return `${subject} disconnected while reading market data. Check Polygon connectivity, then refresh broker review before re-arming automation.`;
    }
    if (detail) {
      return `${subject} encountered an error. ${ensureTrailingPeriod(detail)} Refresh broker review before re-arming automation.`;
    }
    return `${subject} encountered an error. Refresh broker review before re-arming automation.`;
  }
  return detail;
}

function normalizeRunnerFailureMessage(
  rawMessage: unknown,
  symbolRaw: unknown,
): string {
  const detail = text(rawMessage);
  const lowered = detail.toLowerCase();
  const symbol = canonicalSymbol(symbolRaw);
  const subject = `Auto-runner${symbol ? ` for ${symbol}` : ""}`;
  if (!detail) return "";
  if (lowered.startsWith("runner failed to apply live bar:")) {
    const inner = detailAfterPrefix(detail, "runner failed to apply live bar:");
    const normalizedInner = normalizeExecutionOperatorError(inner);
    if (normalizedInner === inner || !normalizedInner) {
      return `${subject} could not apply the latest live candle to the loaded dataset. Review the dataset and re-arm only after broker review is refreshed.`;
    }
    return `${subject} could not apply the latest live candle. ${normalizedInner}`;
  }
  if (lowered.startsWith("runner failed to load strategy candles:")) {
    const inner = detailAfterPrefix(
      detail,
      "runner failed to load strategy candles:",
    );
    if (inner.toLowerCase().includes("unknown data id")) {
      return `${subject} could not load the selected execution dataset. Reload the matching dataset and refresh broker review before re-arming.`;
    }
    const normalizedInner = normalizeExecutionOperatorError(inner);
    if (normalizedInner === inner || !normalizedInner) {
      return `${subject} could not load strategy candles for the selected dataset. Reload the matching dataset before re-arming.`;
    }
    return `${subject} could not load strategy candles. ${normalizedInner}`;
  }
  if (lowered.startsWith("runner execution failed:")) {
    const inner = detailAfterPrefix(detail, "runner execution failed:");
    const normalizedInner = normalizeExecutionOperatorError(inner);
    if (normalizedInner === inner || !normalizedInner) {
      return `${subject} execution failed. ${ensureTrailingPeriod(inner)}`;
    }
    return `${subject} execution failed. ${normalizedInner}`;
  }
  return detail;
}

function normalizeRunnerEventMessage(
  statusRaw: unknown,
  rawMessage: unknown,
  symbolRaw: unknown,
): string {
  const status = text(statusRaw).toLowerCase();
  const message = text(rawMessage);
  const symbol = canonicalSymbol(symbolRaw);
  if (status === "started") {
    return `Auto-runner started${symbol ? ` for ${symbol}` : ""}.`;
  }
  if (status === "stopped") return "Auto-runner stopped.";
  if (status === "cancelled") return "Auto-runner cancelled.";
  if (status === "triggered") {
    return `Auto-runner triggered on the latest live bar${symbol ? ` for ${symbol}` : ""}.`;
  }
  if (status === "queued_latest") {
    return "Auto-runner queued the latest live bar while the prior run was still in flight.";
  }
  if (status === "error") {
    return (
      normalizeRunnerFailureMessage(message, symbolRaw) ||
      `Auto-runner error${symbol ? ` for ${symbol}` : ""}.`
    );
  }
  return message;
}

function positiveInt(value: unknown): number {
  const parsed = Math.floor(toNumber(value));
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return isBoundaryObject(value);
}

const BROKER_REVIEW_STALE_THRESHOLD_MS = 90_000;

function fetchedAtUnixMs(raw: unknown): number | null {
  const parsed = Math.floor(toNumber(raw));
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

function looksLikeTradingSymbol(raw: unknown): boolean {
  const symbol = canonicalSymbol(raw);
  return /^[A-Z][A-Z0-9./-]{0,23}$/.test(symbol);
}

function latestExecutionDatasetContext(): {
  dataId: string;
  datasetLabel: string;
  datasetSymbol: string;
} {
  const pinia = resolveRuntimePinia();
  if (pinia) {
    try {
      const dataStore = useDataStore(pinia);
      const dataId = text(dataStore.dataId);
      const datasetLabel = text(dataStore.datasetLabel) || dataId || "-";
      const directSymbol = looksLikeTradingSymbol(dataId)
        ? canonicalSymbol(dataId)
        : "";
      const labelSymbol = looksLikeTradingSymbol(
        datasetLabel.replace(/\.[A-Za-z0-9]+$/, ""),
      )
        ? canonicalSymbol(datasetLabel.replace(/\.[A-Za-z0-9]+$/, ""))
        : "";
      if (dataId) {
        return {
          dataId,
          datasetLabel,
          datasetSymbol: directSymbol || labelSymbol,
        };
      }
    } catch {}
  }
  const snapshot = readLatestWorkflowRunSnapshot();
  const runReq = isRecord(snapshot?.req) ? snapshot.req : null;
  const runRes = isRecord(snapshot?.res) ? snapshot.res : null;
  const dataId = text(
    runReq?.data_id ?? runReq?.dataId ?? runRes?.data_id ?? runRes?.dataId,
  );
  const datasetLabel = dataId || "-";
  const datasetSymbol = looksLikeTradingSymbol(datasetLabel)
    ? canonicalSymbol(datasetLabel)
    : "";
  return {
    dataId,
    datasetLabel,
    datasetSymbol,
  };
}

function buildPreviewMeta(
  sourceCommand: ExecutionPreviewMeta["sourceCommand"],
  scopeLabel: string,
  generatedAtUnixMs: number | null,
): ExecutionPreviewMeta {
  return {
    sourceCommand,
    scopeLabel: text(scopeLabel) || "unscoped",
    generatedAtUnixMs:
      generatedAtUnixMs && generatedAtUnixMs > 0
        ? Math.floor(generatedAtUnixMs)
        : null,
  };
}

function modeValue(raw: unknown): "paper" | "live" {
  return text(raw).toLowerCase() === "live" ? "live" : "paper";
}

export function executionTargetValue(
  raw: unknown,
  modeRaw: unknown = "paper",
  dryRunRaw: unknown = true,
): ExecutionTarget {
  const normalized = text(raw).toLowerCase();
  if (normalized === "local_paper" || normalized === "local-paper")
    return "local_paper";
  if (normalized === "alpaca_paper" || normalized === "alpaca-paper")
    return "alpaca_paper";
  if (normalized === "alpaca_live" || normalized === "alpaca-live")
    return "alpaca_live";
  if (
    normalized === "dry_run" ||
    normalized === "dry-run" ||
    normalized === "simulation"
  )
    return "dry_run";
  if (dryRunRaw) return "dry_run";
  return modeValue(modeRaw) === "live" ? "alpaca_live" : "alpaca_paper";
}

function modeForExecutionTarget(target: ExecutionTarget): "paper" | "live" {
  return target === "alpaca_live" ? "live" : "paper";
}

function isDryRunTarget(target: ExecutionTarget): boolean {
  return target === "dry_run";
}

function isAlpacaExecutionTarget(target: ExecutionTarget): boolean {
  return target === "alpaca_paper" || target === "alpaca_live";
}

function brokerTargetForMode(modeRaw: unknown): ExecutionTarget {
  return modeValue(modeRaw) === "live" ? "alpaca_live" : "alpaca_paper";
}

function reviewExecutionTarget(
  options: ExecutionActionOptions,
): ExecutionTarget {
  const explicit = text(options.executionTarget);
  if (explicit === "local_paper" || explicit === "local-paper")
    return "local_paper";
  if (explicit === "alpaca_paper" || explicit === "alpaca-paper")
    return "alpaca_paper";
  if (explicit === "alpaca_live" || explicit === "alpaca-live")
    return "alpaca_live";
  return brokerTargetForMode(options.mode);
}

function sourceValue(raw: unknown): "selected" | "best" | "manual" {
  const normalized = text(raw).toLowerCase();
  if (normalized === "best") return "best";
  if (normalized === "manual") return "manual";
  return "selected";
}

function parseRiskPct(raw: unknown, label: string): number | null {
  const value = text(raw);
  if (!value) return null;
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`${label} must be finite and > 0.`);
  }
  const fraction = parsed > 1 ? parsed / 100 : parsed;
  if (!Number.isFinite(fraction) || fraction <= 0 || fraction >= 1) {
    throw new Error(`${label} must be in (0, 1) or a percent in (0, 100).`);
  }
  return fraction;
}

function parsePositiveOptionalNumber(
  raw: unknown,
  label: string,
): number | null {
  const value = text(raw);
  if (!value) return null;
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`${label} must be finite and > 0.`);
  }
  return parsed;
}

function parsePositiveOptionalInteger(
  raw: unknown,
  label: string,
): number | null {
  const parsed = parsePositiveOptionalNumber(raw, label);
  if (parsed == null) return null;
  if (!Number.isSafeInteger(parsed)) {
    throw new Error(`${label} must be a positive integer.`);
  }
  return parsed;
}

function nullableBoolean(value: unknown): boolean | null {
  return typeof value === "boolean" ? value : null;
}

function positiveNumberOrNull(value: unknown): number | null {
  const parsed = toNumber(value);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

function parseQty(raw: unknown): number {
  const rawText = text(raw);
  const value = Number(raw);
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error("Execution quantity must be finite and > 0.");
  }
  const rounded = decimalRoundQuantity(rawText, value);
  if (!Number.isFinite(rounded) || rounded <= 0) {
    throw new Error("Execution quantity is too small.");
  }
  return rounded;
}

function decimalRoundQuantity(rawText: string, fallbackValue: number): number {
  const decimalText = /^[+-]?(?:\d+|\d*\.\d+)$/.test(rawText)
    ? rawText
    : fallbackValue.toFixed(12);
  const unsigned = decimalText.trim().replace(/^[+-]/, "");
  const [integerRaw = "0", fractionalRaw = ""] = unsigned.split(".", 2);
  const integerUnits = Number.parseInt(integerRaw || "0", 10) * 10_000;
  const padded = `${fractionalRaw}00000`;
  const fractionalUnits = Number.parseInt(padded.slice(0, 4) || "0", 10);
  const roundDigit = Number.parseInt(padded.slice(4, 5) || "0", 10);
  return (integerUnits + fractionalUnits + (roundDigit >= 5 ? 1 : 0)) / 10_000;
}

function requireSymbol(raw: unknown): string {
  const symbol = canonicalSymbol(raw);
  if (!symbol) throw new Error("Execution symbol is required (e.g. AAPL).");
  return symbol;
}

function parseFallbackLen(value: unknown): number | null {
  if (Array.isArray(value)) {
    const next = Math.floor(toNumber(value[0]));
    return Number.isFinite(next) && next >= 1 && next <= 65_535 ? next : null;
  }
  if (isRecord(value)) {
    const next = Math.floor(toNumber(value.start));
    return Number.isFinite(next) && next >= 1 && next <= 65_535 ? next : null;
  }
  const next = Math.floor(toNumber(value));
  return Number.isFinite(next) && next >= 1 && next <= 65_535 ? next : null;
}

function parseManualLen(
  raw: unknown,
  fallbackRaw: unknown,
  label: string,
): number {
  const value = text(raw);
  const parsedRaw = value
    ? Math.floor(toNumber(value))
    : parseFallbackLen(fallbackRaw);
  if (
    typeof parsedRaw !== "number" ||
    !Number.isFinite(parsedRaw) ||
    parsedRaw < 1 ||
    parsedRaw > 65_535
  ) {
    throw new Error(`${label} must be an integer in [1, 65535].`);
  }
  return parsedRaw;
}

function safePrettyJson(value: unknown): string {
  try {
    return JSON.stringify(value ?? null, null, 2);
  } catch {
    return String(value ?? "");
  }
}

function trimTrailingZeros(num: number, maxDecimals = 6): string {
  if (!Number.isFinite(num)) return "-";
  const fixed = num.toFixed(Math.max(0, Math.floor(maxDecimals)));
  return fixed.replace(/(?:\.0+|(\.\d*?)0+)$/, "$1");
}

function redactExecutionSecrets(
  detail: string,
  secrets: Array<string | null | undefined>,
): string {
  let next = text(detail);
  for (const secret of secrets) {
    const token = text(secret);
    if (!token) continue;
    next = next.split(token).join("[redacted]");
  }
  return next;
}

function renderFollowupWarnings(
  base: string,
  warnings: ExecutionFollowupWarning[],
): string {
  const prefix = String(base || "").trim();
  if (!warnings.length) return prefix;
  const rendered = warnings
    .map((warning) => text(warning.detail))
    .filter((warning) => warning.length > 0);
  if (!rendered.length) return prefix;
  if (!prefix) return `Follow-up warnings:\n${rendered.join("\n")}`;
  return `${prefix}\n\nFollow-up warnings:\n${rendered.join("\n")}`;
}

function applyExecutionFollowupWarnings(
  result: ExecutionActionResult,
  warnings: ExecutionFollowupWarning[],
): ExecutionActionResult {
  const activeWarnings = warnings.filter(
    (warning) =>
      text(warning.short).length > 0 || text(warning.detail).length > 0,
  );
  if (!activeWarnings.length) return result;
  const shortText = activeWarnings
    .map((warning) => text(warning.short))
    .filter((warning) => warning.length > 0)
    .join(" ");
  const detailText = activeWarnings
    .map((warning) => text(warning.detail))
    .filter((warning) => warning.length > 0)
    .join(" ");
  return {
    ...result,
    statusLine:
      `${text(result.statusLine)}${shortText ? ` ${shortText}` : ""}`.trim(),
    statusHint:
      `${text(result.statusHint || result.statusLine)}${detailText ? ` ${detailText}` : ""}`.trim(),
    outputPreview: renderFollowupWarnings(
      result.outputPreview || "",
      activeWarnings,
    ),
  };
}

function toUtcDateTime(rawMs: unknown): string {
  return formatUtcDateTimeFromUnixMs(rawMs);
}

function orderTimeMs(order: ExecutionSnapshotOrderResponse): number {
  const candidates: unknown[] = [
    order.filled_at,
    order.submitted_at,
    order.updated_at,
    order.canceled_at,
    order.created_at,
  ];
  for (const raw of candidates) {
    if (typeof raw === "number" && Number.isFinite(raw) && raw > 0) {
      const abs = Math.abs(raw);
      return abs >= 1e12 ? Math.floor(raw) : Math.floor(raw * 1000);
    }
    const txt = text(raw);
    if (!txt) continue;
    const parsed = Date.parse(txt);
    if (Number.isFinite(parsed) && parsed > 0) return parsed;
  }
  return Number.NaN;
}

function buildOrderRows(ordersRaw: unknown): ExecutionOrderRow[] {
  if (!Array.isArray(ordersRaw)) return [];
  const rows: ExecutionOrderRow[] = [];
  for (const entry of ordersRaw) {
    const order = isRecord(entry)
      ? (entry as ExecutionSnapshotOrderResponse)
      : null;
    if (!order) continue;
    const timeText = toUtcDateTime(orderTimeMs(order));
    const side = text(order.side) || "-";
    const status = text(order.status) || "-";
    const qty = trimTrailingZeros(toNumber(order.qty));
    const filled = trimTrailingZeros(toNumber(order.filled_qty));
    const avgPx = trimTrailingZeros(toNumber(order.filled_avg_price));
    const orderId = text(order.id) || "-";
    const orderType = text(order.order_type) || text(order.type);
    const limitPrice = trimTrailingZeros(toNumber(order.limit_price));
    const stopPrice = trimTrailingZeros(toNumber(order.stop_price));
    const trigger = stopPrice
      ? `stop ${stopPrice}`
      : limitPrice
        ? `limit ${limitPrice}`
        : orderType || "-";
    const bracketLeg = text(order.bracket_leg);
    const tieBreak = text(order.tie_break);
    const bracket = bracketLeg
      ? `${bracketLeg}${tieBreak ? ` · ${tieBreak}` : ""}`
      : "-";
    const parent =
      text(order.parent_order_id) || text(order.bracket_group_id) || "-";
    rows.push({
      cells: [
        timeText,
        side,
        status,
        qty,
        filled,
        avgPx,
        orderId,
        trigger,
        bracket,
        parent,
      ],
    });
  }
  return rows;
}

function bracketPreviewHint(raw: unknown): string {
  const preview = isRecord(raw)
    ? (raw as ExecutionBracketPreviewResponse)
    : null;
  if (!preview || preview.preview_only !== true) return "";
  const entrySide = text(preview.entry_side) || "entry";
  const entryPrice = trimTrailingZeros(toNumber(preview.entry_price));
  const qty = trimTrailingZeros(toNumber(preview.qty));
  const tif = text(preview.time_in_force) || "gtc";
  const tieBreak = text(preview.tie_break) || "stop_loss_first";
  const legsRaw = Array.isArray(preview.legs) ? preview.legs : [];
  const legs = legsRaw
    .map((entry) => {
      const leg = isRecord(entry)
        ? (entry as ExecutionBracketPreviewLegResponse)
        : null;
      if (!leg) return "";
      const label = text(leg.leg).replace(/_/g, " ") || "child";
      const side = text(leg.side) || "-";
      const trigger =
        text(leg.trigger) ||
        (Number.isFinite(toNumber(leg.stop_price))
          ? `stop ${trimTrailingZeros(toNumber(leg.stop_price))}`
          : Number.isFinite(toNumber(leg.limit_price))
            ? `limit ${trimTrailingZeros(toNumber(leg.limit_price))}`
            : text(leg.order_type) || "-");
      return `${label}: ${side} ${trigger}`;
    })
    .filter(Boolean);
  if (!legs.length) return "";
  const entry = `${entrySide} ${qty || "-"} @ ${entryPrice || "-"}`;
  return `Local paper bracket preview only: ${entry}; ${legs.join("; ")}; tif ${tif}; tie-break ${tieBreak}.`;
}

function buildPositionReview(
  raw: unknown,
  symbolRaw: unknown,
): ExecutionPositionReview {
  const symbol = canonicalSymbol(symbolRaw);
  if (!symbol) {
    return {
      state: "suppressed",
      symbol: "",
      qty: null,
      side: "",
      avgEntryPrice: null,
      currentPrice: null,
      marketValue: null,
      unrealizedPl: null,
      unrealizedPlpc: null,
    };
  }
  const position = isRecord(raw)
    ? (raw as ExecutionSnapshotPositionResponse)
    : null;
  if (!position) {
    return {
      state: "flat",
      symbol,
      qty: 0,
      side: "",
      avgEntryPrice: null,
      currentPrice: null,
      marketValue: null,
      unrealizedPl: null,
      unrealizedPlpc: null,
    };
  }
  const qty = toNumber(position.qty);
  if (!Number.isFinite(qty) || Math.abs(qty) < 1e-12) {
    return {
      state: "flat",
      symbol,
      qty: 0,
      side: text(position.side),
      avgEntryPrice: null,
      currentPrice: null,
      marketValue: null,
      unrealizedPl: null,
      unrealizedPlpc: null,
    };
  }
  return {
    state: "open",
    symbol,
    qty,
    side: text(position.side) || (qty > 0 ? "long" : "short"),
    avgEntryPrice: Number.isFinite(toNumber(position.avg_entry_price))
      ? toNumber(position.avg_entry_price)
      : null,
    currentPrice: Number.isFinite(toNumber(position.current_price))
      ? toNumber(position.current_price)
      : null,
    marketValue: Number.isFinite(toNumber(position.market_value))
      ? toNumber(position.market_value)
      : null,
    unrealizedPl: Number.isFinite(toNumber(position.unrealized_pl))
      ? toNumber(position.unrealized_pl)
      : null,
    unrealizedPlpc: Number.isFinite(toNumber(position.unrealized_plpc))
      ? toNumber(position.unrealized_plpc)
      : null,
  };
}

function buildChartReview(
  symbolRaw: unknown,
  orderCount: number,
  fetchedAt: number | null,
  sourceCommand: ExecutionChartReview["sourceCommand"],
): ExecutionChartReview {
  const symbol = canonicalSymbol(symbolRaw);
  const dataset = latestExecutionDatasetContext();
  if (!symbol) {
    return {
      state: "no_symbol",
      symbol: "",
      datasetId: dataset.dataId,
      datasetLabel: dataset.datasetLabel,
      datasetSymbol: dataset.datasetSymbol,
      orderCount,
      sourceCommand,
      fetchedAtUnixMs: fetchedAt,
    };
  }
  if (!dataset.dataId) {
    return {
      state: "no_dataset",
      symbol,
      datasetId: "",
      datasetLabel: "-",
      datasetSymbol: "",
      orderCount,
      sourceCommand,
      fetchedAtUnixMs: fetchedAt,
    };
  }
  if (dataset.datasetSymbol && dataset.datasetSymbol !== symbol) {
    return {
      state: "symbol_mismatch",
      symbol,
      datasetId: dataset.dataId,
      datasetLabel: dataset.datasetLabel,
      datasetSymbol: dataset.datasetSymbol,
      orderCount,
      sourceCommand,
      fetchedAtUnixMs: fetchedAt,
    };
  }
  const stale =
    Number.isFinite(fetchedAt ?? Number.NaN) &&
    fetchedAt != null &&
    Date.now() - fetchedAt > BROKER_REVIEW_STALE_THRESHOLD_MS;
  if (stale) {
    return {
      state: "stale_broker_review",
      symbol,
      datasetId: dataset.dataId,
      datasetLabel: dataset.datasetLabel,
      datasetSymbol: dataset.datasetSymbol,
      orderCount,
      sourceCommand,
      fetchedAtUnixMs: fetchedAt,
    };
  }
  return {
    state: orderCount > 0 ? "loaded_broker_markers" : "loaded_no_markers",
    symbol,
    datasetId: dataset.dataId,
    datasetLabel: dataset.datasetLabel,
    datasetSymbol: dataset.datasetSymbol,
    orderCount,
    sourceCommand,
    fetchedAtUnixMs: fetchedAt,
  };
}

function buildAccountReview(
  mode: "paper" | "live",
  raw: unknown,
  sourceCommand: ExecutionAccountReview["sourceCommand"],
  fetchedAtRaw: unknown,
): ExecutionAccountReview {
  const resp = isRecord(raw) ? (raw as ExecutionConnectionResponse) : null;
  if (!resp) {
    return {
      state: sourceCommand === "none" ? "not_verified" : "unavailable",
      mode,
      sourceCommand,
      fetchedAtUnixMs: fetchedAtUnixMs(fetchedAtRaw),
      accountId: "",
      accountNumber: "",
      status: "",
      currency: "",
      buyingPower: "",
      cash: "",
      portfolioValue: "",
    };
  }
  const accountId = text(resp.account_id);
  const accountNumber = text(resp.account_number);
  const status = text(resp.status);
  const currency = text(resp.currency);
  const buyingPower = text(resp.buying_power);
  const cash = text(resp.cash);
  const portfolioValue = text(resp.portfolio_value);
  const hasAnyDetails = !!(
    accountId ||
    accountNumber ||
    status ||
    currency ||
    buyingPower ||
    cash ||
    portfolioValue
  );
  return {
    state: hasAnyDetails ? "broker_confirmed" : "unavailable",
    mode,
    sourceCommand,
    fetchedAtUnixMs: fetchedAtUnixMs(resp.fetched_at_unix_ms ?? fetchedAtRaw),
    accountId,
    accountNumber,
    status,
    currency,
    buyingPower,
    cash,
    portfolioValue,
  };
}

function positionMeta(
  review: ExecutionPositionReview | null | undefined,
): string {
  if (!review || review.state === "suppressed") return "Position: -";
  if (review.state === "flat") return "Position: flat";
  if (review.state !== "open" || !Number.isFinite(review.qty ?? Number.NaN))
    return "Position: unavailable.";
  const avg = trimTrailingZeros(review.avgEntryPrice ?? Number.NaN, 4);
  const now = trimTrailingZeros(review.currentPrice ?? Number.NaN, 4);
  const upl = trimTrailingZeros(review.unrealizedPl ?? Number.NaN, 4);
  const uplPctText = Number.isFinite(review.unrealizedPlpc ?? Number.NaN)
    ? `${trimTrailingZeros((review.unrealizedPlpc ?? 0) * 100, 2)}%`
    : "-";
  return `Position: qty=${trimTrailingZeros(review.qty ?? Number.NaN, 6)} side=${review.side || "-"} avg=${avg} now=${now} upl=${upl} (${uplPctText})`;
}

function chartMeta(review: ExecutionChartReview | null | undefined): string {
  if (!review)
    return "Load a dataset and refresh orders to overlay broker events.";
  if (review.state === "no_symbol")
    return "Execution chart unavailable: select a symbol first.";
  if (review.state === "no_dataset")
    return "Execution chart unavailable: load a workflow dataset first.";
  if (review.state === "symbol_mismatch") {
    return `Execution chart unavailable: loaded dataset ${review.datasetLabel || review.datasetId || "-"} does not match ${review.symbol || "-"}.`;
  }
  if (review.state === "stale_broker_review") {
    return `Execution chart loaded for ${review.symbol || "-"}, but broker review is stale as of ${toUtcDateTime(review.fetchedAtUnixMs)}.`;
  }
  if (review.state === "provenance_suppressed") {
    return `Execution chart loaded for ${review.symbol || "-"}. Broker-confirmed markers stay suppressed until a symbol-scoped broker review is refreshed.`;
  }
  if (review.state === "loaded_no_markers") {
    return `Execution chart loaded for ${review.symbol || "-"}. No broker-confirmed order markers are present in the current review.`;
  }
  if (review.state === "loaded_broker_markers") {
    return `Execution chart loaded for ${review.symbol || "-"} with ${review.orderCount} broker-confirmed order marker${review.orderCount === 1 ? "" : "s"}.`;
  }
  if (review.state === "broker_review_loaded_chart_unavailable") {
    return `Execution chart unavailable after broker review. symbol=${review.symbol || "-"}`;
  }
  return `Execution chart sync pending. symbol=${review.symbol || "-"} orders=${review.orderCount}`;
}

function accountMeta(
  review: ExecutionAccountReview | null | undefined,
): string {
  if (!review || review.state === "not_verified")
    return "Account review: not verified.";
  if (review.state === "unavailable")
    return `Account review: ${review.mode} account unavailable.`;
  const parts = [
    `Account review: ${review.mode}`,
    review.accountNumber || review.accountId
      ? `account=${review.accountNumber || review.accountId}`
      : "",
    review.status ? `status=${review.status}` : "",
    review.currency ? `currency=${review.currency}` : "",
    review.buyingPower ? `buying_power=${review.buyingPower}` : "",
    review.cash ? `cash=${review.cash}` : "",
    review.portfolioValue ? `portfolio=${review.portfolioValue}` : "",
  ].filter((part) => part.length > 0);
  return parts.join(" · ");
}

function isOpenBrokerOrderStatus(raw: unknown): boolean {
  return BROKER_OPEN_ORDER_STATUSES.has(text(raw).toLowerCase());
}

function openOrderCount(ordersRaw: unknown): number {
  if (!Array.isArray(ordersRaw)) return 0;
  let count = 0;
  for (const entry of ordersRaw) {
    if (!isRecord(entry)) continue;
    if (isOpenBrokerOrderStatus(entry.status)) count += 1;
  }
  return count;
}

function reviewMeta(review: ExecutionBrokerReview | null | undefined): string {
  if (!review)
    return "Broker review: unavailable until the selected symbol is refreshed.";
  const symbol = review.symbol || "-";
  const fetchedAt = toUtcDateTime(review.fetchedAtUnixMs);
  const accountStatus = review.account.status || "unknown";
  return `Broker review: ${review.mode} symbol-scoped ${symbol} · as of ${fetchedAt} · status=${accountStatus} · ${review.orders.openCount} open orders`;
}

function buildBrokerReview(
  mode: "paper" | "live",
  symbolRaw: unknown,
  raw: unknown,
  sourceCommand: ExecutionBrokerReview["sourceCommand"] = "get_live_execution_snapshot",
): ExecutionBrokerReview | null {
  const resp = isRecord(raw) ? (raw as ExecutionSnapshotResponse) : null;
  const symbol = canonicalSymbol(resp?.symbol ?? symbolRaw);
  if (!resp || !symbol) return null;
  const fetchedAt = fetchedAtUnixMs(resp.fetched_at_unix_ms);
  const ordersCount = Array.isArray(resp.orders) ? resp.orders.length : 0;
  return {
    sourceCommand,
    mode,
    symbol,
    fetchedAtUnixMs: fetchedAt,
    staleThresholdMs: BROKER_REVIEW_STALE_THRESHOLD_MS,
    account: buildAccountReview(
      mode,
      {
        account_id: resp.account_id,
        account_number: resp.account_number,
        status: resp.account_status,
        currency: resp.currency,
        buying_power: resp.buying_power,
        cash: resp.cash,
        portfolio_value: resp.portfolio_value,
        fetched_at_unix_ms: resp.fetched_at_unix_ms,
      },
      sourceCommand,
      resp.fetched_at_unix_ms,
    ),
    position: buildPositionReview(resp.position, symbol),
    orders: {
      state: "broker_confirmed",
      scope: "symbol_scoped",
      sourceCommand,
      fetchedAtUnixMs: fetchedAt,
      totalCount: ordersCount,
      openCount: openOrderCount(resp.orders),
    },
    chart: buildChartReview(symbol, ordersCount, fetchedAt, sourceCommand),
  };
}

function summarizeRunnerStatus(raw: unknown): {
  text: string;
  active: boolean;
  lifecycle: ExecutionRunnerLifecycleState;
} {
  const status = parseBoundaryObject(raw) as ExecutionRunnerStatusResponse;
  const active = !!status.active;
  const cfg = isRecord(status.config)
    ? (status.config as ExecutionRunnerConfigResponse)
    : null;
  const lastError = compactRunnerFailureForSummary(
    normalizeRunnerFailureMessage(status.last_error, cfg?.symbol),
  );
  if (!active || !cfg) {
    return {
      text: lastError
        ? `Auto-runner: off. Last failure: ${lastError}`
        : "Auto-runner: off",
      active: false,
      lifecycle: lastError ? "error" : "off",
    };
  }
  const mode = text(cfg.mode).toLowerCase() || "paper";
  const dry = !!cfg.dry_run ? "dry-run" : "broker";
  const sym = canonicalSymbol(cfg.symbol) || text(cfg.symbol) || "-";
  const runs = Math.max(0, Math.floor(Number(status.run_count || 0)));
  const errs = Math.max(0, Math.floor(Number(status.error_count || 0)));
  const skips = Math.max(0, Math.floor(Number(status.skip_count || 0)));
  const inflight = !!status.inflight;
  const lastAction = text(status.last_action).toLowerCase();
  const lastOrderId = text(status.last_order_id);
  const lastCompletedAt = fetchedAtUnixMs(status.last_completed_at_unix_ms);
  const lastTriggeredAt = fetchedAtUnixMs(status.last_triggered_at_unix_ms);
  const lastBarAt = fetchedAtUnixMs(status.last_bar_open_unix_ms);
  const detailParts: string[] = [];
  if (lastAction) {
    detailParts.push(`last action=${lastAction}`);
  }
  if (lastOrderId) {
    detailParts.push(`order=${lastOrderId}`);
  }
  if (lastCompletedAt != null) {
    detailParts.push(`completed=${toUtcDateTime(lastCompletedAt)}`);
  } else if (lastTriggeredAt != null) {
    detailParts.push(`triggered=${toUtcDateTime(lastTriggeredAt)}`);
  } else if (lastBarAt != null) {
    detailParts.push(`bar=${toUtcDateTime(lastBarAt)}`);
  }
  const detailSuffix =
    detailParts.length > 0 ? ` · ${detailParts.join(" · ")}` : "";
  const summary = `Auto-runner: on (${sym}, ${mode}, ${dry}) runs=${runs} skips=${skips} errs=${errs}${inflight ? " [in-flight]" : ""}${detailSuffix}`;
  return {
    text: lastError ? `${summary}. Last failure: ${lastError}` : summary,
    active,
    lifecycle: "active",
  };
}

function liveStatusHint(
  active: boolean,
  symbolRaw: unknown,
  delayed: boolean,
  statusRaw?: unknown,
  rawMessage?: unknown,
): string {
  const symbol = canonicalSymbol(symbolRaw);
  const suffix = symbol ? ` for ${symbol}` : "";
  const delayText = delayed ? " [delayed]" : "";
  const status = text(statusRaw).toLowerCase();
  if (active && status === "connecting") {
    return `Live candles connecting${suffix}${delayText}.`.trim();
  }
  if (active && status === "subscribed") {
    return `Live candles subscribed${suffix}${delayText}.`.trim();
  }
  if (active && status === "started") {
    return `Live candles started${suffix}${delayText}.`.trim();
  }
  if (!active) {
    if (status === "stopped") return "Live candles stopped.";
    if (status === "closed" || status === "error") {
      return normalizeLiveRuntimeMessage(status, rawMessage, symbol, delayed);
    }
    return "Live candles: off";
  }
  return `Live candles active${suffix}${delayText}.`.trim();
}

function parseLiveStreamStatus(raw: unknown): ParsedLiveStatus {
  const row = parseBoundaryObject(raw) as LiveCandleStreamStatusResponse;
  const stream = isRecord(row.stream)
    ? (row.stream as LiveCandleStreamInfoResponse)
    : null;
  const active = !!row.active;
  const symbol = canonicalSymbol(stream?.symbol);
  const delayed = !!stream?.delayed;
  const status =
    text(stream?.status).toLowerCase() || (active ? "active" : "off");
  const lastStatusAtUnixMs = fetchedAtUnixMs(stream?.last_status_at_unix_ms);
  const lastAcceptedBarOpenUnixMs = fetchedAtUnixMs(
    stream?.last_accepted_bar_open_unix_ms,
  );
  const lastAcceptedBarCloseUnixMs = fetchedAtUnixMs(
    stream?.last_accepted_bar_close_unix_ms,
  );
  const lastAcceptedBarReceivedAtUnixMs = fetchedAtUnixMs(
    stream?.last_accepted_bar_received_at_unix_ms,
  );
  const message = normalizeLiveRuntimeMessage(
    status,
    stream?.last_message,
    symbol,
    delayed,
  );
  const baseHint = liveStatusHint(
    active,
    symbol,
    delayed,
    status,
    stream?.last_message,
  );
  const hint =
    active && lastAcceptedBarReceivedAtUnixMs != null
      ? `${baseHint.replace(/\.$/, "")} · last bar received=${toUtcDateTime(lastAcceptedBarReceivedAtUnixMs)}.`
      : baseHint;
  return {
    active,
    symbol,
    delayed,
    hint,
    status,
    message,
    lastStatusAtUnixMs,
    lastAcceptedBarOpenUnixMs,
    lastAcceptedBarCloseUnixMs,
    lastAcceptedBarReceivedAtUnixMs,
  };
}

export function parsePolygonLiveStatusEvent(raw: unknown): ParsedLiveStatus {
  const row = parseBoundaryObject(raw) as PolygonLiveStatusEventPayload;
  const status = text(row.status).toLowerCase();
  const symbol = canonicalSymbol(row.symbol);
  const delayed = !!row.delayed;
  const active =
    status === "started" || status === "connecting" || status === "subscribed";
  const message = normalizeLiveRuntimeMessage(
    status,
    row.message,
    symbol,
    delayed,
  );
  return {
    active,
    symbol,
    delayed,
    hint: liveStatusHint(active, symbol, delayed, status, row.message),
    status,
    message,
  };
}

export function optimisticOrderRowFromExecution(
  executionRaw: unknown,
): ExecutionOrderRow | null {
  const execution = isRecord(executionRaw)
    ? (executionRaw as ExecuteSignalResponse)
    : null;
  if (!execution?.order_submitted) return null;
  const lastBarTs = toNumber(execution.last_bar_ts);
  const atUnixMs =
    Number.isFinite(lastBarTs) && lastBarTs > 0
      ? Math.floor(lastBarTs)
      : Date.now();
  const side = text(execution.action).toLowerCase() || "-";
  const qty = trimTrailingZeros(Math.abs(toNumber(execution.delta_qty)), 6);
  const orderId = text(execution.order_id) || "-";
  return {
    cells: [
      toUtcDateTime(atUnixMs),
      side,
      "submitted*",
      qty,
      "-",
      "-",
      orderId,
    ],
  };
}

export function parseRunnerEvent(raw: unknown): ParsedRunnerEvent {
  const row = parseBoundaryObject(raw) as LiveExecutionRunnerEventPayload;
  const status = text(row.status).toLowerCase();
  const symbol = canonicalSymbol(row.symbol);
  const message = normalizeRunnerEventMessage(status, row.message, symbol);
  let active: boolean | null = null;
  if (RUNNER_INACTIVE_EVENT_STATUSES.has(status)) {
    active = false;
  }
  if (RUNNER_ACTIVE_EVENT_STATUSES.has(status)) {
    active = true;
  }
  return {
    status,
    symbol,
    message,
    hint: message
      ? startsWithAutoRunnerLabel(message)
        ? message
        : `Auto-runner: ${message}`
      : "Auto-runner status updated.",
    active,
    execution: isRecord(row.execution)
      ? (row.execution as ExecuteSignalResponse)
      : null,
  };
}

export async function subscribeExecutionRuntimeEvents(
  handlers: ExecutionRuntimeEventHandlers,
): Promise<() => void> {
  const unsubs = await Promise.all([
    listenEvent<PolygonLiveStatusEventPayload>(
      "polygon_live_status",
      (payload) => {
        handlers.onPolygonLiveStatus?.(parsePolygonLiveStatusEvent(payload));
      },
    ),
    listenEvent<LiveExecutionRunnerEventPayload>(
      "live_execution_runner_event",
      (payload) => {
        handlers.onRunnerEvent?.(parseRunnerEvent(payload));
      },
    ),
  ]);
  return () => {
    for (const off of unsubs) {
      try {
        off?.();
      } catch {}
    }
  };
}

function parseStoredExecutionKeys(raw: unknown): {
  apiKey: string;
  apiSecret: string;
} {
  const payload = isRecord(raw) ? (raw as ExecutionStoredKeysPayload) : null;
  if (!payload) return { apiKey: "", apiSecret: "" };
  return {
    apiKey: text(payload.api_key),
    apiSecret: text(payload.api_secret),
  };
}

function parseStoredExecutionCredentialStatus(
  raw: unknown,
): ExecutionStoredCredentialStatus {
  const row = parseBoundaryObject(
    raw,
  ) as ExecutionStoredCredentialStatusResponse;
  const stateRaw = text(row.state).toLowerCase();
  const state =
    stateRaw === "stored" ||
    stateRaw === "incomplete" ||
    stateRaw === "unavailable"
      ? (stateRaw as ExecutionStoredCredentialStatus["state"])
      : "missing";
  return {
    rememberAvailable: !!row.remember_available,
    hasApiKey: !!row.has_api_key,
    hasApiSecret: !!row.has_api_secret,
    hasCredentials: !!row.has_credentials,
    state,
    storageBackend: text(row.storage_backend),
    message: text(row.message),
  };
}

function routeProvenanceSummary(raw: unknown): string {
  if (!isRecord(raw)) return "";
  const requested = text(raw.requestedRoute ?? raw.requested_route);
  const resolved = text(raw.resolvedRoute ?? raw.resolved_route);
  const realized = text(raw.realizedRoute ?? raw.realized_route);
  const fallback = text(raw.fallbackReason ?? raw.fallback_reason);
  const parts = [
    requested ? `requested=${requested}` : "",
    resolved ? `resolved=${resolved}` : "",
    realized ? `realized=${realized}` : "",
    fallback ? `fallback=${fallback}` : "",
  ].filter((part) => part.length > 0);
  return parts.join(" · ");
}

function parseSavedSessionSummary(
  raw: unknown,
): ExecutionSavedSessionSummary | null {
  const row = isRecord(raw) ? raw : null;
  if (!row) return null;
  const path = text(row.path);
  if (!path) return null;
  return {
    ts: text(row.ts),
    path,
    sessionId: text(row.sessionId ?? row.session_id),
    mode: modeValue(row.mode),
    symbol: canonicalSymbol(row.symbol),
    strategyKind: text(row.strategyKind ?? row.strategy_kind),
    dataId: text(row.dataId ?? row.data_id),
    dryRun: !!row.dryRun,
    paramSource: text(row.paramSource ?? row.param_source),
    sessionState: text(row.sessionState ?? row.session_state),
    createdAtUnixMs: fetchedAtUnixMs(
      row.createdAtUnixMs ?? row.created_at_unix_ms,
    ),
    finalizedAtUnixMs: fetchedAtUnixMs(
      row.finalizedAtUnixMs ?? row.finalized_at_unix_ms,
    ),
    eventCount: Math.max(
      0,
      Math.floor(toNumber(row.eventCount ?? row.event_count) || 0),
    ),
    brokerSnapshotCount: Math.max(
      0,
      Math.floor(
        toNumber(row.brokerSnapshotCount ?? row.broker_snapshot_count) || 0,
      ),
    ),
  };
}

function parseSavedSessionEvent(
  raw: unknown,
): ExecutionSavedSessionEvent | null {
  const row = isRecord(raw) ? raw : null;
  if (!row) return null;
  const provenance = text(row.provenance);
  const status = text(row.status);
  const message = text(row.message);
  if (!provenance && !status && !message) return null;
  const deltaQty = toNumber(row.deltaQty ?? row.delta_qty);
  return {
    provenance,
    status,
    message,
    symbol: canonicalSymbol(row.symbol),
    atUnixMs: fetchedAtUnixMs(row.atUnixMs ?? row.at_unix_ms),
    barOpenUnixMs: fetchedAtUnixMs(row.barOpenUnixMs ?? row.bar_open_unix_ms),
    action: text(row.action),
    orderId: text(row.orderId ?? row.order_id),
    deltaQty: Number.isFinite(deltaQty) ? deltaQty : null,
    orderSubmitted:
      typeof (row.orderSubmitted ?? row.order_submitted) === "boolean"
        ? Boolean(row.orderSubmitted ?? row.order_submitted)
        : null,
    warnings: Array.isArray(row.warnings)
      ? row.warnings
          .map((entry) => text(entry))
          .filter((entry) => entry.length > 0)
      : [],
  };
}

function parseSavedSessionBrokerSnapshot(
  raw: unknown,
): ExecutionSavedSessionBrokerSnapshot | null {
  const row = isRecord(raw) ? raw : null;
  if (!row) return null;
  const sourceCommand = text(row.sourceCommand ?? row.source_command);
  const symbol = canonicalSymbol(row.symbol);
  if (!sourceCommand && !symbol) return null;
  const positionQty = toNumber(row.positionQty ?? row.position_qty);
  return {
    sourceCommand,
    mode: modeValue(row.mode),
    symbol,
    atUnixMs: fetchedAtUnixMs(row.atUnixMs ?? row.at_unix_ms),
    accountStatus: text(row.accountStatus ?? row.account_status),
    currency: text(row.currency),
    positionState: text(row.positionState ?? row.position_state),
    positionQty: Number.isFinite(positionQty) ? positionQty : null,
    positionSide: text(row.positionSide ?? row.position_side),
    openOrderCount: Math.max(
      0,
      Math.floor(toNumber(row.openOrderCount ?? row.open_order_count) || 0),
    ),
    orderCount: Math.max(
      0,
      Math.floor(toNumber(row.orderCount ?? row.order_count) || 0),
    ),
  };
}

function parseSavedSessionDetail(raw: unknown): ExecutionSavedSessionDetail {
  const row = parseBoundaryObject(raw);
  const qty = toNumber(row.qty);
  const rawBrokerSnapshots = row.brokerSnapshots ?? row.broker_snapshots;
  const events = parseBoundaryArray(row.events)
    .map(parseSavedSessionEvent)
    .filter((event): event is ExecutionSavedSessionEvent => !!event);
  const brokerSnapshots = parseBoundaryArray(rawBrokerSnapshots)
    .map(parseSavedSessionBrokerSnapshot)
    .filter(
      (snapshot): snapshot is ExecutionSavedSessionBrokerSnapshot => !!snapshot,
    );
  return {
    ts: text(row.ts),
    artifactPath: text(row.artifactPath ?? row.artifact_path),
    sessionId: text(row.sessionId ?? row.session_id),
    sessionState: text(row.sessionState ?? row.session_state),
    mode: modeValue(row.mode),
    symbol: canonicalSymbol(row.symbol),
    qty: Number.isFinite(qty) ? qty : 0,
    dryRun: !!row.dryRun,
    strategyKind: text(row.strategyKind ?? row.strategy_kind),
    dataId: text(row.dataId ?? row.data_id),
    paramSource: text(row.paramSource ?? row.param_source),
    createdAtUnixMs: fetchedAtUnixMs(
      row.createdAtUnixMs ?? row.created_at_unix_ms,
    ),
    finalizedAtUnixMs: fetchedAtUnixMs(
      row.finalizedAtUnixMs ?? row.finalized_at_unix_ms,
    ),
    sourceRunArtifactPath: text(
      row.sourceRunArtifactPath ?? row.source_run_artifact_path,
    ),
    sourceRunConfigFingerprint: text(
      row.sourceRunConfigFingerprint ?? row.source_run_config_fingerprint,
    ),
    sourceRunRouteSummary: routeProvenanceSummary(
      row.sourceRunRouteProvenance ?? row.source_run_route_provenance,
    ),
    eventCount: Math.max(
      0,
      Math.floor(toNumber(row.eventCount ?? row.event_count) || events.length),
    ),
    brokerSnapshotCount: Math.max(
      0,
      Math.floor(
        toNumber(row.brokerSnapshotCount ?? row.broker_snapshot_count) ||
          brokerSnapshots.length,
      ),
    ),
    events,
    brokerSnapshots,
  };
}

function latestExecutionSourceRunContext(): {
  artifactPath: string;
  configFingerprint: string;
  routeProvenance: unknown;
} {
  const snapshot = readLatestWorkflowRunSnapshot();
  return {
    artifactPath: text(snapshot?.artifactPath),
    configFingerprint: text(snapshot?.configFingerprint),
    routeProvenance: snapshot?.routeProvenance ?? null,
  };
}

export function loadPersistedExecutionKeys(): {
  apiKey: string;
  apiSecret: string;
} {
  try {
    const parsed = JSON.parse(readStorageString(EXEC_KEYS_STORAGE_KEY, "null"));
    return parseStoredExecutionKeys(parsed);
  } catch {
    return { apiKey: "", apiSecret: "" };
  }
}

function resolveTypedExecutionCredentialOverride(
  options: ExecutionActionOptions,
): { apiKey: string; apiSecret: string } | null {
  const apiKey = text(options.apiKey);
  const apiSecret = text(options.apiSecret);
  if (!apiKey && !apiSecret) return null;
  if (!apiKey || !apiSecret) {
    throw new Error(
      "API key + secret must both be provided when using a typed override.",
    );
  }
  return { apiKey, apiSecret };
}

function applyTypedExecutionCredentialOverride(
  req: Record<string, unknown>,
  options: ExecutionActionOptions,
): {
  req: Record<string, unknown>;
  typedOverride: { apiKey: string; apiSecret: string } | null;
} {
  const target = req.execution_target
    ? executionTargetValue(
        req.execution_target,
        req.mode ?? options.mode,
        req.dry_run ?? options.dryRun,
      )
    : req.dry_run == null && req.mode != null
      ? brokerTargetForMode(req.mode)
      : executionTargetValue(
          options.executionTarget,
          req.mode ?? options.mode,
          req.dry_run ?? options.dryRun,
        );
  if (!isAlpacaExecutionTarget(target)) {
    return { req, typedOverride: null };
  }
  const typedOverride = resolveTypedExecutionCredentialOverride(options);
  if (typedOverride) {
    req.alpaca_api_key = typedOverride.apiKey;
    req.alpaca_api_secret = typedOverride.apiSecret;
  }
  return { req, typedOverride };
}

async function persistRememberedExecutionTypedOverride(
  options: ExecutionActionOptions,
  typedOverride: { apiKey: string; apiSecret: string } | null,
): Promise<void> {
  if (!options.rememberKeys || !typedOverride) return;
  await storeExecutionCredentialsSecurely(
    typedOverride.apiKey,
    typedOverride.apiSecret,
  );
}

async function persistRememberedExecutionTypedOverrideWarning(
  options: ExecutionActionOptions,
  typedOverride: { apiKey: string; apiSecret: string } | null,
  successContext: string,
): Promise<ExecutionFollowupWarning | null> {
  if (!options.rememberKeys || !typedOverride) return null;
  try {
    await persistRememberedExecutionTypedOverride(options, typedOverride);
    return null;
  } catch (error) {
    const detail = redactExecutionSecrets(storeErrorText(error), [
      typedOverride.apiKey,
      typedOverride.apiSecret,
      options.apiKey,
      options.apiSecret,
    ]);
    return {
      short: "Secure credential storage did not update.",
      detail: `${text(successContext) || "The action still succeeded."} Secure credential storage did not update${detail ? `: ${detail}` : "."}`,
    };
  }
}

function brokerReviewRefreshFailureWarning(
  actionSummary: string,
  error: unknown,
): ExecutionFollowupWarning {
  const detail = storeErrorText(error);
  return {
    short: "Broker review refresh failed.",
    detail: `${text(actionSummary) || "The broker-facing action still succeeded."} Broker review refresh failed${detail ? `: ${detail}` : "."} Refresh orders manually now.`,
  };
}

function brokerReconciliationRecoveryCause(raw: unknown): string {
  if (!Array.isArray(raw)) return "";
  for (const entry of raw) {
    if (!isRecord(entry)) continue;
    const status = text(entry.status);
    const timedOut = !!entry.timed_out;
    if (timedOut)
      return `Order timeout: broker status ${text(entry.broker_order_status) || status || "unknown"}.`;
    if (status === "reconciliation_unknown" || status === "broker_unknown")
      return "Broker unknown: refresh broker review before re-arming.";
    if (status === "reconciliation_error")
      return "Broker reconciliation error: refresh broker review before re-arming.";
  }
  return "";
}

async function resolveManualExecutionStrategy(
  options: ExecutionActionOptions,
): Promise<{
  strategyKind: string;
  strategy: {
    kind: "double_ma" | "composite_cpu";
    req: Record<string, unknown>;
  };
  source: string;
}> {
  let runReq = isRecord(readLatestWorkflowRunSnapshot()?.req)
    ? (readLatestWorkflowRunSnapshot()?.req as Record<string, unknown>)
    : null;
  if (!runReq) {
    const { buildOptimizationRequestFromVueStores } =
      await loadOptimizeService();
    runReq = await buildOptimizationRequestFromVueStores();
  }

  const fastMaType = text(
    Array.isArray(runReq.fast_ma_types)
      ? runReq.fast_ma_types[0]
      : runReq.fast_ma_type,
  );
  const slowMaType = text(
    Array.isArray(runReq.slow_ma_types)
      ? runReq.slow_ma_types[0]
      : runReq.slow_ma_type,
  );
  if (!fastMaType || !slowMaType) {
    throw new Error(
      "Select fast/slow MA types before using Manual parameter source.",
    );
  }

  const runRes = isRecord(readLatestWorkflowRunSnapshot()?.res)
    ? (readLatestWorkflowRunSnapshot()?.res as Record<string, unknown>)
    : null;
  const fallbackBest = isRecord(runRes?.best_params)
    ? (runRes?.best_params as Record<string, unknown>)
    : null;
  const fastLen = parseManualLen(
    options.manualFastLen,
    runReq.fast_range ?? fallbackBest?.fast_len ?? 5,
    "Manual fast length",
  );
  const slowLen = parseManualLen(
    options.manualSlowLen,
    runReq.slow_range ?? fallbackBest?.slow_len ?? 20,
    "Manual slow length",
  );
  if (fastLen >= slowLen) {
    throw new Error("Manual parameters require fast length < slow length.");
  }

  const { buildExecutionStrategyFromRunContext } = await loadAnalysisService();
  const strategySnapshot = buildExecutionStrategyFromRunContext(
    {
      fast_len: fastLen,
      slow_len: slowLen,
      fast_ma_type: fastMaType,
      slow_ma_type: slowMaType,
    },
    runReq,
  );
  if (!strategySnapshot) {
    throw new Error(
      "Unable to build manual execution strategy from current settings.",
    );
  }
  return {
    ...strategySnapshot,
    source: "manual_ui",
  };
}

async function buildExecutionInvokePayload(
  options: ExecutionActionOptions,
): Promise<{
  req: Record<string, unknown>;
  strategyKind: string;
  source: string;
}> {
  const executionTarget = executionTargetValue(
    options.executionTarget,
    options.mode,
    options.dryRun,
  );
  const mode = modeForExecutionTarget(executionTarget);
  const dry_run = isDryRunTarget(executionTarget);
  const qty = parseQty(options.qty);
  const symbol = requireSymbol(options.symbol);
  const risk: ExecutionRiskPayload = {
    stop_loss_pct: parseRiskPct(options.riskStopLossPct, "Risk override SL"),
    take_profit_pct: parseRiskPct(
      options.riskTakeProfitPct,
      "Risk override TP",
    ),
    max_order_notional: parsePositiveOptionalNumber(
      options.riskMaxOrderNotional,
      "Max order notional",
    ),
    max_position_notional: parsePositiveOptionalNumber(
      options.riskMaxPositionNotional,
      "Max position notional",
    ),
    max_daily_loss: parsePositiveOptionalNumber(
      options.riskMaxDailyLoss,
      "Max daily loss",
    ),
    max_daily_orders: parsePositiveOptionalInteger(
      options.riskMaxDailyOrders,
      "Max daily orders",
    ),
    allow_short: nullableBoolean(options.riskAllowShort),
    require_bracket: nullableBoolean(options.riskRequireBracket),
  };
  const sourceRun = latestExecutionSourceRunContext();

  const paramSource = sourceValue(options.paramSource);
  if (paramSource === "manual") {
    const manual = await resolveManualExecutionStrategy(options);
    return {
      req: {
        symbol,
        qty,
        mode,
        dry_run,
        execution_target: executionTarget,
        risk,
        param_source: manual.source,
        source_run_artifact_path: sourceRun.artifactPath,
        source_run_config_fingerprint: sourceRun.configFingerprint,
        source_run_route_provenance: sourceRun.routeProvenance,
        strategy: manual.strategy,
      },
      strategyKind: manual.strategyKind,
      source: manual.source,
    };
  }

  const snapshot = readLatestWorkflowRunSnapshot();
  const runReq = isRecord(snapshot?.req)
    ? (snapshot.req as Record<string, unknown>)
    : null;
  if (!runReq) {
    throw new Error(
      "No optimization run loaded. Use Manual parameter source or run an optimization.",
    );
  }
  const {
    buildExecutionStrategyFromRunContext,
    resolveExecutionTopRowSnapshot,
  } = await loadAnalysisService();
  const row = resolveExecutionTopRowSnapshot(paramSource);
  const strategy = buildExecutionStrategyFromRunContext(row.params, runReq);
  if (!strategy) {
    throw new Error(
      "Unable to build strategy request from selected parameters.",
    );
  }
  return {
    req: {
      symbol,
      qty,
      mode,
      dry_run,
      execution_target: executionTarget,
      risk,
      param_source: row.source,
      source_run_artifact_path: sourceRun.artifactPath,
      source_run_config_fingerprint: sourceRun.configFingerprint,
      source_run_route_provenance: sourceRun.routeProvenance,
      strategy: strategy.strategy,
    },
    strategyKind: strategy.strategyKind,
    source: row.source,
  };
}

async function refreshRunnerStatusHint(): Promise<{
  hint: string;
  active: boolean;
  lifecycle: ExecutionRunnerLifecycleState;
}> {
  const raw = await invokeCommand("live_execution_runner_status");
  const summary = summarizeRunnerStatus(raw);
  return {
    hint: summary.text,
    active: summary.active,
    lifecycle: summary.lifecycle,
  };
}

async function refreshLiveStatus(): Promise<ParsedLiveStatus> {
  return parseLiveStreamStatus(
    (await invokeCommand(
      "polygon_live_candle_stream_status",
    )) as LiveCandleStreamStatusResponse,
  );
}

function normalizeExecutionRiskPolicy(raw: unknown): ExecutionRiskPolicy {
  const row = parseBoundaryObject(raw);
  return {
    max_order_notional: positiveNumberOrNull(row.max_order_notional),
    max_position_notional: positiveNumberOrNull(row.max_position_notional),
    max_daily_loss: positiveNumberOrNull(row.max_daily_loss),
    max_daily_orders: positiveNumberOrNull(row.max_daily_orders),
    allow_short: nullableBoolean(row.allow_short),
    require_bracket: nullableBoolean(row.require_bracket),
  };
}

function serializeExecutionRiskPolicy(
  policy: Partial<ExecutionRiskPolicy>,
): ExecutionRiskPolicy {
  return {
    max_order_notional: parsePositiveOptionalNumber(
      policy.max_order_notional,
      "Max order notional",
    ),
    max_position_notional: parsePositiveOptionalNumber(
      policy.max_position_notional,
      "Max position notional",
    ),
    max_daily_loss: parsePositiveOptionalNumber(
      policy.max_daily_loss,
      "Max daily loss",
    ),
    max_daily_orders: parsePositiveOptionalInteger(
      policy.max_daily_orders,
      "Max daily orders",
    ),
    allow_short: nullableBoolean(policy.allow_short),
    require_bracket: nullableBoolean(policy.require_bracket),
  };
}

function normalizeExecutionStrategySupportRow(
  raw: unknown,
): ExecutionStrategySupportRow | null {
  const row = isRecord(raw) ? (raw as Record<string, unknown>) : null;
  if (!row) return null;
  const strategyKind = text(row.strategy_kind ?? row.strategyKind);
  if (!strategyKind) return null;
  return {
    strategyKind,
    supported: Boolean(row.supported),
    adapter: text(row.adapter),
    supportLevel: text(row.support_level ?? row.supportLevel),
    notes: text(row.notes),
  };
}

export async function loadExecutionStrategySupportMatrix(): Promise<
  ExecutionStrategySupportRow[]
> {
  const raw = await invokeCommand("get_execution_strategy_support_matrix");
  return parseBoundaryArray(raw)
    .map((row) => normalizeExecutionStrategySupportRow(row))
    .filter((row): row is ExecutionStrategySupportRow => row !== null);
}

export async function loadExecutionRiskPolicy(): Promise<ExecutionRiskPolicy> {
  const resp = (await invokeCommand(
    "get_execution_risk_policy",
  )) as ExecutionRiskPolicyResponse;
  return normalizeExecutionRiskPolicy(resp?.policy);
}

export async function saveExecutionRiskPolicy(
  policy: Partial<ExecutionRiskPolicy>,
): Promise<ExecutionRiskPolicy> {
  const resp = (await invokeCommand("set_execution_risk_policy", {
    policy: serializeExecutionRiskPolicy(policy),
  })) as ExecutionRiskPolicyResponse;
  return normalizeExecutionRiskPolicy(resp?.policy);
}

export async function resetExecutionRiskPolicy(): Promise<ExecutionRiskPolicy> {
  const resp = (await invokeCommand(
    "reset_execution_risk_policy",
  )) as ExecutionRiskPolicyResponse;
  return normalizeExecutionRiskPolicy(resp?.policy);
}

export async function previewExecutionRiskPolicy(
  options: ExecutionActionOptions,
): Promise<ExecutionActionResult> {
  const payload = await buildExecutionInvokePayload(options);
  const resp = await invokeCommand("preview_execution_policy_decision", {
    req: payload.req,
  });
  const row = parseBoundaryObject(resp);
  const decision = isRecord(row.decision) ? row.decision : {};
  const allowed = !!decision.allowed;
  const code = text(decision.code) || (allowed ? "allowed" : "blocked");
  const message =
    text(decision.message) ||
    (allowed
      ? "Execution policy allowed the current signal."
      : "Execution policy blocked the current signal.");
  const warnings = Array.isArray(row.warnings)
    ? row.warnings
        .map((warning) => text(warning))
        .filter((warning) => warning.length > 0)
    : [];
  const previewHint = bracketPreviewHint(row.bracket_preview);
  const hintParts = [message];
  if (previewHint) hintParts.push(previewHint);
  if (warnings.length > 0) hintParts.push(warnings.join(" "));
  return {
    statusLine: allowed
      ? "Execution policy preview allowed."
      : `Execution policy preview blocked: ${code}.`,
    statusHint: hintParts.join(" ").trim(),
    outputPreview: safePrettyJson(resp),
    outputPreviewMeta: buildPreviewMeta(
      "preview_execution_policy_decision",
      `policy preview ${canonicalSymbol(row.symbol) || "-"}`,
      Date.now(),
    ),
  };
}

export async function testConnection(
  options: ExecutionActionOptions,
): Promise<ExecutionActionResult> {
  const mode = modeValue(options.mode);
  const { req, typedOverride } = applyTypedExecutionCredentialOverride(
    { mode },
    options,
  );
  const resp = (await invokeCommand("test_alpaca_connection", {
    req,
  })) as ExecutionConnectionResponse;
  const persistWarning = await persistRememberedExecutionTypedOverrideWarning(
    options,
    typedOverride,
    `Alpaca ${mode} connection still succeeded.`,
  );
  const accountReview = buildAccountReview(
    mode,
    resp,
    "test_alpaca_connection",
    resp?.fetched_at_unix_ms,
  );
  const account = text(resp?.account_number || resp?.account_id);
  const status = text(resp?.status);
  const suffix = `${account ? ` account=${account}` : ""}${status ? ` status=${status}` : ""}`;
  return applyExecutionFollowupWarnings(
    {
      statusLine: `Alpaca ${mode} connection OK.`,
      statusHint: `Alpaca ${mode} connection OK.${suffix}`.trim(),
      accountMeta: accountMeta(accountReview),
      accountReview,
      outputPreview: safePrettyJson(resp),
      outputPreviewMeta: buildPreviewMeta(
        "test_alpaca_connection",
        `${mode} account verification`,
        accountReview.fetchedAtUnixMs,
      ),
    },
    persistWarning ? [persistWarning] : [],
  );
}

export async function refreshOrders(
  options: ExecutionActionOptions,
): Promise<ExecutionActionResult> {
  const target = reviewExecutionTarget(options);
  const mode = modeForExecutionTarget(target);
  const symbol = requireSymbol(options.symbol);
  const sourceCommand =
    target === "local_paper"
      ? "get_local_paper_execution_snapshot"
      : "get_live_execution_snapshot";
  const baseReq: Record<string, unknown> =
    target === "local_paper"
      ? { symbol, limit: 200 }
      : {
          mode,
          symbol,
          limit: 200,
          include_account: true,
          include_orders: true,
          include_position: true,
        };
  const { req, typedOverride } = applyTypedExecutionCredentialOverride(
    baseReq,
    options,
  );
  const resp = (await invokeCommand(sourceCommand, {
    req,
  })) as ExecutionSnapshotResponse;
  const persistWarning =
    target === "local_paper"
      ? null
      : await persistRememberedExecutionTypedOverrideWarning(
          options,
          typedOverride,
          "Broker review still loaded successfully.",
        );
  const rows = buildOrderRows(resp?.orders);
  const count = rows.length;
  const brokerReview = buildBrokerReview(
    mode,
    resp?.symbol || symbol,
    resp,
    sourceCommand,
  );
  const scopeLabel = symbol
    ? `recent order${count === 1 ? "" : "s"} for ${symbol}`
    : `order${count === 1 ? "" : "s"} for all symbols`;
  const result: ExecutionActionResult = {
    statusLine: `Loaded ${count} ${scopeLabel}.`,
    statusHint: `Loaded ${count} ${scopeLabel}.`,
    outputPreview: safePrettyJson(resp),
    outputPreviewMeta: buildPreviewMeta(
      sourceCommand,
      `symbol-scoped ${canonicalSymbol(resp?.symbol || symbol) || symbol}`,
      fetchedAtUnixMs(resp?.fetched_at_unix_ms),
    ),
    orderRows: rows,
    orderCount: count,
    positionMeta: positionMeta(brokerReview?.position),
    chartMeta: chartMeta(brokerReview?.chart),
    accountMeta: accountMeta(brokerReview?.account),
    reviewMeta: reviewMeta(brokerReview),
  };
  if (brokerReview?.account) result.accountReview = brokerReview.account;
  if (brokerReview) result.brokerReview = brokerReview;
  return applyExecutionFollowupWarnings(
    result,
    persistWarning ? [persistWarning] : [],
  );
}

export async function startLive(
  options: ExecutionActionOptions,
): Promise<ExecutionActionResult> {
  const symbol = requireSymbol(options.symbol);
  const delayed = !!options.liveUseDelayed;
  const resp = (await invokeCommand("polygon_start_live_candle_stream", {
    req: { symbol, market: "stocks", delayed },
  })) as LiveCandleStreamResponse;
  return {
    statusLine: `Live candles started for ${text(resp?.symbol) || symbol}.`,
    liveStatusHint: `Live candles started for ${text(resp?.symbol) || symbol}${text(resp?.channel) ? ` (${text(resp?.channel)})` : ""}${resp?.delayed ? " [delayed]" : ""}.`,
    liveLifecycleState: "started",
    liveActive: true,
    liveSymbol: canonicalSymbol(resp?.symbol || symbol),
    liveDelayed: !!resp?.delayed,
  };
}

export async function stopLive(): Promise<ExecutionActionResult> {
  const resp = (await invokeCommand(
    "polygon_stop_live_candle_stream",
  )) as LiveCandleStreamResponse;
  const stopped = !!resp?.stopped;
  return {
    statusLine: stopped
      ? "Live candles stopped."
      : "Live candles were already stopped.",
    liveStatusHint: stopped
      ? "Live candles stopped."
      : "Live candles were already stopped.",
    liveLifecycleState: stopped ? "stopped" : "off",
    liveActive: false,
    liveSymbol: "",
    liveDelayed: false,
  };
}

export async function startRunner(
  options: ExecutionActionOptions,
): Promise<ExecutionActionResult> {
  const payload = await buildExecutionInvokePayload(options);
  const { req, typedOverride } = applyTypedExecutionCredentialOverride(
    payload.req,
    options,
  );

  const live = parseLiveStreamStatus(
    (await invokeCommand(
      "polygon_live_candle_stream_status",
    )) as LiveCandleStreamStatusResponse,
  );
  if (!live.active) {
    throw new Error(
      "Start live candles first. Auto-runner triggers on Polygon live bars.",
    );
  }
  const requestedSymbol = canonicalSymbol(req.symbol);
  if (live.symbol && live.symbol !== requestedSymbol) {
    throw new Error(
      `Live candles are active for ${live.symbol}, not ${requestedSymbol}.`,
    );
  }

  const statusRaw = await invokeCommand("start_live_execution_runner", { req });
  const persistWarning = await persistRememberedExecutionTypedOverrideWarning(
    options,
    typedOverride,
    "Auto-runner start still succeeded.",
  );
  const summary = summarizeRunnerStatus(statusRaw);
  return applyExecutionFollowupWarnings(
    {
      statusLine: `Auto-runner started for ${text(req.symbol) || "-"}.`,
      runnerStatusHint: summary.text,
      runnerLifecycleState: "started",
      runnerActive: summary.active,
      liveSymbol: live.symbol,
      liveDelayed: live.delayed,
    },
    persistWarning ? [persistWarning] : [],
  );
}

export async function stopRunner(): Promise<ExecutionActionResult> {
  const resp = await invokeCommand("stop_live_execution_runner");
  const row = isRecord(resp) ? resp : {};
  const stopped = !!row.stopped;
  const summary = summarizeRunnerStatus(row.status);
  return {
    statusLine: stopped
      ? "Auto-runner stopped."
      : "Auto-runner was already off.",
    runnerStatusHint: summary.text,
    runnerLifecycleState: stopped ? "stopped" : "off",
    runnerActive: summary.active,
  };
}

export async function resetLocalPaperBroker(
  options: ExecutionActionOptions,
): Promise<ExecutionActionResult> {
  const symbol = canonicalSymbol(options.symbol);
  const resp = (await invokeCommand(
    "reset_local_paper_broker",
  )) as ResetLocalPaperBrokerResponse;
  const snapshot = isRecord(resp?.snapshot)
    ? (resp.snapshot as ExecutionSnapshotResponse)
    : {};
  const rows = buildOrderRows(snapshot.orders);
  const reviewSnapshot = {
    ...snapshot,
    symbol: text(snapshot.symbol) || symbol || "LOCAL_PAPER",
  };
  const brokerReview = buildBrokerReview(
    "paper",
    reviewSnapshot.symbol,
    reviewSnapshot,
    "get_local_paper_execution_snapshot",
  );
  const result: ExecutionActionResult = {
    statusLine: "Local paper broker reset.",
    statusHint:
      "Local paper broker reset to its starting simulated account state.",
    outputPreview: safePrettyJson(resp),
    outputPreviewMeta: buildPreviewMeta(
      "get_local_paper_execution_snapshot",
      "local paper reset snapshot",
      fetchedAtUnixMs(snapshot.fetched_at_unix_ms),
    ),
    orderRows: rows,
    orderCount: rows.length,
    positionMeta: positionMeta(brokerReview?.position),
    chartMeta: chartMeta(brokerReview?.chart),
    accountMeta: accountMeta(brokerReview?.account),
    reviewMeta: reviewMeta(brokerReview),
  };
  if (brokerReview?.account) result.accountReview = brokerReview.account;
  if (brokerReview) result.brokerReview = brokerReview;
  return result;
}

export async function cancelOpenOrders(
  options: ExecutionActionOptions,
): Promise<ExecutionActionResult> {
  const target = reviewExecutionTarget(options);
  const mode = modeForExecutionTarget(target);
  const symbol = requireSymbol(options.symbol);
  const command =
    target === "local_paper"
      ? "cancel_local_paper_open_orders"
      : "cancel_live_execution_open_orders";
  const { req, typedOverride } = applyTypedExecutionCredentialOverride(
    { mode, symbol },
    options,
  );
  const resp = (await invokeCommand(command, {
    req,
  })) as CancelLiveExecutionOpenOrdersResponse;
  const cancelledCount = positiveInt(resp?.cancelled_count);
  const openOrderCountBefore = positiveInt(resp?.open_order_count_before);
  const statusLine =
    cancelledCount > 0
      ? `Cancelled ${cancelledCount} open broker order${cancelledCount === 1 ? "" : "s"} for ${symbol}.`
      : `No open broker orders to cancel for ${symbol}.`;
  const statusHint =
    cancelledCount > 0
      ? `${statusLine} Broker review refreshed from ${openOrderCountBefore} previously open order${openOrderCountBefore === 1 ? "" : "s"}.`
      : `${statusLine} Broker review refreshed.`;
  const warnings: ExecutionFollowupWarning[] = [];
  const persistWarning =
    target === "local_paper"
      ? null
      : await persistRememberedExecutionTypedOverrideWarning(
          options,
          typedOverride,
          `${statusLine} Broker-side order cancellation still succeeded.`,
        );
  if (persistWarning) warnings.push(persistWarning);
  try {
    const refreshed = await refreshOrders({ ...options, mode, symbol });
    return applyExecutionFollowupWarnings(
      {
        ...refreshed,
        statusLine,
        statusHint,
      },
      warnings,
    );
  } catch (error) {
    warnings.push(
      brokerReviewRefreshFailureWarning(
        `${statusLine} Broker-side order cancellation still succeeded.`,
        error,
      ),
    );
    return applyExecutionFollowupWarnings(
      {
        statusLine,
        statusHint: `${statusLine} Broker review refresh is still required.`,
        reviewMeta: `Broker review: ${mode} symbol-scoped ${symbol} pending manual refresh after a successful cancel-open-orders action.`,
        positionMeta: "Position: pending broker refresh.",
        chartMeta: `Execution chart pending refreshed broker review for ${symbol}.`,
        outputPreview: safePrettyJson(resp),
        brokerReviewRefreshRequired: true,
      },
      warnings,
    );
  }
}

export async function flattenPosition(
  options: ExecutionActionOptions,
): Promise<ExecutionActionResult> {
  const target = reviewExecutionTarget(options);
  const mode = modeForExecutionTarget(target);
  const symbol = requireSymbol(options.symbol);
  const command =
    target === "local_paper"
      ? "flatten_local_paper_position"
      : "flatten_live_execution_position";
  const { req, typedOverride } = applyTypedExecutionCredentialOverride(
    { mode, symbol },
    options,
  );
  const resp = (await invokeCommand(command, {
    req,
  })) as FlattenLiveExecutionPositionResponse;
  const submitted = !!resp?.order_submitted;
  const positionFound = !!resp?.position_found;
  const action = text(resp?.action).toUpperCase();
  const orderId = text(resp?.order_id);
  const statusLine = !positionFound
    ? `No open broker position to flatten for ${symbol}.`
    : submitted
      ? `Submitted ${action || "FLATTEN"} flatten order${orderId ? ` (id=${orderId})` : ""} for ${symbol}.`
      : `Flatten request for ${symbol} did not submit a broker order.`;
  const warnings: ExecutionFollowupWarning[] = [];
  const persistWarning =
    target === "local_paper"
      ? null
      : await persistRememberedExecutionTypedOverrideWarning(
          options,
          typedOverride,
          `${statusLine} Broker-side flatten handling still succeeded.`,
        );
  if (persistWarning) warnings.push(persistWarning);
  try {
    const refreshed = await refreshOrders({ ...options, mode, symbol });
    return applyExecutionFollowupWarnings(
      {
        ...refreshed,
        statusLine,
        statusHint: `${statusLine} Broker review refreshed.`,
      },
      warnings,
    );
  } catch (error) {
    warnings.push(
      brokerReviewRefreshFailureWarning(
        `${statusLine} Broker-side flatten handling still succeeded.`,
        error,
      ),
    );
    const result: ExecutionActionResult = {
      statusLine,
      statusHint: `${statusLine} Broker review refresh is still required.`,
      reviewMeta: `Broker review: ${mode} symbol-scoped ${symbol} pending manual refresh after a successful flatten-position action.`,
      positionMeta: "Position: pending broker refresh.",
      chartMeta: `Execution chart pending refreshed broker review for ${symbol}.`,
      outputPreview: safePrettyJson(resp),
      brokerReviewRefreshRequired: true,
    };
    if (submitted) {
      const optimisticRow = optimisticOrderRowFromExecution({
        order_submitted: true,
        action: text(resp?.action),
        order_id: orderId,
        delta_qty: toNumber(resp?.qty),
      });
      if (optimisticRow) result.prependOrderRows = [optimisticRow];
    }
    return applyExecutionFollowupWarnings(result, warnings);
  }
}

export async function emergencyHalt(): Promise<ExecutionActionResult> {
  const resp = (await invokeCommand(
    "emergency_halt_live_execution",
  )) as EmergencyHaltLiveExecutionResponse;
  const liveStopped = !!resp?.live_stopped;
  const runnerStopped = !!resp?.runner_stopped;
  const liveSymbol = canonicalSymbol(resp?.live_symbol);
  const statusLine =
    text(resp?.status_message) || "Live automation was already disarmed.";
  return {
    statusLine,
    statusHint: statusLine,
    liveStatusHint: liveStopped
      ? `Live candles stopped${liveSymbol ? ` for ${liveSymbol}` : ""}.`
      : "Live candles were already off.",
    runnerStatusHint: runnerStopped
      ? "Auto-runner stopped."
      : "Auto-runner was already off.",
    liveLifecycleState: liveStopped ? "stopped" : "off",
    runnerLifecycleState: runnerStopped ? "stopped" : "off",
    recoveryReviewRequired: !!resp?.requires_manual_review,
    liveActive: false,
    runnerActive: false,
    liveSymbol: "",
    liveDelayed: false,
    outputPreview: safePrettyJson(resp),
    outputPreviewMeta: buildPreviewMeta("none", "emergency halt", Date.now()),
  };
}

export async function runOnce(
  options: ExecutionActionOptions,
): Promise<ExecutionActionResult> {
  const payload = await buildExecutionInvokePayload(options);
  const { req, typedOverride } = applyTypedExecutionCredentialOverride(
    payload.req,
    options,
  );
  const resp = (await invokeCommand("execute_strategy_signal_once", {
    req,
  })) as ExecuteSignalResponse;

  const action = text(resp?.action || "hold") || "hold";
  const submitted = !!resp?.order_submitted;
  const orderId = text(resp?.order_id);
  const warningCount = Array.isArray(resp?.warnings) ? resp.warnings.length : 0;
  const policyBlockCode = action.startsWith("policy_blocked:")
    ? action.slice("policy_blocked:".length)
    : "";
  const recoveryCause = brokerReconciliationRecoveryCause(
    resp?.broker_reconciliation_events,
  );
  let statusHint = policyBlockCode
    ? `Execution policy blocked order: ${policyBlockCode}.`
    : submitted
      ? `Submitted ${action.toUpperCase()} order${orderId ? ` (id=${orderId})` : ""}.`
      : `Signal processed: action=${action}.`;
  if (req.dry_run) statusHint += " Dry-run only.";
  if (warningCount) statusHint += ` Warnings=${warningCount}.`;
  if (submitted && !req.dry_run) {
    statusHint += " Use Refresh orders for broker-confirmed state.";
  }

  const result: ExecutionActionResult = {
    statusLine: statusHint,
    statusHint,
    outputPreview: safePrettyJson(resp),
    outputPreviewMeta: buildPreviewMeta(
      "execute_strategy_signal_once",
      `symbol-scoped ${canonicalSymbol(req.symbol) || "-"}`,
      Date.now(),
    ),
  };
  if (policyBlockCode)
    result.recoveryCause = `Policy block: ${policyBlockCode}.`;
  if (recoveryCause) {
    result.recoveryCause = recoveryCause;
    result.recoveryReviewRequired = true;
  }

  if (submitted && !req.dry_run) {
    const optimisticRow = optimisticOrderRowFromExecution(resp);
    if (optimisticRow) {
      result.prependOrderRows = [optimisticRow];
    }
    result.brokerReviewRefreshRequired = true;
  }

  const persistWarning = await persistRememberedExecutionTypedOverrideWarning(
    options,
    typedOverride,
    submitted && !req.dry_run
      ? "Signal execution still completed and a broker order may already have been submitted."
      : "Signal execution still completed.",
  );
  return applyExecutionFollowupWarnings(
    result,
    persistWarning ? [persistWarning] : [],
  );
}

export async function refreshStatuses(): Promise<ExecutionActionResult> {
  const [runner, live] = await Promise.allSettled([
    refreshRunnerStatusHint(),
    refreshLiveStatus(),
  ]);
  const refreshWarnings: string[] = [];
  if (runner.status !== "fulfilled")
    refreshWarnings.push("Auto-runner status refresh unavailable.");
  if (live.status !== "fulfilled")
    refreshWarnings.push("Live-feed status refresh unavailable.");
  const runnerHint =
    runner.status === "fulfilled"
      ? runner.value.hint
      : "Auto-runner status unavailable.";
  const liveSummary = live.status === "fulfilled" ? live.value : null;
  const liveHint = liveSummary ? liveSummary.hint : "Live status unavailable.";
  const result: ExecutionActionResult = {
    statusLine:
      refreshWarnings.length > 0
        ? "Execution status refreshed with warnings."
        : "Execution status refreshed.",
    statusHint:
      refreshWarnings.length > 0
        ? `Execution status refreshed with warnings. ${refreshWarnings.join(" ")}`
        : "Execution status refreshed.",
    runnerStatusHint: runnerHint,
    liveStatusHint: liveHint,
  };
  if (runner.status === "fulfilled") {
    result.runnerActive = runner.value.active;
    result.runnerLifecycleState = runner.value.lifecycle;
  }
  if (liveSummary) {
    result.liveActive = liveSummary.active;
    result.liveLifecycleState =
      liveSummary.status as ExecutionLiveLifecycleState;
    result.liveSymbol = liveSummary.symbol;
    result.liveDelayed = liveSummary.delayed;
  }
  return result;
}

export async function getStoredExecutionCredentialStatus(): Promise<ExecutionStoredCredentialStatus> {
  return parseStoredExecutionCredentialStatus(
    await invokeCommand("alpaca_get_credential_status"),
  );
}

export async function storeExecutionCredentialsSecurely(
  apiKey: string,
  apiSecret: string,
): Promise<ExecutionStoredCredentialStatus> {
  return parseStoredExecutionCredentialStatus(
    await invokeCommand("alpaca_set_api_credentials", {
      req: { api_key: text(apiKey), api_secret: text(apiSecret) },
    }),
  );
}

export async function clearStoredExecutionCredentialsSecurely(): Promise<ExecutionStoredCredentialStatus> {
  return parseStoredExecutionCredentialStatus(
    await invokeCommand("alpaca_clear_api_credentials"),
  );
}

export async function migrateLegacyStoredExecutionKeysToSecureStorage(): Promise<ExecutionStoredCredentialStatus> {
  const legacy = loadPersistedExecutionKeys();
  const legacyKey = text(legacy.apiKey);
  const legacySecret = text(legacy.apiSecret);
  if (!legacyKey && !legacySecret) {
    return getStoredExecutionCredentialStatus();
  }
  if (!legacyKey || !legacySecret) {
    return {
      rememberAvailable: false,
      hasApiKey: !!legacyKey,
      hasApiSecret: !!legacySecret,
      hasCredentials: false,
      state: "incomplete",
      storageBackend: "",
      message: "Legacy stored execution credentials are incomplete.",
    };
  }
  const status = await storeExecutionCredentialsSecurely(
    legacyKey,
    legacySecret,
  );
  if (status.hasCredentials) {
    try {
      removeStorageKey(EXEC_KEYS_STORAGE_KEY);
    } catch {}
  }
  return status;
}

export async function clearKeys(): Promise<ExecutionActionResult> {
  let secureClearError: unknown = null;
  try {
    await clearStoredExecutionCredentialsSecurely();
  } catch (error) {
    secureClearError = error;
  }
  try {
    removeStorageKey(EXEC_KEYS_STORAGE_KEY);
  } catch {}
  if (secureClearError) {
    const detail = storeErrorText(secureClearError);
    throw new Error(detail || "Stored credential clear failed.");
  }
  return {
    statusLine: "Cleared execution API keys.",
    statusHint: "Cleared execution API keys.",
    outputPreview: "-",
    outputPreviewMeta: buildPreviewMeta(
      "alpaca_clear_api_credentials",
      "credential clear",
      Date.now(),
    ),
  };
}

export async function listSavedLiveSessions(): Promise<
  ExecutionSavedSessionSummary[]
> {
  const raw = await invokeCommand("list_live_session_artifacts");
  return parseBoundaryArray(raw)
    .map(parseSavedSessionSummary)
    .filter((row): row is ExecutionSavedSessionSummary => !!row);
}

export async function loadSavedLiveSessionDetail(
  path: string,
): Promise<ExecutionSavedSessionDetail> {
  return parseSavedSessionDetail(
    await invokeCommand("load_live_session_artifact_detail", {
      path: text(path),
    }),
  );
}
