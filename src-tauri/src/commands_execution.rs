use crate::broker_paper::{
    self, PaperBrokerMarketOrder, PaperBrokerOrder, PaperBrokerPosition, PaperOrderSide,
};
use crate::execution_policy::{
    evaluate_execution_policy, ExecutionPolicyDecision, ExecutionPolicyInput, ExecutionPolicyLimits,
};
use crate::execution_quantity::normalize_execution_quantity;
use crate::settings::{self, ExecutionRiskPolicySettings};
use crate::state::AppState;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(test)]
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tauri::Manager;
use vector_ta::utilities::data_loader::Candles;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveExecutionMode {
    Paper,
    Live,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionBrokerTarget {
    DryRun,
    LocalPaper,
    AlpacaPaper,
    AlpacaLive,
}

impl ExecutionBrokerTarget {
    fn is_alpaca(self) -> bool {
        matches!(
            self,
            ExecutionBrokerTarget::AlpacaPaper | ExecutionBrokerTarget::AlpacaLive
        )
    }

    fn is_broker_armed(self) -> bool {
        !matches!(self, ExecutionBrokerTarget::DryRun)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LiveRiskOverride {
    #[serde(default)]
    pub stop_loss_pct: Option<f64>,
    #[serde(default)]
    pub take_profit_pct: Option<f64>,
    #[serde(default)]
    pub max_order_notional: Option<f64>,
    #[serde(default)]
    pub max_position_notional: Option<f64>,
    #[serde(default)]
    pub allow_short: Option<bool>,
    #[serde(default)]
    pub require_bracket: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LiveStrategySpec {
    DoubleMa {
        req: ta_app_core::DoubleMaDrilldownRequest,
    },
    CompositeCpu {
        req: ta_app_core::CompositeCpuDrilldownRequest,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct ExecuteStrategySignalRequest {
    pub symbol: String,
    pub qty: f64,
    pub mode: LiveExecutionMode,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub execution_target: Option<ExecutionBrokerTarget>,
    #[serde(default)]
    pub risk: LiveRiskOverride,
    #[serde(default)]
    pub alpaca_api_key: String,
    #[serde(default)]
    pub alpaca_api_secret: String,
    #[serde(default)]
    pub param_source: String,
    #[serde(default)]
    pub source_run_artifact_path: String,
    #[serde(default)]
    pub source_run_config_fingerprint: String,
    #[serde(default)]
    pub source_run_route_provenance: Option<Value>,
    pub strategy: LiveStrategySpec,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TestAlpacaConnectionRequest {
    pub mode: LiveExecutionMode,
    #[serde(default)]
    pub alpaca_api_key: String,
    #[serde(default)]
    pub alpaca_api_secret: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestAlpacaConnectionResponse {
    pub mode: LiveExecutionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buying_power: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portfolio_value: Option<String>,
    #[serde(default)]
    pub fetched_at_unix_ms: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SetAlpacaApiCredentialsRequest {
    pub api_key: String,
    pub api_secret: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlpacaCredentialStorageState {
    Missing,
    Stored,
    Incomplete,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlpacaCredentialStorageStatus {
    pub remember_available: bool,
    pub has_api_key: bool,
    pub has_api_secret: bool,
    pub has_credentials: bool,
    pub state: AlpacaCredentialStorageState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetLiveExecutionSnapshotRequest {
    pub mode: LiveExecutionMode,
    #[serde(default)]
    pub alpaca_api_key: String,
    #[serde(default)]
    pub alpaca_api_secret: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub include_account: Option<bool>,
    #[serde(default)]
    pub include_orders: Option<bool>,
    #[serde(default)]
    pub include_position: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetLocalPaperExecutionSnapshotRequest {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResetLocalPaperBrokerResponse {
    pub snapshot: GetLiveExecutionSnapshotResponse,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionRiskPolicyResponse {
    pub policy: ExecutionRiskPolicySettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreviewExecutionPolicyDecisionResponse {
    pub symbol: String,
    pub mode: LiveExecutionMode,
    pub execution_target: ExecutionBrokerTarget,
    pub desired_position: i32,
    pub current_position_qty: f64,
    pub target_position_qty: f64,
    pub delta_qty: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_bar_ts: Option<i64>,
    pub last_close: f64,
    pub opening_new_position: bool,
    pub policy: ExecutionRiskPolicySettings,
    pub decision: ExecutionPolicyDecision,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionStrategySupportRow {
    pub strategy_kind: String,
    pub supported: bool,
    pub adapter: String,
    pub support_level: String,
    pub notes: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveOrderSnapshot {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
    pub symbol: String,
    pub side: String,
    pub status: String,
    pub order_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filled_qty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filled_avg_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filled_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canceled_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LivePositionSnapshot {
    pub symbol: String,
    pub qty: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_entry_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unrealized_pl: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unrealized_plpc: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetLiveExecutionSnapshotResponse {
    pub mode: LiveExecutionMode,
    pub symbol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buying_power: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portfolio_value: Option<String>,
    #[serde(default)]
    pub fetched_at_unix_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<LivePositionSnapshot>,
    #[serde(default)]
    pub orders: Vec<LiveOrderSnapshot>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LiveExecutionSafetyActionRequest {
    pub mode: LiveExecutionMode,
    #[serde(default)]
    pub alpaca_api_key: String,
    #[serde(default)]
    pub alpaca_api_secret: String,
    pub symbol: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CancelLiveExecutionOpenOrdersResponse {
    pub mode: LiveExecutionMode,
    pub symbol: String,
    pub open_order_count_before: usize,
    pub cancelled_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cancelled_order_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlattenLiveExecutionPositionResponse {
    pub mode: LiveExecutionMode,
    pub symbol: String,
    pub position_found: bool,
    pub open_order_count_before: usize,
    pub order_submitted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qty: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmergencyHaltLiveExecutionResponse {
    pub runner_stopped: bool,
    pub live_stopped: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub live_symbol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_runner: Option<LiveExecutionRunnerStatus>,
    pub requires_manual_review: bool,
    pub status_message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BrokerReconciliationEvent {
    pub provenance: String,
    pub status: String,
    pub message: String,
    pub at_unix_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broker_order_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filled_qty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filled_avg_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refreshed_position_qty: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_broker_review: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rearm_blocked_reason: Option<String>,
    pub terminal: bool,
    pub timed_out: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecuteStrategySignalResponse {
    pub symbol: String,
    pub mode: LiveExecutionMode,
    pub desired_position: i32,
    pub current_position_qty: f64,
    pub target_position_qty: f64,
    pub delta_qty: f64,
    pub order_submitted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_bar_ts: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_close: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub broker_reconciliation_events: Vec<BrokerReconciliationEvent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

const LIVE_EXECUTION_RUNNER_EVENT: &str = "live_execution_runner_event";
const RUNNER_CANCELLED_ERR: &str = "__live_execution_runner_cancelled__";
const ALPACA_CONNECT_TIMEOUT_MS: u64 = 3_000;
const ALPACA_REQUEST_TIMEOUT_MS: u64 = 12_000;
const ALPACA_IDLE_POOL_TIMEOUT_SECS: u64 = 90;
const ALPACA_ORDER_RECONCILE_POLL_ATTEMPTS: usize = 3;
const ALPACA_ORDER_RECONCILE_POLL_DELAY_MS: u64 = 250;
const LIVE_RUNNER_FEED_STALE_AFTER_MS: i64 = 180_000;
const ALPACA_KEYRING_API_KEY_USERNAME: &str = "alpaca_execution_api_key";
const ALPACA_KEYRING_API_SECRET_USERNAME: &str = "alpaca_execution_api_secret";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveExecutionRunnerConfig {
    pub symbol: String,
    pub qty: f64,
    pub mode: LiveExecutionMode,
    pub dry_run: bool,
    pub execution_target: ExecutionBrokerTarget,
    pub data_id: String,
    pub strategy_kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveExecutionRunnerStatus {
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<LiveExecutionRunnerConfig>,
    pub inflight: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_bar_open_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_triggered_at_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_at_unix_ms: Option<i64>,
    pub run_count: u64,
    pub skip_count: u64,
    pub error_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_order_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StopLiveExecutionRunnerResponse {
    pub stopped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior: Option<LiveExecutionRunnerStatus>,
    pub status: LiveExecutionRunnerStatus,
}

#[derive(Clone, Debug, Serialize)]
struct LiveExecutionRunnerEvent {
    pub status: String,
    pub message: String,
    pub symbol: String,
    pub at_unix_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_open_unix_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecuteStrategySignalResponse>,
}

#[derive(Clone, Debug)]
struct LiveExecutionRunnerRuntime {
    generation: u64,
    abort: RunnerAbortHandle,
    pending_bar: Option<LiveExecutionRunnerBar>,
    cfg: LiveExecutionRunnerInternalCfg,
    inflight: bool,
    started_at_unix_ms: i64,
    last_bar_open_unix_ms: Option<i64>,
    last_triggered_at_unix_ms: Option<i64>,
    last_completed_at_unix_ms: Option<i64>,
    run_count: u64,
    skip_count: u64,
    error_count: u64,
    last_action: Option<String>,
    last_order_id: Option<String>,
    last_error: Option<String>,
}

#[derive(Clone, Debug)]
struct LiveSessionAuditRuntime {
    path: String,
    symbol: String,
}

#[derive(Clone, Debug)]
struct LiveExecutionRunnerInternalCfg {
    req: ExecuteStrategySignalRequest,
    symbol: String,
    qty: f64,
    data_id: String,
    strategy_kind: String,
    app_local_data_dir: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct LiveExecutionRunnerBar {
    pub symbol: String,
    pub time_open_unix_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<f64>,
}

static LIVE_EXECUTION_RUNNER: OnceLock<Mutex<Option<LiveExecutionRunnerRuntime>>> = OnceLock::new();
static LIVE_EXECUTION_RUNNER_GEN: AtomicU64 = AtomicU64::new(1);
static LIVE_SESSION_AUDIT: OnceLock<Mutex<Option<LiveSessionAuditRuntime>>> = OnceLock::new();

#[derive(Clone, Debug)]
struct RunnerAbortHandle {
    cancelled: Arc<AtomicBool>,
}

impl RunnerAbortHandle {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn check(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err(RUNNER_CANCELLED_ERR.to_string())
        } else {
            Ok(())
        }
    }
}

struct SignalDecision {
    desired_position: i32,
    cfg: ta_strategies::double_ma::StrategyConfig,
    last_bar_ts: Option<i64>,
    last_close: f64,
}

fn open_position_from_trades(trades: &[ta_strategies::double_ma::Trade]) -> i32 {
    for t in trades.iter().rev() {
        if t.open {
            return t.direction.clamp(-1, 1);
        }
    }
    0
}

fn sanitize_pct(v: Option<f64>) -> Option<f64> {
    let x = v?;
    if x.is_finite() && x > 0.0 {
        Some(x.min(0.95))
    } else {
        None
    }
}

fn apply_risk_override(
    cfg: &mut ta_strategies::double_ma::StrategyConfig,
    risk: &LiveRiskOverride,
) {
    if risk.stop_loss_pct.is_some() {
        cfg.stop_loss_pct = sanitize_pct(risk.stop_loss_pct);
    }
    if risk.take_profit_pct.is_some() {
        cfg.take_profit_pct = sanitize_pct(risk.take_profit_pct);
    }
}

fn merge_execution_risk_policy(
    persisted: &ExecutionRiskPolicySettings,
    risk: &LiveRiskOverride,
) -> Result<ExecutionRiskPolicySettings, String> {
    fn request_cap(value: Option<f64>, field_name: &str) -> Result<Option<f64>, String> {
        if value.is_some_and(|v| !v.is_finite() || v <= 0.0) {
            return Err(format!(
                "risk.{field_name} must be a finite positive number."
            ));
        }
        Ok(value)
    }
    fn tightest_cap(
        persisted: Option<f64>,
        request: Option<f64>,
        field_name: &str,
    ) -> Result<Option<f64>, String> {
        let request = request_cap(request, field_name)?;
        Ok(match (persisted, request) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        })
    }

    Ok(ExecutionRiskPolicySettings {
        max_order_notional: tightest_cap(
            persisted.max_order_notional,
            risk.max_order_notional,
            "max_order_notional",
        )?,
        max_position_notional: tightest_cap(
            persisted.max_position_notional,
            risk.max_position_notional,
            "max_position_notional",
        )?,
        allow_short: match (persisted.allow_short, risk.allow_short) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), Some(true)) => Some(true),
            (Some(true), None) => Some(true),
            (None, Some(true)) => Some(true),
            (None, None) => None,
        },
        require_bracket: match (persisted.require_bracket, risk.require_bracket) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            (Some(false), None) => Some(false),
            (None, Some(false)) => Some(false),
            (None, None) => None,
        },
    })
}

fn execution_policy_limits_from_settings(
    policy: &ExecutionRiskPolicySettings,
) -> ExecutionPolicyLimits {
    ExecutionPolicyLimits {
        max_order_notional: policy.max_order_notional,
        max_position_notional: policy.max_position_notional,
        allow_short: policy.allow_short,
        require_bracket: policy.require_bracket,
    }
}

fn load_execution_risk_policy_for_base(
    app_local_data_dir: Option<&Path>,
) -> Result<ExecutionRiskPolicySettings, String> {
    if let Some(base) = app_local_data_dir {
        settings::configured_execution_risk_policy_settings(base)
    } else {
        Ok(ExecutionRiskPolicySettings::default())
    }
}

fn merged_execution_risk_policy_for_request(
    app_local_data_dir: Option<&Path>,
    risk: &LiveRiskOverride,
) -> Result<ExecutionRiskPolicySettings, String> {
    let persisted = load_execution_risk_policy_for_base(app_local_data_dir)?;
    merge_execution_risk_policy(&persisted, risk)
}

fn finite_last(xs: &[f64]) -> Option<f64> {
    xs.iter().rev().copied().find(|v| v.is_finite() && *v > 0.0)
}

enum StrategyExecutionAdapter<'a> {
    DoubleMa(&'a ta_app_core::DoubleMaDrilldownRequest),
    CompositeCpu(&'a ta_app_core::CompositeCpuDrilldownRequest),
}

impl<'a> StrategyExecutionAdapter<'a> {
    fn from_spec(spec: &'a LiveStrategySpec) -> Self {
        match spec {
            LiveStrategySpec::DoubleMa { req } => Self::DoubleMa(req),
            LiveStrategySpec::CompositeCpu { req } => Self::CompositeCpu(req),
        }
    }

    fn kind_label(&self) -> &'static str {
        match self {
            Self::DoubleMa(_) => "DoubleMa",
            Self::CompositeCpu(_) => "CompositeCpu",
        }
    }

    fn signal_decision(
        &self,
        candles: &vector_ta::utilities::data_loader::Candles,
        risk: &LiveRiskOverride,
    ) -> Result<SignalDecision, String> {
        match self {
            Self::DoubleMa(req) => {
                let mut dd = (*req).clone();
                apply_risk_override(&mut dd.strategy, risk);
                let curves = ta_app_core::compute_double_ma_drilldown_blocking_with_candles(
                    dd.clone(),
                    candles,
                )?;
                let desired_position = open_position_from_trades(&curves.trades);
                let last_close = finite_last(&candles.close)
                    .ok_or_else(|| "missing finite close".to_string())?;
                Ok(SignalDecision {
                    desired_position,
                    cfg: dd.strategy,
                    last_bar_ts: candles.timestamp.last().copied(),
                    last_close,
                })
            }
            Self::CompositeCpu(req) => {
                let mut dd = (*req).clone();
                apply_risk_override(&mut dd.strategy, risk);
                let curves = ta_app_core::compute_composite_cpu_drilldown_blocking_with_candles(
                    dd.clone(),
                    candles,
                )?;
                let desired_position = open_position_from_trades(&curves.curves.trades);
                let last_close = finite_last(&candles.close)
                    .ok_or_else(|| "missing finite close".to_string())?;
                Ok(SignalDecision {
                    desired_position,
                    cfg: dd.strategy,
                    last_bar_ts: candles.timestamp.last().copied(),
                    last_close,
                })
            }
        }
    }
}

fn signal_decision_from_request(
    req: &LiveStrategySpec,
    candles: &vector_ta::utilities::data_loader::Candles,
    risk: &LiveRiskOverride,
) -> Result<SignalDecision, String> {
    StrategyExecutionAdapter::from_spec(req).signal_decision(candles, risk)
}

fn strategy_data_id(spec: &LiveStrategySpec) -> &str {
    match spec {
        LiveStrategySpec::DoubleMa { req } => &req.data_id,
        LiveStrategySpec::CompositeCpu { req } => &req.data_id,
    }
}

fn execution_target_for_request(req: &ExecuteStrategySignalRequest) -> ExecutionBrokerTarget {
    req.execution_target.unwrap_or_else(|| {
        if req.dry_run {
            ExecutionBrokerTarget::DryRun
        } else {
            match req.mode {
                LiveExecutionMode::Paper => ExecutionBrokerTarget::AlpacaPaper,
                LiveExecutionMode::Live => ExecutionBrokerTarget::AlpacaLive,
            }
        }
    })
}

fn normalize_execution_target_flags(
    req: &mut ExecuteStrategySignalRequest,
) -> ExecutionBrokerTarget {
    let target = execution_target_for_request(req);
    match target {
        ExecutionBrokerTarget::DryRun => {
            req.dry_run = true;
        }
        ExecutionBrokerTarget::LocalPaper => {
            req.dry_run = false;
            req.mode = LiveExecutionMode::Paper;
        }
        ExecutionBrokerTarget::AlpacaPaper => {
            req.dry_run = false;
            req.mode = LiveExecutionMode::Paper;
        }
        ExecutionBrokerTarget::AlpacaLive => {
            req.dry_run = false;
            req.mode = LiveExecutionMode::Live;
        }
    }
    req.execution_target = Some(target);
    target
}

fn execution_target_label(target: ExecutionBrokerTarget) -> &'static str {
    match target {
        ExecutionBrokerTarget::DryRun => "dry_run",
        ExecutionBrokerTarget::LocalPaper => "local_paper",
        ExecutionBrokerTarget::AlpacaPaper => "alpaca_paper",
        ExecutionBrokerTarget::AlpacaLive => "alpaca_live",
    }
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn alpaca_client_order_id(
    req: &ExecuteStrategySignalRequest,
    symbol: &str,
    decision: &SignalDecision,
    target_position_qty: f64,
    delta_qty: f64,
) -> String {
    let payload = serde_json::json!({
        "symbol": symbol,
        "mode": match req.mode {
            LiveExecutionMode::Paper => "paper",
            LiveExecutionMode::Live => "live",
        },
        "execution_target": execution_target_label(execution_target_for_request(req)),
        "strategy_kind": strategy_kind_label(&req.strategy),
        "data_id": strategy_data_id(&req.strategy),
        "param_source": req.param_source.trim(),
        "source_run_config_fingerprint": req.source_run_config_fingerprint.trim(),
        "last_bar_ts": decision.last_bar_ts,
        "desired_position": decision.desired_position,
        "target_position_qty": format!("{target_position_qty:.4}"),
        "delta_qty": format!("{delta_qty:.4}"),
    });
    let text = serde_json::to_string(&payload).unwrap_or_default();
    format!("vg-{}", fnv1a64_hex(text.as_bytes()))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn live_execution_runner_slot() -> &'static Mutex<Option<LiveExecutionRunnerRuntime>> {
    LIVE_EXECUTION_RUNNER.get_or_init(|| Mutex::new(None))
}

fn live_session_audit_slot() -> &'static Mutex<Option<LiveSessionAuditRuntime>> {
    LIVE_SESSION_AUDIT.get_or_init(|| Mutex::new(None))
}

fn with_active_runner_generation<T>(
    generation: u64,
    f: impl FnOnce(&mut LiveExecutionRunnerRuntime) -> T,
) -> Option<T> {
    let Ok(mut slot) = live_execution_runner_slot().lock() else {
        return None;
    };
    let runtime = slot.as_mut()?;
    if runtime.generation != generation {
        return None;
    }
    Some(f(runtime))
}

fn active_live_session_path() -> Option<String> {
    let slot = live_session_audit_slot().lock().ok()?;
    slot.as_ref().map(|runtime| runtime.path.clone())
}

fn strategy_kind_label(spec: &LiveStrategySpec) -> String {
    StrategyExecutionAdapter::from_spec(spec)
        .kind_label()
        .to_string()
}

fn execution_strategy_support_matrix() -> Vec<ExecutionStrategySupportRow> {
    vec![
        ExecutionStrategySupportRow {
            strategy_kind: "DoubleMa".to_string(),
            supported: true,
            adapter: "double_ma_execution_adapter".to_string(),
            support_level: "signal_and_delta".to_string(),
            notes: "Uses Double-MA candidate-detail candles and params to derive latest desired position."
                .to_string(),
        },
        ExecutionStrategySupportRow {
            strategy_kind: "CompositeCpu".to_string(),
            supported: true,
            adapter: "composite_cpu_execution_adapter".to_string(),
            support_level: "signal_and_delta".to_string(),
            notes:
                "Uses CompositeCpu candidate-detail candles and params to derive latest desired position."
                    .to_string(),
        },
    ]
}

fn runner_config_view(cfg: &LiveExecutionRunnerInternalCfg) -> LiveExecutionRunnerConfig {
    LiveExecutionRunnerConfig {
        symbol: cfg.symbol.clone(),
        qty: cfg.qty,
        mode: cfg.req.mode,
        dry_run: cfg.req.dry_run,
        execution_target: execution_target_for_request(&cfg.req),
        data_id: cfg.data_id.clone(),
        strategy_kind: cfg.strategy_kind.clone(),
    }
}

fn runner_status_from_runtime(
    rt: Option<&LiveExecutionRunnerRuntime>,
) -> LiveExecutionRunnerStatus {
    if let Some(r) = rt {
        LiveExecutionRunnerStatus {
            active: true,
            config: Some(runner_config_view(&r.cfg)),
            inflight: r.inflight,
            started_at_unix_ms: Some(r.started_at_unix_ms),
            last_bar_open_unix_ms: r.last_bar_open_unix_ms,
            last_triggered_at_unix_ms: r.last_triggered_at_unix_ms,
            last_completed_at_unix_ms: r.last_completed_at_unix_ms,
            run_count: r.run_count,
            skip_count: r.skip_count,
            error_count: r.error_count,
            last_action: r.last_action.clone(),
            last_order_id: r.last_order_id.clone(),
            last_error: r.last_error.clone(),
        }
    } else {
        LiveExecutionRunnerStatus {
            active: false,
            config: None,
            inflight: false,
            started_at_unix_ms: None,
            last_bar_open_unix_ms: None,
            last_triggered_at_unix_ms: None,
            last_completed_at_unix_ms: None,
            run_count: 0,
            skip_count: 0,
            error_count: 0,
            last_action: None,
            last_order_id: None,
            last_error: None,
        }
    }
}

fn emit_live_execution_runner_event(
    app: &tauri::AppHandle,
    status: &str,
    message: impl Into<String>,
    symbol: impl Into<String>,
    bar_open_unix_ms: Option<i64>,
    execution: Option<ExecuteStrategySignalResponse>,
) {
    let payload = LiveExecutionRunnerEvent {
        status: status.to_string(),
        message: message.into(),
        symbol: symbol.into(),
        at_unix_ms: now_unix_ms(),
        bar_open_unix_ms,
        execution,
    };
    let provenance = match status {
        "signal_processed" => "signal_outcome",
        "order_submitted" => "submission_attempt",
        _ => "runner_lifecycle",
    };
    let event_value = serde_json::json!({
        "provenance": provenance,
        "status": payload.status,
        "message": payload.message,
        "symbol": payload.symbol,
        "at_unix_ms": payload.at_unix_ms,
        "bar_open_unix_ms": payload.bar_open_unix_ms,
        "action": payload.execution.as_ref().map(|row| row.action.clone()).unwrap_or_default(),
        "order_id": payload.execution.as_ref().and_then(|row| row.order_id.clone()).unwrap_or_default(),
        "client_order_id": payload.execution.as_ref().and_then(|row| row.client_order_id.clone()).unwrap_or_default(),
        "delta_qty": payload.execution.as_ref().map(|row| row.delta_qty),
        "order_submitted": payload.execution.as_ref().map(|row| row.order_submitted),
        "warnings": payload.execution.as_ref().map(|row| row.warnings.clone()).unwrap_or_default(),
    });
    let (reconciliation_symbol, reconciliation_action, reconciliation_events) = payload
        .execution
        .as_ref()
        .map(|row| {
            (
                payload.symbol.clone(),
                row.action.clone(),
                row.broker_reconciliation_events.clone(),
            )
        })
        .unwrap_or_else(|| ("".to_string(), "".to_string(), Vec::new()));
    let _ = app.emit(LIVE_EXECUTION_RUNNER_EVENT, payload);
    let _ = record_live_session_event(app, event_value);
    for event in reconciliation_events {
        let _ = record_live_session_event(
            app,
            broker_reconciliation_event_value(
                &reconciliation_symbol,
                &reconciliation_action,
                &event,
            ),
        );
    }
}

fn live_session_id(symbol: &str) -> String {
    format!(
        "live-session-{}-{}",
        now_unix_ms(),
        canonical_symbol_key(symbol).to_ascii_lowercase()
    )
}

fn build_live_session_seed(
    req: &ExecuteStrategySignalRequest,
    symbol: &str,
    qty: f64,
    strategy_kind: &str,
    data_id: &str,
) -> Value {
    serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "session_id": live_session_id(symbol),
        "session_state": "active",
        "created_at_unix_ms": now_unix_ms(),
        "mode": match req.mode {
            LiveExecutionMode::Paper => "paper",
            LiveExecutionMode::Live => "live",
        },
        "execution_target": execution_target_label(execution_target_for_request(req)),
        "symbol": symbol,
        "qty": qty,
        "dry_run": req.dry_run,
        "strategy_kind": strategy_kind,
        "data_id": data_id,
        "param_source": req.param_source.trim(),
        "source_run_artifact_path": req.source_run_artifact_path.trim(),
        "source_run_config_fingerprint": req.source_run_config_fingerprint.trim(),
        "source_run_route_provenance": req.source_run_route_provenance.clone(),
    })
}

fn record_live_session_event(app: &tauri::AppHandle, event: Value) -> Result<(), String> {
    let Some(path) = active_live_session_path() else {
        return Ok(());
    };
    super::run_artifacts::append_live_session_event_blocking(path, event, app.clone())
}

fn broker_reconciliation_event_value(
    symbol: &str,
    action: &str,
    event: &BrokerReconciliationEvent,
) -> Value {
    let mut value = serde_json::to_value(event).unwrap_or_else(|_| {
        serde_json::json!({
            "provenance": "broker_reconciliation",
            "status": "serialization_error",
            "message": "failed to serialize broker reconciliation event",
            "at_unix_ms": now_unix_ms(),
            "terminal": false,
            "timed_out": false,
        })
    });
    if let Some(obj) = value.as_object_mut() {
        obj.insert("symbol".to_string(), serde_json::json!(symbol));
        obj.insert("action".to_string(), serde_json::json!(action));
    }
    value
}

fn record_live_session_broker_snapshot(
    app: &tauri::AppHandle,
    snapshot: Value,
) -> Result<(), String> {
    let Some(path) = active_live_session_path() else {
        return Ok(());
    };
    super::run_artifacts::append_live_session_broker_snapshot_blocking(path, snapshot, app.clone())
}

fn finalize_active_live_session(app: &tauri::AppHandle, session_state: &str) -> Result<(), String> {
    let runtime = {
        let slot = live_session_audit_slot()
            .lock()
            .map_err(|_| "failed to lock live session audit state".to_string())?;
        let Some(runtime) = slot.as_ref() else {
            return Ok(());
        };
        runtime.clone()
    };
    super::run_artifacts::finalize_live_session_artifact_blocking(
        runtime.path.clone(),
        session_state.to_string(),
        app.clone(),
    )?;
    let mut slot = live_session_audit_slot()
        .lock()
        .map_err(|_| "failed to lock live session audit state".to_string())?;
    if slot
        .as_ref()
        .map(|active| active.path == runtime.path)
        .unwrap_or(false)
    {
        *slot = None;
    }
    Ok(())
}

pub(crate) fn record_live_session_interruption_and_finalize(
    app: &tauri::AppHandle,
    status: &str,
    message: impl Into<String>,
) -> Result<(), String> {
    let symbol = {
        let slot = live_session_audit_slot()
            .lock()
            .map_err(|_| "failed to lock live session audit state".to_string())?;
        let Some(runtime) = slot.as_ref() else {
            return Ok(());
        };
        runtime.symbol.clone()
    };
    record_live_session_event(
        app,
        serde_json::json!({
            "provenance": "interruption",
            "status": status,
            "message": message.into(),
            "symbol": symbol,
            "at_unix_ms": now_unix_ms(),
        }),
    )?;
    finalize_active_live_session(app, "interrupted")
}

fn capture_manual_execution_live_session(
    app: &tauri::AppHandle,
    req: &ExecuteStrategySignalRequest,
    symbol: &str,
    qty: f64,
    data_id: &str,
    strategy_kind: &str,
    response: &ExecuteStrategySignalResponse,
) -> Result<(), String> {
    let saved = super::run_artifacts::create_live_session_artifact_blocking(
        build_live_session_seed(req, symbol, qty, strategy_kind, data_id),
        app.clone(),
    )?;
    super::run_artifacts::append_live_session_event_blocking(
        saved.path.clone(),
        serde_json::json!({
            "provenance": "local_control",
            "status": "execute_strategy_signal_once",
            "message": format!("manual execution requested for {symbol}"),
            "symbol": symbol,
            "at_unix_ms": now_unix_ms(),
            "action": response.action.clone(),
            "order_id": response.order_id.clone(),
            "client_order_id": response.client_order_id.clone(),
            "delta_qty": response.delta_qty,
            "order_submitted": response.order_submitted,
            "warnings": response.warnings.clone(),
        }),
        app.clone(),
    )?;
    super::run_artifacts::append_live_session_event_blocking(
        saved.path.clone(),
        serde_json::json!({
            "provenance": if response.order_submitted { "submission_attempt" } else { "signal_outcome" },
            "status": if response.order_submitted { "order_submitted" } else { "signal_processed" },
            "message": if response.order_submitted {
                format!(
                    "manual execution submitted {} order{}",
                    response.action,
                    response.order_id
                        .as_deref()
                        .map(|id| format!(" (id={id})"))
                        .unwrap_or_default()
                )
            } else {
                format!("manual execution processed signal action={}", response.action)
            },
            "symbol": symbol,
            "at_unix_ms": now_unix_ms(),
            "action": response.action.clone(),
            "order_id": response.order_id.clone(),
            "client_order_id": response.client_order_id.clone(),
            "delta_qty": response.delta_qty,
            "order_submitted": response.order_submitted,
            "warnings": response.warnings.clone(),
        }),
        app.clone(),
    )?;
    for event in &response.broker_reconciliation_events {
        super::run_artifacts::append_live_session_event_blocking(
            saved.path.clone(),
            broker_reconciliation_event_value(symbol, &response.action, event),
            app.clone(),
        )?;
    }
    super::run_artifacts::finalize_live_session_artifact_blocking(
        saved.path,
        "completed".to_string(),
        app.clone(),
    )
}

pub(crate) fn canonical_symbol_key(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn normalize_execute_request(
    mut req: ExecuteStrategySignalRequest,
) -> Result<(ExecuteStrategySignalRequest, String, f64), String> {
    normalize_execution_target_flags(&mut req);
    let symbol = canonical_symbol_key(&req.symbol);
    if symbol.is_empty() {
        return Err("symbol is required".to_string());
    }
    if !req.qty.is_finite() || req.qty <= 0.0 {
        return Err("qty must be finite and > 0".to_string());
    }
    let qty = normalize_execution_quantity(req.qty)?;
    req.symbol = symbol.clone();
    req.qty = qty;
    Ok((req, symbol, qty))
}

fn delta_qty_is_hold(delta_qty: f64) -> bool {
    delta_qty.abs() < 1e-6
}

fn should_attach_opening_bracket(current_position_qty: f64, target_position_qty: f64) -> bool {
    current_position_qty.abs() < 1e-6 && target_position_qty.abs() > 1e-6
}

fn local_paper_bracket_child_orders_required_but_unsupported(
    policy: &ExecutionRiskPolicySettings,
    opening_new_position: bool,
) -> bool {
    opening_new_position
        && policy
            .require_bracket
            .is_some_and(|require_bracket| require_bracket)
}

fn validate_live_runner_stream_preconditions(
    stream: Option<&crate::state::LiveCandleStreamInfo>,
    requested_symbol: &str,
    execution_target: ExecutionBrokerTarget,
    now_unix_ms: i64,
) -> Result<(), String> {
    let Some(stream) = stream.filter(|stream| stream.active) else {
        return Err(
            "Start live candles first. Auto-runner triggers on Polygon live bars.".to_string(),
        );
    };
    if !symbol_matches(&stream.symbol, requested_symbol) {
        let live_symbol = canonical_symbol_key(&stream.symbol);
        let requested_symbol = canonical_symbol_key(requested_symbol);
        return Err(format!(
            "Live candles are active for {live_symbol}, not {requested_symbol}."
        ));
    }
    if matches!(execution_target, ExecutionBrokerTarget::AlpacaLive) && stream.delayed {
        return Err(
            "Live Alpaca automation requires a real-time Polygon stream. Turn delayed data off before arming the broker."
                .to_string(),
        );
    }
    let Some(received_at_unix_ms) = stream.last_accepted_bar_received_at_unix_ms else {
        return Err(
            "Live candles are active but no accepted bar has been received yet. Wait for the next Polygon minute bar before arming the auto-runner."
                .to_string(),
        );
    };
    if received_at_unix_ms <= 0 {
        return Err(
            "Live candles are active but their last accepted bar timestamp is invalid. Restart live candles before arming the auto-runner."
                .to_string(),
        );
    }
    let age_ms = now_unix_ms.saturating_sub(received_at_unix_ms);
    if now_unix_ms > received_at_unix_ms && age_ms > LIVE_RUNNER_FEED_STALE_AFTER_MS {
        return Err(format!(
            "Live candle feed is stale: last accepted bar was {age_ms} ms ago. Wait for a fresh Polygon bar or restart live candles before arming the auto-runner."
        ));
    }
    Ok(())
}

fn base_url(mode: LiveExecutionMode) -> &'static str {
    match mode {
        LiveExecutionMode::Paper => "https://paper-api.alpaca.markets",
        LiveExecutionMode::Live => "https://api.alpaca.markets",
    }
}

fn build_alpaca_headers(api_key: &str, api_secret: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    let key_name = HeaderName::from_static("apca-api-key-id");
    let sec_name = HeaderName::from_static("apca-api-secret-key");
    headers.insert(
        key_name,
        HeaderValue::from_str(api_key).map_err(|e| format!("invalid API key header: {e}"))?,
    );
    headers.insert(
        sec_name,
        HeaderValue::from_str(api_secret).map_err(|e| format!("invalid API secret header: {e}"))?,
    );
    Ok(headers)
}

fn alpaca_keyring_service_name(app_identifier: &str) -> String {
    let id = app_identifier.trim();
    if id.is_empty() {
        "vectorgrid.alpaca.execution".to_string()
    } else {
        format!("{id}.alpaca.execution")
    }
}

fn build_alpaca_credential_storage_status(
    remember_available: bool,
    has_api_key: bool,
    has_api_secret: bool,
    message: Option<String>,
) -> AlpacaCredentialStorageStatus {
    let state = if !remember_available {
        AlpacaCredentialStorageState::Unavailable
    } else if has_api_key && has_api_secret {
        AlpacaCredentialStorageState::Stored
    } else if has_api_key || has_api_secret {
        AlpacaCredentialStorageState::Incomplete
    } else {
        AlpacaCredentialStorageState::Missing
    };
    AlpacaCredentialStorageStatus {
        remember_available,
        has_api_key,
        has_api_secret,
        has_credentials: has_api_key && has_api_secret,
        state,
        storage_backend: remember_available.then(|| "keyring".to_string()),
        message,
    }
}

fn read_keyring_trimmed(entry: &keyring::Entry) -> Option<String> {
    entry.get_password().ok().and_then(|raw| {
        let trimmed = raw.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

fn alpaca_keyring_entries(
    app_identifier: &str,
) -> Result<(keyring::Entry, keyring::Entry), String> {
    let service = alpaca_keyring_service_name(app_identifier);
    let key_entry = keyring::Entry::new(&service, ALPACA_KEYRING_API_KEY_USERNAME)
        .map_err(|e| format!("failed to initialize Alpaca API key secure storage: {e}"))?;
    let secret_entry = keyring::Entry::new(&service, ALPACA_KEYRING_API_SECRET_USERNAME)
        .map_err(|e| format!("failed to initialize Alpaca API secret secure storage: {e}"))?;
    Ok((key_entry, secret_entry))
}

#[cfg(test)]
fn test_alpaca_credential_store() -> &'static Mutex<HashMap<String, (String, String)>> {
    static TEST_STORE: OnceLock<Mutex<HashMap<String, (String, String)>>> = OnceLock::new();
    TEST_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
fn read_test_alpaca_credentials(app_identifier: &str) -> Option<(String, String)> {
    test_alpaca_credential_store()
        .lock()
        .ok()
        .and_then(|map| map.get(app_identifier).cloned())
}

#[cfg(test)]
fn write_test_alpaca_credentials(app_identifier: &str, api_key: &str, api_secret: &str) {
    if let Ok(mut map) = test_alpaca_credential_store().lock() {
        map.insert(
            app_identifier.to_string(),
            (api_key.to_string(), api_secret.to_string()),
        );
    }
}

#[cfg(test)]
fn clear_test_alpaca_credentials(app_identifier: &str) {
    if let Ok(mut map) = test_alpaca_credential_store().lock() {
        map.remove(app_identifier);
    }
}

fn alpaca_credential_storage_status_for_app(app_identifier: &str) -> AlpacaCredentialStorageStatus {
    #[cfg(test)]
    if let Some((key, secret)) = read_test_alpaca_credentials(app_identifier) {
        return build_alpaca_credential_storage_status(
            true,
            !key.is_empty(),
            !secret.is_empty(),
            None,
        );
    }
    let Ok((key_entry, secret_entry)) = alpaca_keyring_entries(app_identifier) else {
        return build_alpaca_credential_storage_status(
            false,
            false,
            false,
            Some("Secure credential storage is unavailable on this system.".to_string()),
        );
    };
    let has_api_key = read_keyring_trimmed(&key_entry).is_some();
    let has_api_secret = read_keyring_trimmed(&secret_entry).is_some();
    build_alpaca_credential_storage_status(true, has_api_key, has_api_secret, None)
}

fn store_alpaca_api_credentials(
    app_identifier: &str,
    api_key: &str,
    api_secret: &str,
) -> Result<AlpacaCredentialStorageStatus, String> {
    let key = api_key.trim();
    let secret = api_secret.trim();
    if key.is_empty() || secret.is_empty() {
        return Err("Both Alpaca API key and secret are required.".to_string());
    }
    #[cfg(test)]
    {
        write_test_alpaca_credentials(app_identifier, key, secret);
        Ok(build_alpaca_credential_storage_status(
            true, true, true, None,
        ))
    }
    #[cfg(not(test))]
    {
        let (key_entry, secret_entry) = alpaca_keyring_entries(app_identifier)?;
        key_entry
            .set_password(key)
            .map_err(|e| format!("failed to store Alpaca API key securely: {e}"))?;
        if let Err(error) = secret_entry.set_password(secret) {
            let _ = key_entry.delete_credential();
            return Err(format!(
                "failed to store Alpaca API secret securely: {error}"
            ));
        }
        let status = alpaca_credential_storage_status_for_app(app_identifier);
        if !status.has_credentials {
            return Err("secure storage write could not be verified".to_string());
        }
        Ok(status)
    }
}

fn clear_alpaca_api_credentials_for_app(app_identifier: &str) -> AlpacaCredentialStorageStatus {
    #[cfg(test)]
    {
        clear_test_alpaca_credentials(app_identifier);
        build_alpaca_credential_storage_status(true, false, false, None)
    }
    #[cfg(not(test))]
    {
        if let Ok((key_entry, secret_entry)) = alpaca_keyring_entries(app_identifier) {
            let _ = key_entry.delete_credential();
            let _ = secret_entry.delete_credential();
        }
        alpaca_credential_storage_status_for_app(app_identifier)
    }
}

fn normalize_alpaca_typed_override(
    api_key: &str,
    api_secret: &str,
) -> Result<Option<(String, String)>, String> {
    let key = api_key.trim();
    let secret = api_secret.trim();
    if key.is_empty() && secret.is_empty() {
        return Ok(None);
    }
    if key.is_empty() || secret.is_empty() {
        return Err(
            "Both alpaca_api_key and alpaca_api_secret are required when using typed overrides."
                .to_string(),
        );
    }
    Ok(Some((key.to_string(), secret.to_string())))
}

fn resolve_alpaca_credentials_for_app(
    app_identifier: &str,
    api_key: &str,
    api_secret: &str,
) -> Result<Option<(String, String)>, String> {
    if let Some(override_pair) = normalize_alpaca_typed_override(api_key, api_secret)? {
        return Ok(Some(override_pair));
    }
    #[cfg(test)]
    if let Some((key, secret)) = read_test_alpaca_credentials(app_identifier) {
        return Ok(Some((key, secret)));
    }
    let Ok((key_entry, secret_entry)) = alpaca_keyring_entries(app_identifier) else {
        return Ok(None);
    };
    let stored_key = read_keyring_trimmed(&key_entry);
    let stored_secret = read_keyring_trimmed(&secret_entry);
    match (stored_key, stored_secret) {
        (Some(key), Some(secret)) => Ok(Some((key, secret))),
        (None, None) => Ok(None),
        _ => Err(
            "Stored Alpaca credentials are incomplete. Clear them and save a complete key pair."
                .to_string(),
        ),
    }
}

fn alpaca_client() -> Result<&'static Client, String> {
    static ALPACA_CLIENT: OnceLock<Client> = OnceLock::new();
    if let Some(client) = ALPACA_CLIENT.get() {
        return Ok(client);
    }

    let client = Client::builder()
        .user_agent("VectorGrid/1.0")
        .connect_timeout(Duration::from_millis(ALPACA_CONNECT_TIMEOUT_MS))
        .timeout(Duration::from_millis(ALPACA_REQUEST_TIMEOUT_MS))
        .pool_idle_timeout(Duration::from_secs(ALPACA_IDLE_POOL_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("failed to build Alpaca HTTP client: {e}"))?;
    let _ = ALPACA_CLIENT.set(client);
    ALPACA_CLIENT
        .get()
        .ok_or_else(|| "failed to initialize Alpaca HTTP client".to_string())
}

fn fetch_alpaca_position_qty(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
) -> Result<f64, String> {
    let headers = build_alpaca_headers(api_key, api_secret)?;
    let url = format!("{}/v2/positions/{}", base_url(mode), symbol);
    let resp = client
        .get(url)
        .headers(headers)
        .send()
        .map_err(|e| format!("failed to fetch Alpaca position: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(0.0);
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Alpaca position request failed ({status}): {body}"));
    }
    let v: serde_json::Value = resp
        .json()
        .map_err(|e| format!("failed to decode Alpaca position response: {e}"))?;
    let qty_s = v
        .get("qty")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "Alpaca response missing qty".to_string())?;
    f64::from_str(qty_s).map_err(|e| format!("invalid Alpaca qty '{qty_s}': {e}"))
}

fn build_alpaca_clock_url(mode: LiveExecutionMode) -> String {
    format!("{}/v2/clock", base_url(mode))
}

fn fetch_alpaca_account(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
) -> Result<serde_json::Value, String> {
    let headers = build_alpaca_headers(api_key, api_secret)?;
    let url = format!("{}/v2/account", base_url(mode));
    let resp = client
        .get(url)
        .headers(headers)
        .send()
        .map_err(|e| format!("failed to fetch Alpaca account: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Alpaca account request failed ({status}): {body}"));
    }
    resp.json()
        .map_err(|e| format!("failed to decode Alpaca account response: {e}"))
}

fn fetch_alpaca_clock(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
) -> Result<serde_json::Value, String> {
    let headers = build_alpaca_headers(api_key, api_secret)?;
    let resp = client
        .get(build_alpaca_clock_url(mode))
        .headers(headers)
        .send()
        .map_err(|e| format!("failed to fetch Alpaca market clock: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!(
            "Alpaca market clock request failed ({status}): {body}"
        ));
    }
    resp.json()
        .map_err(|e| format!("failed to decode Alpaca market clock response: {e}"))
}

fn validate_alpaca_account_tradeable(account: &serde_json::Value) -> Result<(), String> {
    let status = json_field_as_string(account, "status")
        .unwrap_or_else(|| "unknown".to_string())
        .to_ascii_uppercase();
    if status != "ACTIVE" {
        return Err(format!(
            "Alpaca account status is {status}; broker submission requires ACTIVE."
        ));
    }
    if json_field_as_bool(account, "trading_blocked").unwrap_or(false) {
        return Err("Alpaca account is trading_blocked; broker submission is blocked.".to_string());
    }
    if json_field_as_bool(account, "account_blocked").unwrap_or(false) {
        return Err("Alpaca account is account_blocked; broker submission is blocked.".to_string());
    }
    Ok(())
}

fn validate_alpaca_market_clock_open(clock: &serde_json::Value) -> Result<(), String> {
    if json_field_as_bool(clock, "is_open").unwrap_or(false) {
        Ok(())
    } else {
        let next_open = json_field_as_string(clock, "next_open")
            .map(|value| format!(" next_open={value}"))
            .unwrap_or_default();
        Err(format!(
            "Alpaca market clock is closed; live broker submission requires an open regular session.{next_open}"
        ))
    }
}

fn validate_live_execution_source_context(
    req: &ExecuteStrategySignalRequest,
    execution_target: ExecutionBrokerTarget,
) -> Result<(), String> {
    if matches!(execution_target, ExecutionBrokerTarget::AlpacaLive)
        && req.source_run_config_fingerprint.trim().is_empty()
    {
        return Err(
            "Alpaca live execution requires a validation/source-run config fingerprint before broker submission."
                .to_string(),
        );
    }
    Ok(())
}

fn validate_execution_source_preconditions(
    req: &ExecuteStrategySignalRequest,
    execution_target: ExecutionBrokerTarget,
) -> Result<(), String> {
    validate_live_execution_source_context(req, execution_target)
}

fn validate_alpaca_live_broker_preflight(
    client: &Client,
    req: &ExecuteStrategySignalRequest,
    execution_target: ExecutionBrokerTarget,
) -> Result<(), String> {
    if !execution_target.is_alpaca() {
        return Ok(());
    }
    validate_alpaca_account_tradeable(&fetch_alpaca_account(
        client,
        req.mode,
        req.alpaca_api_key.trim(),
        req.alpaca_api_secret.trim(),
    )?)?;
    if matches!(execution_target, ExecutionBrokerTarget::AlpacaLive) {
        validate_alpaca_market_clock_open(&fetch_alpaca_clock(
            client,
            req.mode,
            req.alpaca_api_key.trim(),
            req.alpaca_api_secret.trim(),
        )?)?;
    }
    Ok(())
}

fn json_field_as_string(v: &serde_json::Value, field: &str) -> Option<String> {
    match v.get(field) {
        Some(serde_json::Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        Some(serde_json::Value::Bool(b)) => Some(if *b { "true" } else { "false" }.to_string()),
        _ => None,
    }
}

fn json_field_as_bool(v: &serde_json::Value, field: &str) -> Option<bool> {
    match v.get(field) {
        Some(serde_json::Value::Bool(b)) => Some(*b),
        Some(serde_json::Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        Some(serde_json::Value::Number(n)) => n.as_i64().map(|v| v != 0),
        _ => None,
    }
}

fn json_field_as_f64(v: &serde_json::Value, field: &str) -> Option<f64> {
    match v.get(field) {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                f64::from_str(t).ok()
            }
        }
        _ => None,
    }
}

pub(crate) fn symbol_matches(order_symbol: &str, target_symbol: &str) -> bool {
    if target_symbol.is_empty() {
        return true;
    }
    canonical_symbol_key(order_symbol) == canonical_symbol_key(target_symbol)
}

fn update_pending_bar(
    current: &mut Option<LiveExecutionRunnerBar>,
    incoming: &LiveExecutionRunnerBar,
) -> bool {
    match current.as_ref() {
        Some(existing) if existing.time_open_unix_ms >= incoming.time_open_unix_ms => false,
        _ => {
            *current = Some(incoming.clone());
            true
        }
    }
}

fn is_runner_cancelled_error(err: &str) -> bool {
    err.trim() == RUNNER_CANCELLED_ERR
}

fn fetch_alpaca_orders(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    limit: u32,
) -> Result<Vec<serde_json::Value>, String> {
    fetch_alpaca_orders_with_filters(client, mode, api_key, api_secret, "all", limit, None)
}

fn build_alpaca_orders_url(
    mode: LiveExecutionMode,
    status: &str,
    limit: u32,
    symbol: Option<&str>,
) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(&format!("{}/v2/orders", base_url(mode)))
        .map_err(|e| format!("failed to build Alpaca orders url: {e}"))?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("status", status.trim());
        query.append_pair("direction", "desc");
        query.append_pair("limit", &limit.clamp(1, 500).to_string());
        if let Some(symbol) = symbol.map(str::trim).filter(|value| !value.is_empty()) {
            query.append_pair("symbols", symbol);
        }
    }
    Ok(url)
}

fn fetch_alpaca_orders_with_filters(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    status: &str,
    limit: u32,
    symbol: Option<&str>,
) -> Result<Vec<serde_json::Value>, String> {
    let headers = build_alpaca_headers(api_key, api_secret)?;
    let url = build_alpaca_orders_url(mode, status, limit, symbol)?;
    let resp = client
        .get(url)
        .headers(headers)
        .send()
        .map_err(|e| format!("failed to fetch Alpaca orders: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Alpaca orders request failed ({status}): {body}"));
    }
    let v: serde_json::Value = resp
        .json()
        .map_err(|e| format!("failed to decode Alpaca orders response: {e}"))?;
    let arr = v
        .as_array()
        .ok_or_else(|| "Alpaca orders response must be an array".to_string())?;
    Ok(arr.clone())
}

fn build_alpaca_order_by_client_id_url(
    mode: LiveExecutionMode,
    client_order_id: &str,
) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(&format!("{}/v2/orders:by_client_order_id", base_url(mode)))
        .map_err(|e| format!("failed to build Alpaca client-order lookup url: {e}"))?;
    url.query_pairs_mut()
        .append_pair("client_order_id", client_order_id.trim());
    Ok(url)
}

fn fetch_alpaca_order_by_client_order_id(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    client_order_id: &str,
) -> Result<Option<serde_json::Value>, String> {
    let client_order_id = client_order_id.trim();
    if client_order_id.is_empty() {
        return Ok(None);
    }
    let headers = build_alpaca_headers(api_key, api_secret)?;
    let url = build_alpaca_order_by_client_id_url(mode, client_order_id)?;
    let resp = client
        .get(url)
        .headers(headers)
        .send()
        .map_err(|e| format!("failed to fetch Alpaca order by client_order_id: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!(
            "Alpaca client-order lookup failed ({status}): {body}"
        ));
    }
    resp.json()
        .map(Some)
        .map_err(|e| format!("failed to decode Alpaca client-order response: {e}"))
}

fn fetch_alpaca_open_orders_for_symbol(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
) -> Result<Vec<serde_json::Value>, String> {
    fetch_alpaca_orders_with_filters(client, mode, api_key, api_secret, "open", 500, Some(symbol))
}

fn fetch_alpaca_positions(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let headers = build_alpaca_headers(api_key, api_secret)?;
    let url = format!("{}/v2/positions", base_url(mode));
    let resp = client
        .get(url)
        .headers(headers)
        .send()
        .map_err(|e| format!("failed to fetch Alpaca positions: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!(
            "Alpaca positions request failed ({status}): {body}"
        ));
    }
    let v: serde_json::Value = resp
        .json()
        .map_err(|e| format!("failed to decode Alpaca positions response: {e}"))?;
    let arr = v
        .as_array()
        .ok_or_else(|| "Alpaca positions response must be an array".to_string())?;
    Ok(arr.clone())
}

fn fetch_alpaca_position(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
) -> Result<Option<serde_json::Value>, String> {
    let headers = build_alpaca_headers(api_key, api_secret)?;
    let url = format!("{}/v2/positions/{}", base_url(mode), symbol);
    let resp = client
        .get(url)
        .headers(headers)
        .send()
        .map_err(|e| format!("failed to fetch Alpaca position: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Alpaca position request failed ({status}): {body}"));
    }
    resp.json()
        .map(Some)
        .map_err(|e| format!("failed to decode Alpaca position response: {e}"))
}

fn cancel_alpaca_order(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    order_id: &str,
) -> Result<(), String> {
    let headers = build_alpaca_headers(api_key, api_secret)?;
    let url = format!("{}/v2/orders/{}", base_url(mode), order_id);
    let resp = client
        .delete(url)
        .headers(headers)
        .send()
        .map_err(|e| format!("failed to cancel Alpaca order {order_id}: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(());
    }
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Alpaca order cancel failed ({status}): {body}"));
    }
    Ok(())
}

fn normalize_order_snapshot(v: &serde_json::Value) -> Option<LiveOrderSnapshot> {
    let id = json_field_as_string(v, "id")?;
    let symbol = json_field_as_string(v, "symbol").unwrap_or_default();
    let side = json_field_as_string(v, "side").unwrap_or_else(|| "unknown".to_string());
    let status = json_field_as_string(v, "status").unwrap_or_else(|| "unknown".to_string());
    let order_type = json_field_as_string(v, "type").unwrap_or_else(|| "unknown".to_string());
    Some(LiveOrderSnapshot {
        id,
        client_order_id: json_field_as_string(v, "client_order_id"),
        symbol,
        side,
        status,
        order_type,
        qty: json_field_as_f64(v, "qty"),
        filled_qty: json_field_as_f64(v, "filled_qty"),
        filled_avg_price: json_field_as_f64(v, "filled_avg_price"),
        limit_price: json_field_as_f64(v, "limit_price"),
        stop_price: json_field_as_f64(v, "stop_price"),
        submitted_at: json_field_as_string(v, "submitted_at"),
        filled_at: json_field_as_string(v, "filled_at"),
        updated_at: json_field_as_string(v, "updated_at"),
        canceled_at: json_field_as_string(v, "canceled_at"),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BrokerOrderStatusClass {
    OpenCancelable,
    OpenNonCancelable,
    PartialFill,
    DoneForDayReviewRequired,
    TerminalFilled,
    TerminalCanceled,
    TerminalExpired,
    TerminalRejected,
    TerminalReplaced,
    SuspendedReviewRequired,
    UnknownReviewRequired,
}

fn classify_broker_order_status(status: &str) -> BrokerOrderStatusClass {
    match status.trim().to_ascii_lowercase().as_str() {
        "new" | "accepted" | "pending_new" | "accepted_for_bidding" => {
            BrokerOrderStatusClass::OpenCancelable
        }
        "pending_cancel" | "pending_replace" => BrokerOrderStatusClass::OpenNonCancelable,
        "partially_filled" => BrokerOrderStatusClass::PartialFill,
        "filled" => BrokerOrderStatusClass::TerminalFilled,
        "canceled" => BrokerOrderStatusClass::TerminalCanceled,
        "expired" => BrokerOrderStatusClass::TerminalExpired,
        "rejected" => BrokerOrderStatusClass::TerminalRejected,
        "replaced" => BrokerOrderStatusClass::TerminalReplaced,
        "done_for_day" => BrokerOrderStatusClass::DoneForDayReviewRequired,
        "stopped" | "suspended" | "calculated" | "held" => {
            BrokerOrderStatusClass::SuspendedReviewRequired
        }
        _ => BrokerOrderStatusClass::UnknownReviewRequired,
    }
}

fn broker_status_class_requires_review(class: BrokerOrderStatusClass) -> bool {
    matches!(
        class,
        BrokerOrderStatusClass::PartialFill
            | BrokerOrderStatusClass::DoneForDayReviewRequired
            | BrokerOrderStatusClass::TerminalFilled
            | BrokerOrderStatusClass::TerminalCanceled
            | BrokerOrderStatusClass::TerminalExpired
            | BrokerOrderStatusClass::TerminalRejected
            | BrokerOrderStatusClass::TerminalReplaced
            | BrokerOrderStatusClass::SuspendedReviewRequired
            | BrokerOrderStatusClass::UnknownReviewRequired
    )
}

fn broker_status_review_reason(class: BrokerOrderStatusClass, status: &str) -> Option<String> {
    let normalized = status.trim();
    match class {
        BrokerOrderStatusClass::PartialFill => {
            Some("partial_fill_requires_broker_review_before_rearm".to_string())
        }
        BrokerOrderStatusClass::DoneForDayReviewRequired => {
            Some("done_for_day_requires_broker_review_before_rearm".to_string())
        }
        BrokerOrderStatusClass::TerminalFilled => {
            Some("terminal_filled_requires_position_refresh_before_rearm".to_string())
        }
        BrokerOrderStatusClass::TerminalCanceled => {
            Some("terminal_canceled_requires_broker_review_before_rearm".to_string())
        }
        BrokerOrderStatusClass::TerminalExpired => {
            Some("terminal_expired_requires_broker_review_before_rearm".to_string())
        }
        BrokerOrderStatusClass::TerminalRejected => {
            Some("terminal_rejected_requires_broker_review_before_rearm".to_string())
        }
        BrokerOrderStatusClass::TerminalReplaced => {
            Some("terminal_replaced_requires_broker_review_before_rearm".to_string())
        }
        BrokerOrderStatusClass::SuspendedReviewRequired => Some(format!(
            "broker_status_{}_requires_review_before_rearm",
            normalized.to_ascii_lowercase()
        )),
        BrokerOrderStatusClass::UnknownReviewRequired => {
            Some("broker_unknown_status_requires_review_before_rearm".to_string())
        }
        BrokerOrderStatusClass::OpenCancelable | BrokerOrderStatusClass::OpenNonCancelable => None,
    }
}

fn is_broker_cancelable_order_status(status: &str) -> bool {
    matches!(
        classify_broker_order_status(status),
        BrokerOrderStatusClass::OpenCancelable | BrokerOrderStatusClass::PartialFill
    )
}

fn is_broker_open_order_status(status: &str) -> bool {
    matches!(
        classify_broker_order_status(status),
        BrokerOrderStatusClass::OpenCancelable
            | BrokerOrderStatusClass::OpenNonCancelable
            | BrokerOrderStatusClass::PartialFill
            | BrokerOrderStatusClass::SuspendedReviewRequired
    )
}

fn is_broker_terminal_order_status(status: &str) -> bool {
    matches!(
        classify_broker_order_status(status),
        BrokerOrderStatusClass::TerminalFilled
            | BrokerOrderStatusClass::TerminalCanceled
            | BrokerOrderStatusClass::TerminalExpired
            | BrokerOrderStatusClass::TerminalRejected
            | BrokerOrderStatusClass::TerminalReplaced
    )
}

fn alpaca_intent_status_for_order_snapshot(order: &LiveOrderSnapshot) -> String {
    match classify_broker_order_status(&order.status) {
        BrokerOrderStatusClass::OpenCancelable | BrokerOrderStatusClass::OpenNonCancelable => {
            "broker_open".to_string()
        }
        BrokerOrderStatusClass::PartialFill => "partial_fill".to_string(),
        BrokerOrderStatusClass::DoneForDayReviewRequired => {
            "done_for_day_review_required".to_string()
        }
        BrokerOrderStatusClass::TerminalFilled => "terminal_filled_review_required".to_string(),
        BrokerOrderStatusClass::TerminalCanceled => "terminal_canceled_review_required".to_string(),
        BrokerOrderStatusClass::TerminalExpired => "terminal_expired_review_required".to_string(),
        BrokerOrderStatusClass::TerminalRejected => "terminal_rejected_review_required".to_string(),
        BrokerOrderStatusClass::TerminalReplaced => "terminal_replaced_review_required".to_string(),
        BrokerOrderStatusClass::SuspendedReviewRequired => "suspended_review_required".to_string(),
        BrokerOrderStatusClass::UnknownReviewRequired => "broker_unknown".to_string(),
    }
}

fn broker_reconciliation_event_from_snapshot(
    order: &LiveOrderSnapshot,
    terminal: bool,
    timed_out: bool,
) -> BrokerReconciliationEvent {
    let status_class = classify_broker_order_status(&order.status);
    let status = alpaca_intent_status_for_order_snapshot(order);
    let requires_review = timed_out || broker_status_class_requires_review(status_class);
    let message = if terminal {
        format!(
            "Alpaca order reconciled to terminal status {}.",
            order.status
        )
    } else if timed_out {
        format!(
            "Alpaca order is still {} after post-submit reconciliation.",
            order.status
        )
    } else {
        format!("Alpaca order reconciled to status {}.", order.status)
    };
    BrokerReconciliationEvent {
        provenance: "broker_reconciliation".to_string(),
        status,
        message,
        at_unix_ms: now_unix_ms(),
        order_id: Some(order.id.clone()),
        client_order_id: order.client_order_id.clone(),
        broker_order_status: Some(order.status.clone()),
        filled_qty: order.filled_qty,
        filled_avg_price: order.filled_avg_price,
        refreshed_position_qty: None,
        requires_broker_review: requires_review.then_some(true),
        rearm_blocked_reason: broker_status_review_reason(status_class, &order.status).or_else(
            || {
                timed_out.then(|| {
                    "broker_open_after_reconciliation_timeout_requires_review_before_rearm"
                        .to_string()
                })
            },
        ),
        terminal,
        timed_out,
    }
}

fn broker_reconciliation_event(
    status: &str,
    message: impl Into<String>,
    order_id: Option<String>,
    client_order_id: Option<String>,
) -> BrokerReconciliationEvent {
    let requires_review = matches!(
        status,
        "reconciliation_unknown" | "reconciliation_error" | "submit_error_unconfirmed"
    );
    BrokerReconciliationEvent {
        provenance: "broker_reconciliation".to_string(),
        status: status.to_string(),
        message: message.into(),
        at_unix_ms: now_unix_ms(),
        order_id,
        client_order_id,
        broker_order_status: None,
        filled_qty: None,
        filled_avg_price: None,
        refreshed_position_qty: None,
        requires_broker_review: requires_review.then_some(true),
        rearm_blocked_reason: requires_review
            .then(|| format!("{status}_requires_broker_review_before_rearm")),
        terminal: false,
        timed_out: false,
    }
}

fn broker_position_refreshed_event(
    order_id: Option<String>,
    client_order_id: Option<String>,
    refreshed_position_qty: f64,
) -> BrokerReconciliationEvent {
    BrokerReconciliationEvent {
        provenance: "broker_reconciliation".to_string(),
        status: "position_refreshed_after_terminal".to_string(),
        message: format!(
            "Broker position refreshed after terminal order state: qty={refreshed_position_qty:.6}."
        ),
        at_unix_ms: now_unix_ms(),
        order_id,
        client_order_id,
        broker_order_status: None,
        filled_qty: None,
        filled_avg_price: None,
        refreshed_position_qty: Some(refreshed_position_qty),
        requires_broker_review: Some(false),
        rearm_blocked_reason: None,
        terminal: true,
        timed_out: false,
    }
}

fn filter_symbol_orders_by_status<F>(
    orders: &[serde_json::Value],
    symbol: &str,
    mut include_status: F,
) -> Vec<LiveOrderSnapshot>
where
    F: FnMut(&str) -> bool,
{
    orders
        .iter()
        .filter_map(normalize_order_snapshot)
        .filter(|order| include_status(&order.status))
        .filter(|order| symbol_matches(&order.symbol, symbol))
        .collect()
}

fn open_orders_for_symbol(orders: &[serde_json::Value], symbol: &str) -> Vec<LiveOrderSnapshot> {
    filter_symbol_orders_by_status(orders, symbol, is_broker_open_order_status)
}

fn cancelable_orders_for_symbol(
    orders: &[serde_json::Value],
    symbol: &str,
) -> Vec<LiveOrderSnapshot> {
    filter_symbol_orders_by_status(orders, symbol, is_broker_cancelable_order_status)
}

#[derive(Clone, Debug, PartialEq)]
enum OpenOrderDeltaGuard {
    Proceed,
    Reconciled {
        projected_position_qty: f64,
        warning: String,
    },
}

fn remaining_open_order_qty(order: &LiveOrderSnapshot) -> Option<f64> {
    let qty = order.qty?;
    let filled_qty = order.filled_qty.unwrap_or(0.0);
    if !qty.is_finite() || !filled_qty.is_finite() || qty < 0.0 || filled_qty < 0.0 {
        return None;
    }
    Some((qty - filled_qty).max(0.0))
}

fn signed_remaining_open_order_qty(order: &LiveOrderSnapshot) -> Option<f64> {
    let remaining_qty = remaining_open_order_qty(order)?;
    match order.side.trim().to_ascii_lowercase().as_str() {
        "buy" => Some(remaining_qty),
        "sell" => Some(-remaining_qty),
        _ => None,
    }
}

fn guard_open_orders_before_fresh_delta(
    symbol: &str,
    current_position_qty: f64,
    target_position_qty: f64,
    open_orders: &[LiveOrderSnapshot],
) -> Result<OpenOrderDeltaGuard, String> {
    if open_orders.is_empty() {
        return Ok(OpenOrderDeltaGuard::Proceed);
    }

    let mut pending_delta_qty = 0.0_f64;
    for order in open_orders {
        let remaining_qty = signed_remaining_open_order_qty(order).ok_or_else(|| {
            format!(
                "Open orders already exist for {symbol}. Cancel or reconcile them before submitting a new delta because at least one order has incomplete quantity metadata."
            )
        })?;
        pending_delta_qty += remaining_qty;
    }

    let projected_position_qty = current_position_qty + pending_delta_qty;
    if delta_qty_is_hold(target_position_qty - projected_position_qty) {
        let plural = if open_orders.len() == 1 { "" } else { "s" };
        return Ok(OpenOrderDeltaGuard::Reconciled {
            projected_position_qty,
            warning: format!(
                "Reusing {} open order{plural} already working toward the target position for {symbol}; no new market delta submitted.",
                open_orders.len()
            ),
        });
    }

    Err(format!(
        "Open orders already exist for {symbol}. Cancel or reconcile them before submitting a new delta. current_position_qty={current_position_qty:.6}, target_position_qty={target_position_qty:.6}, projected_position_qty_with_open_orders={projected_position_qty:.6}."
    ))
}

fn summarize_snapshot_orders(
    orders_raw: &[serde_json::Value],
    symbol: &str,
    return_limit: usize,
) -> (Vec<LiveOrderSnapshot>, usize) {
    let mut orders: Vec<LiveOrderSnapshot> = Vec::new();
    let mut open_order_count = 0usize;
    for order in orders_raw.iter().filter_map(normalize_order_snapshot) {
        if !symbol_matches(&order.symbol, symbol) {
            continue;
        }
        if is_broker_open_order_status(&order.status) {
            open_order_count = open_order_count.saturating_add(1);
        }
        if orders.len() < return_limit {
            orders.push(order);
        }
    }
    (orders, open_order_count)
}

fn apply_manual_execution_audit_result(
    mut response: ExecuteStrategySignalResponse,
    audit_result: Result<(), String>,
) -> Result<ExecuteStrategySignalResponse, String> {
    match audit_result {
        Ok(()) => Ok(response),
        Err(error) if response.order_submitted => {
            response.warnings.push(format!(
                "Order was submitted, but live-session audit capture failed: {error}"
            ));
            Ok(response)
        }
        Err(error) => Err(error),
    }
}

fn rollback_runner_start(generation: u64) {
    if let Ok(mut slot) = live_execution_runner_slot().lock() {
        if slot
            .as_ref()
            .map(|runtime| runtime.generation == generation)
            .unwrap_or(false)
        {
            if let Some(runtime) = slot.as_ref() {
                runtime.abort.cancel();
            }
            *slot = None;
        }
    }
}

fn normalize_position_snapshot(v: &serde_json::Value) -> Option<LivePositionSnapshot> {
    let symbol = json_field_as_string(v, "symbol")?;
    let qty = json_field_as_f64(v, "qty").unwrap_or(0.0);
    Some(LivePositionSnapshot {
        symbol,
        qty,
        side: json_field_as_string(v, "side"),
        avg_entry_price: json_field_as_f64(v, "avg_entry_price"),
        current_price: json_field_as_f64(v, "current_price"),
        market_value: json_field_as_f64(v, "market_value"),
        unrealized_pl: json_field_as_f64(v, "unrealized_pl"),
        unrealized_plpc: json_field_as_f64(v, "unrealized_plpc"),
    })
}

fn flatten_position_order_payload(
    symbol: &str,
    current_position_qty: f64,
) -> Option<(String, Value)> {
    if !current_position_qty.is_finite() || current_position_qty.abs() <= f64::EPSILON {
        return None;
    }
    let action = if current_position_qty > 0.0 {
        "sell"
    } else {
        "buy"
    }
    .to_string();
    Some((
        action,
        serde_json::json!({
            "symbol": symbol,
            "qty": format!("{:.6}", current_position_qty.abs()),
            "side": if current_position_qty > 0.0 { "sell" } else { "buy" },
            "type": "market",
            "time_in_force": "day"
        }),
    ))
}

fn format_alpaca_price_by_increment(price: f64) -> Result<String, String> {
    if !price.is_finite() || price <= 0.0 {
        return Err("Alpaca bracket price must be finite and > 0.".to_string());
    }
    let decimals: i32 = if price >= 1.0 { 2 } else { 4 };
    let scale = 10_f64.powi(decimals);
    let rounded = (price * scale).round() / scale;
    if !rounded.is_finite() || rounded <= 0.0 {
        return Err("Alpaca bracket price rounded to a non-positive value.".to_string());
    }
    let decimals: usize = if rounded >= 1.0 { 2 } else { 4 };
    let scale = 10_f64.powi(decimals as i32);
    let rounded = (price * scale).round() / scale;
    if !rounded.is_finite() || rounded <= 0.0 {
        return Err("Alpaca bracket price rounded to a non-positive value.".to_string());
    }
    Ok(format!("{rounded:.decimals$}"))
}

fn validate_formatted_alpaca_bracket_prices(
    side: &str,
    stop_price: &str,
    take_profit_price: &str,
) -> Result<(), String> {
    let stop = stop_price
        .parse::<f64>()
        .map_err(|_| "formatted Alpaca stop price was not numeric".to_string())?;
    let take = take_profit_price
        .parse::<f64>()
        .map_err(|_| "formatted Alpaca take-profit price was not numeric".to_string())?;
    match side {
        "buy" if take <= stop => Err(format!(
            "Alpaca buy bracket would invert after price rounding: take_profit={take_profit_price} stop_loss={stop_price}."
        )),
        "sell" if take >= stop => Err(format!(
            "Alpaca sell bracket would invert after price rounding: take_profit={take_profit_price} stop_loss={stop_price}."
        )),
        _ => Ok(()),
    }
}

fn alpaca_order_payload(
    symbol: &str,
    delta_qty: f64,
    cfg: &ta_strategies::double_ma::StrategyConfig,
    last_close: f64,
) -> Result<(serde_json::Value, Vec<String>), String> {
    let mut warnings: Vec<String> = Vec::new();
    let qty = delta_qty.abs();
    let side = if delta_qty > 0.0 { "buy" } else { "sell" };
    let mut payload = serde_json::json!({
        "symbol": symbol,
        "qty": format!("{qty:.6}"),
        "side": side,
        "type": "market",
        "time_in_force": "day"
    });

    let stop = cfg.stop_loss_pct();
    let take = cfg.take_profit_pct();
    if let (Some(stop_pct), Some(take_pct)) = (stop, take) {
        let entry_dir = if delta_qty >= 0.0 { 1 } else { -1 };
        let (stop_px, take_px) = if entry_dir > 0 {
            (
                (last_close * (1.0 - stop_pct)).max(0.01),
                (last_close * (1.0 + take_pct)).max(0.01),
            )
        } else {
            (
                (last_close * (1.0 + stop_pct)).max(0.01),
                (last_close * (1.0 - take_pct)).max(0.01),
            )
        };
        let stop_price = format_alpaca_price_by_increment(stop_px)?;
        let take_profit_price = format_alpaca_price_by_increment(take_px)?;
        validate_formatted_alpaca_bracket_prices(side, &stop_price, &take_profit_price)?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("order_class".to_string(), serde_json::json!("bracket"));
            obj.insert(
                "take_profit".to_string(),
                serde_json::json!({ "limit_price": take_profit_price }),
            );
            obj.insert(
                "stop_loss".to_string(),
                serde_json::json!({ "stop_price": stop_price }),
            );
        }
    } else if stop.is_some() || take.is_some() {
        warnings.push(
            "Live bracket was skipped: Alpaca bracket requires both stop_loss_pct and take_profit_pct."
                .to_string(),
        );
    }

    Ok((payload, warnings))
}

fn paper_order_side_for_delta(delta_qty: f64) -> PaperOrderSide {
    if delta_qty > 0.0 {
        PaperOrderSide::Buy
    } else {
        PaperOrderSide::Sell
    }
}

fn paper_order_timestamp(value: i64) -> Option<String> {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(value)
        .map(|ts| ts.to_rfc3339())
        .or_else(|| Some(value.to_string()))
}

fn paper_order_snapshot(order: &PaperBrokerOrder) -> LiveOrderSnapshot {
    LiveOrderSnapshot {
        id: order.id.clone(),
        client_order_id: order.client_order_id.clone(),
        symbol: order.symbol.clone(),
        side: order.side.as_str().to_string(),
        status: order.status.clone(),
        order_type: order.order_type.clone(),
        qty: Some(order.qty),
        filled_qty: Some(order.filled_qty),
        filled_avg_price: Some(order.avg_fill_price),
        limit_price: None,
        stop_price: None,
        submitted_at: paper_order_timestamp(order.submitted_at_unix_ms),
        filled_at: paper_order_timestamp(order.filled_at_unix_ms),
        updated_at: paper_order_timestamp(order.filled_at_unix_ms),
        canceled_at: None,
    }
}

fn paper_position_snapshot(position: &PaperBrokerPosition) -> LivePositionSnapshot {
    LivePositionSnapshot {
        symbol: position.symbol.clone(),
        qty: position.qty,
        side: Some(if position.qty >= 0.0 { "long" } else { "short" }.to_string()),
        avg_entry_price: Some(position.avg_entry_price),
        current_price: Some(position.market_price),
        market_value: Some(position.market_value),
        unrealized_pl: Some(position.unrealized_pnl),
        unrealized_plpc: if position.avg_entry_price.abs() > f64::EPSILON {
            Some(position.unrealized_pnl / (position.avg_entry_price * position.qty.abs()))
        } else {
            None
        },
    }
}

fn local_paper_snapshot_blocking(
    app_local_data_dir: &Path,
    symbol: &str,
    limit: u32,
    now_unix_ms: i64,
) -> Result<GetLiveExecutionSnapshotResponse, String> {
    let symbol = canonical_symbol_key(symbol);
    let doc = broker_paper::load_or_default(app_local_data_dir, now_unix_ms)?;
    let position = if symbol.is_empty() {
        None
    } else {
        doc.positions.get(&symbol).map(paper_position_snapshot)
    };
    let mut orders: Vec<LiveOrderSnapshot> = doc
        .orders
        .iter()
        .rev()
        .filter(|order| symbol.is_empty() || symbol_matches(&order.symbol, &symbol))
        .take(limit as usize)
        .map(paper_order_snapshot)
        .collect();
    orders.reverse();
    Ok(GetLiveExecutionSnapshotResponse {
        mode: LiveExecutionMode::Paper,
        symbol,
        account_id: Some(doc.account.account_id),
        account_number: Some("local-paper".to_string()),
        account_status: Some("simulated".to_string()),
        currency: Some(doc.account.currency),
        buying_power: Some(format!("{:.2}", doc.account.buying_power)),
        cash: Some(format!("{:.2}", doc.account.cash)),
        portfolio_value: Some(format!("{:.2}", doc.account.portfolio_value)),
        fetched_at_unix_ms: doc.account.updated_at_unix_ms,
        position,
        orders,
    })
}

fn normalize_execution_safety_symbol(raw: &str) -> Result<String, String> {
    let symbol = canonical_symbol_key(raw);
    if symbol.is_empty() {
        Err("symbol is required".to_string())
    } else {
        Ok(symbol)
    }
}

fn ensure_runner_inactive_for_broker_safety_action() -> Result<(), String> {
    let slot = live_execution_runner_slot()
        .lock()
        .map_err(|_| "failed to lock live execution runner state".to_string())?;
    if let Some(runtime) = slot.as_ref() {
        return Err(format!(
            "Stop the auto-runner for {} before broker safety actions.",
            runtime.cfg.symbol
        ));
    }
    Ok(())
}

fn ensure_runner_inactive_for_manual_broker_execution(dry_run: bool) -> Result<(), String> {
    if dry_run {
        return Ok(());
    }
    let slot = live_execution_runner_slot()
        .lock()
        .map_err(|_| "failed to lock live execution runner state".to_string())?;
    if let Some(runtime) = slot.as_ref() {
        return Err(format!(
            "Stop the auto-runner for {} before manual broker execution.",
            runtime.cfg.symbol
        ));
    }
    Ok(())
}

fn emergency_halt_status_message(runner_stopped: bool, live_stopped: bool) -> String {
    match (runner_stopped, live_stopped) {
        (true, true) => {
            "Emergency halt applied. Live feed and auto-runner stopped. Refresh orders before re-arming.".to_string()
        }
        (true, false) => {
            "Emergency halt applied. Auto-runner stopped. Refresh orders before re-arming.".to_string()
        }
        (false, true) => {
            "Emergency halt applied. Live feed stopped. Refresh orders before re-arming.".to_string()
        }
        (false, false) => "Live automation was already disarmed.".to_string(),
    }
}

#[derive(Clone, Debug)]
struct AlpacaOrderSubmitOutcome {
    order_id: String,
    client_order_id: String,
    reused_existing: bool,
}

#[derive(Clone, Debug)]
struct AlpacaOrderReconcileOutcome {
    snapshot: Option<LiveOrderSnapshot>,
    terminal: bool,
    timed_out: bool,
}

const ALPACA_ORDER_INTENT_RECORD_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AlpacaOrderIntentRecord {
    version: u32,
    client_order_id: String,
    status: String,
    mode: LiveExecutionMode,
    execution_target: ExecutionBrokerTarget,
    symbol: String,
    side: String,
    qty: Option<f64>,
    current_position_qty: f64,
    target_position_qty: f64,
    delta_qty: f64,
    desired_position: i32,
    last_bar_ts: Option<i64>,
    last_close: f64,
    strategy_kind: String,
    data_id: String,
    param_source: String,
    source_run_artifact_path: String,
    source_run_config_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_run_route_provenance: Option<Value>,
    order_payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    broker_order_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    attempt_count: u32,
    created_at_unix_ms: i64,
    updated_at_unix_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    submitted_at_unix_ms: Option<i64>,
}

fn alpaca_intents_dir(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join("execution").join("alpaca_intents")
}

fn safe_client_order_file_stem(client_order_id: &str) -> String {
    let stem: String = client_order_id
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if stem.is_empty() {
        "unknown_client_order_id".to_string()
    } else {
        stem
    }
}

fn alpaca_intent_record_path(app_local_data_dir: &Path, client_order_id: &str) -> PathBuf {
    alpaca_intents_dir(app_local_data_dir).join(format!(
        "{}.json",
        safe_client_order_file_stem(client_order_id)
    ))
}

fn read_alpaca_order_intent_record(
    app_local_data_dir: &Path,
    client_order_id: &str,
) -> Result<Option<AlpacaOrderIntentRecord>, String> {
    let path = alpaca_intent_record_path(app_local_data_dir, client_order_id);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)
        .map_err(|e| format!("failed to read Alpaca order intent {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map(Some).map_err(|e| {
        format!(
            "failed to decode Alpaca order intent {}: {e}",
            path.display()
        )
    })
}

fn read_all_alpaca_order_intent_records(
    app_local_data_dir: &Path,
) -> Result<Vec<AlpacaOrderIntentRecord>, String> {
    let dir = alpaca_intents_dir(app_local_data_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| {
        format!(
            "failed to read Alpaca order intent directory {}: {e}",
            dir.display()
        )
    })? {
        let path = entry
            .map_err(|e| format!("failed to read Alpaca order intent entry: {e}"))?
            .path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read(&path)
            .map_err(|e| format!("failed to read Alpaca order intent {}: {e}", path.display()))?;
        records.push(serde_json::from_slice(&bytes).map_err(|e| {
            format!(
                "failed to decode Alpaca order intent {}: {e}",
                path.display()
            )
        })?);
    }
    Ok(records)
}

fn write_alpaca_order_intent_record(
    app_local_data_dir: &Path,
    record: &AlpacaOrderIntentRecord,
) -> Result<PathBuf, String> {
    let path = alpaca_intent_record_path(app_local_data_dir, &record.client_order_id);
    let Some(parent) = path.parent() else {
        return Err("failed to resolve Alpaca order intent directory".to_string());
    };
    fs::create_dir_all(parent).map_err(|e| {
        format!(
            "failed to create Alpaca order intent directory {}: {e}",
            parent.display()
        )
    })?;
    let bytes = serde_json::to_vec_pretty(record)
        .map_err(|e| format!("failed to encode Alpaca order intent: {e}"))?;
    fs::write(&path, bytes).map_err(|e| {
        format!(
            "failed to persist Alpaca order intent {}: {e}",
            path.display()
        )
    })?;
    Ok(path)
}

fn build_pending_alpaca_order_intent(
    req: &ExecuteStrategySignalRequest,
    symbol: &str,
    decision: &SignalDecision,
    current_position_qty: f64,
    target_position_qty: f64,
    delta_qty: f64,
    client_order_id: &str,
    order_payload: &Value,
    now_unix_ms: i64,
) -> AlpacaOrderIntentRecord {
    AlpacaOrderIntentRecord {
        version: ALPACA_ORDER_INTENT_RECORD_VERSION,
        client_order_id: client_order_id.to_string(),
        status: "pending_submit".to_string(),
        mode: req.mode,
        execution_target: execution_target_for_request(req),
        symbol: symbol.to_string(),
        side: json_field_as_string(order_payload, "side").unwrap_or_default(),
        qty: json_field_as_f64(order_payload, "qty"),
        current_position_qty,
        target_position_qty,
        delta_qty,
        desired_position: decision.desired_position,
        last_bar_ts: decision.last_bar_ts,
        last_close: decision.last_close,
        strategy_kind: strategy_kind_label(&req.strategy),
        data_id: strategy_data_id(&req.strategy).to_string(),
        param_source: req.param_source.trim().to_string(),
        source_run_artifact_path: req.source_run_artifact_path.trim().to_string(),
        source_run_config_fingerprint: req.source_run_config_fingerprint.trim().to_string(),
        source_run_route_provenance: req.source_run_route_provenance.clone(),
        order_payload: order_payload.clone(),
        broker_order_id: None,
        error: None,
        attempt_count: 1,
        created_at_unix_ms: now_unix_ms,
        updated_at_unix_ms: now_unix_ms,
        submitted_at_unix_ms: None,
    }
}

fn persist_pending_alpaca_order_intent(
    app_local_data_dir: &Path,
    mut record: AlpacaOrderIntentRecord,
) -> Result<AlpacaOrderIntentRecord, String> {
    if let Some(existing) =
        read_alpaca_order_intent_record(app_local_data_dir, &record.client_order_id)?
    {
        record.created_at_unix_ms = existing.created_at_unix_ms;
        record.attempt_count = existing.attempt_count.saturating_add(1);
    }
    write_alpaca_order_intent_record(app_local_data_dir, &record)?;
    Ok(record)
}

fn update_alpaca_order_intent_status(
    app_local_data_dir: &Path,
    client_order_id: &str,
    status: &str,
    broker_order_id: Option<String>,
    error: Option<String>,
    now_unix_ms: i64,
) -> Result<AlpacaOrderIntentRecord, String> {
    let mut record = read_alpaca_order_intent_record(app_local_data_dir, client_order_id)?
        .ok_or_else(|| {
            format!("missing Alpaca order intent for client_order_id={client_order_id}")
        })?;
    record.status = status.to_string();
    record.updated_at_unix_ms = now_unix_ms;
    record.broker_order_id = broker_order_id;
    record.error = error;
    if matches!(status, "submitted" | "reused_existing") {
        record.submitted_at_unix_ms = Some(now_unix_ms);
    }
    write_alpaca_order_intent_record(app_local_data_dir, &record)?;
    Ok(record)
}

fn unresolved_alpaca_intent_status(status: &str) -> bool {
    let status = status.trim().to_ascii_lowercase();
    matches!(
        status.as_str(),
        "pending_submit"
            | "submitted"
            | "reused_existing"
            | "broker_open"
            | "partial_fill"
            | "reconciliation_unknown"
            | "reconciliation_error"
            | "submit_error"
            | "submit_error_unconfirmed"
            | "done_for_day_review_required"
            | "suspended_review_required"
            | "broker_unknown"
    ) || status.ends_with("_review_required")
        || status.starts_with("broker_")
}

fn block_unresolved_alpaca_intents_before_submit(
    app_local_data_dir: &Path,
    symbol: &str,
    current_client_order_id: &str,
) -> Result<(), String> {
    let conflicts: Vec<_> = read_all_alpaca_order_intent_records(app_local_data_dir)?
        .into_iter()
        .filter(|record| symbol_matches(&record.symbol, symbol))
        .filter(|record| record.client_order_id != current_client_order_id)
        .filter(|record| unresolved_alpaca_intent_status(&record.status))
        .collect();
    if let Some(conflict) = conflicts
        .iter()
        .max_by_key(|record| record.updated_at_unix_ms)
    {
        return Err(format!(
            "Unresolved Alpaca intent exists for {symbol}: client_order_id={} status={}. Refresh broker review or resolve the prior order before submitting another broker delta.",
            conflict.client_order_id, conflict.status
        ));
    }
    Ok(())
}

fn alpaca_order_submit_outcome_from_value(
    value: &serde_json::Value,
    expected_client_order_id: &str,
    reused_existing: bool,
) -> Result<AlpacaOrderSubmitOutcome, String> {
    let order_id = json_field_as_string(value, "id")
        .ok_or_else(|| "Alpaca order response missing id".to_string())?;
    let client_order_id = json_field_as_string(value, "client_order_id")
        .unwrap_or_else(|| expected_client_order_id.to_string());
    Ok(AlpacaOrderSubmitOutcome {
        order_id,
        client_order_id,
        reused_existing,
    })
}

fn existing_alpaca_order_submit_outcome(
    value: &serde_json::Value,
    expected_symbol: &str,
    expected_client_order_id: &str,
) -> Result<AlpacaOrderSubmitOutcome, String> {
    let snapshot = normalize_order_snapshot(value)
        .ok_or_else(|| "Alpaca client-order lookup returned an incomplete order".to_string())?;
    if !symbol_matches(&snapshot.symbol, expected_symbol) {
        return Err(format!(
            "Alpaca client_order_id collision: expected symbol {expected_symbol}, found {}.",
            snapshot.symbol
        ));
    }
    alpaca_order_submit_outcome_from_value(value, expected_client_order_id, true)
}

fn submit_alpaca_order(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let headers = build_alpaca_headers(api_key, api_secret)?;
    let url = format!("{}/v2/orders", base_url(mode));
    let resp = client
        .post(url)
        .headers(headers)
        .json(payload)
        .send()
        .map_err(|e| format!("failed to submit Alpaca order: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Alpaca order failed ({status}): {body}"));
    }
    resp.json()
        .map_err(|e| format!("failed to decode Alpaca order response: {e}"))
}

fn submit_or_reuse_alpaca_order(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    symbol: &str,
    payload: &serde_json::Value,
    client_order_id: &str,
) -> Result<AlpacaOrderSubmitOutcome, String> {
    submit_or_reuse_alpaca_order_with_ops(
        symbol,
        client_order_id,
        || {
            fetch_alpaca_order_by_client_order_id(
                client,
                mode,
                api_key,
                api_secret,
                client_order_id,
            )
        },
        || submit_alpaca_order(client, mode, api_key, api_secret, payload),
    )
}

fn submit_or_reuse_alpaca_order_with_ops<L, S>(
    symbol: &str,
    client_order_id: &str,
    mut lookup_by_client_order_id: L,
    mut submit_order: S,
) -> Result<AlpacaOrderSubmitOutcome, String>
where
    L: FnMut() -> Result<Option<serde_json::Value>, String>,
    S: FnMut() -> Result<serde_json::Value, String>,
{
    if let Some(existing) = lookup_by_client_order_id()? {
        return existing_alpaca_order_submit_outcome(&existing, symbol, client_order_id);
    }

    match submit_order() {
        Ok(value) => alpaca_order_submit_outcome_from_value(&value, client_order_id, false),
        Err(submit_error) => match lookup_by_client_order_id() {
            Ok(Some(existing)) => {
                existing_alpaca_order_submit_outcome(&existing, symbol, client_order_id)
            }
            Ok(None) => Err(submit_error),
            Err(lookup_error) => Err(format!(
                "{submit_error}. Follow-up client_order_id lookup also failed: {lookup_error}"
            )),
        },
    }
}

fn reconcile_alpaca_order_after_submit(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    client_order_id: &str,
    max_attempts: usize,
    poll_delay: Duration,
) -> Result<AlpacaOrderReconcileOutcome, String> {
    let attempts = max_attempts.max(1);
    let mut last_snapshot: Option<LiveOrderSnapshot> = None;
    for attempt in 0..attempts {
        if attempt > 0 && !poll_delay.is_zero() {
            std::thread::sleep(poll_delay);
        }
        let Some(value) = fetch_alpaca_order_by_client_order_id(
            client,
            mode,
            api_key,
            api_secret,
            client_order_id,
        )?
        else {
            continue;
        };
        let snapshot = normalize_order_snapshot(&value).ok_or_else(|| {
            "Alpaca client-order reconciliation returned an incomplete order".to_string()
        })?;
        let terminal = is_broker_terminal_order_status(&snapshot.status);
        last_snapshot = Some(snapshot);
        if terminal {
            return Ok(AlpacaOrderReconcileOutcome {
                snapshot: last_snapshot,
                terminal: true,
                timed_out: false,
            });
        }
    }

    let timed_out = last_snapshot
        .as_ref()
        .map(|snapshot| !is_broker_terminal_order_status(&snapshot.status))
        .unwrap_or(true);
    Ok(AlpacaOrderReconcileOutcome {
        snapshot: last_snapshot,
        terminal: false,
        timed_out,
    })
}

fn preview_execution_policy_decision_blocking(
    req: ExecuteStrategySignalRequest,
    symbol: String,
    qty: f64,
    candles: Arc<Candles>,
    app_local_data_dir: PathBuf,
) -> Result<PreviewExecutionPolicyDecisionResponse, String> {
    let execution_target = execution_target_for_request(&req);
    validate_live_execution_source_context(&req, execution_target)?;
    let decision = signal_decision_from_request(&req.strategy, candles.as_ref(), &req.risk)?;
    let target_position_qty = (decision.desired_position as f64) * qty;
    let mut warnings = Vec::new();
    let current_position_qty = if matches!(execution_target, ExecutionBrokerTarget::LocalPaper) {
        let doc = broker_paper::load_or_default(&app_local_data_dir, now_unix_ms())?;
        doc.positions.get(&symbol).map(|p| p.qty).unwrap_or(0.0)
    } else {
        if execution_target.is_alpaca() {
            warnings.push(
                "Policy preview does not fetch Alpaca position or open-order state; submit still rechecks broker state before any order."
                    .to_string(),
            );
        }
        0.0
    };
    let delta_qty = target_position_qty - current_position_qty;
    let opening_new = should_attach_opening_bracket(current_position_qty, target_position_qty);
    let effective_policy =
        merged_execution_risk_policy_for_request(Some(&app_local_data_dir), &req.risk)?;
    let policy_decision = evaluate_execution_policy(ExecutionPolicyInput {
        qty,
        current_position_qty,
        target_position_qty,
        delta_qty,
        last_close: decision.last_close,
        opening_new_position: opening_new,
        stop_loss_pct: decision.cfg.stop_loss_pct(),
        take_profit_pct: decision.cfg.take_profit_pct(),
        limits: execution_policy_limits_from_settings(&effective_policy),
    });
    warnings.extend(policy_decision.warnings.clone());

    Ok(PreviewExecutionPolicyDecisionResponse {
        symbol,
        mode: req.mode,
        execution_target,
        desired_position: decision.desired_position,
        current_position_qty,
        target_position_qty,
        delta_qty,
        last_bar_ts: decision.last_bar_ts,
        last_close: decision.last_close,
        opening_new_position: opening_new,
        policy: effective_policy,
        decision: policy_decision,
        warnings,
    })
}

fn execute_strategy_signal_blocking(
    req: ExecuteStrategySignalRequest,
    symbol: String,
    qty: f64,
    candles: Arc<Candles>,
    abort: Option<RunnerAbortHandle>,
    app_local_data_dir: Option<PathBuf>,
) -> Result<ExecuteStrategySignalResponse, String> {
    if let Some(abort) = abort.as_ref() {
        abort.check()?;
    }
    let execution_target = execution_target_for_request(&req);
    let decision = signal_decision_from_request(&req.strategy, candles.as_ref(), &req.risk)?;
    if let Some(abort) = abort.as_ref() {
        abort.check()?;
    }
    let target_position_qty = (decision.desired_position as f64) * qty;
    let mut warnings: Vec<String> = Vec::new();

    let mut current_position_qty = 0.0_f64;
    let mut open_orders: Vec<LiveOrderSnapshot> = Vec::new();
    if matches!(execution_target, ExecutionBrokerTarget::LocalPaper) {
        let base = app_local_data_dir.as_deref().ok_or_else(|| {
            "app local data directory is required for local paper execution".to_string()
        })?;
        let doc = broker_paper::load_or_default(base, now_unix_ms())?;
        current_position_qty = doc.positions.get(&symbol).map(|p| p.qty).unwrap_or(0.0);
    } else if execution_target.is_alpaca() {
        if req.alpaca_api_key.trim().is_empty() || req.alpaca_api_secret.trim().is_empty() {
            return Err(
                "alpaca_api_key and alpaca_api_secret are required unless dry_run=true".to_string(),
            );
        }
        let client = alpaca_client()?;
        validate_alpaca_live_broker_preflight(client, &req, execution_target)?;
        current_position_qty = fetch_alpaca_position_qty(
            client,
            req.mode,
            req.alpaca_api_key.trim(),
            req.alpaca_api_secret.trim(),
            &symbol,
        )?;
        open_orders = fetch_alpaca_open_orders_for_symbol(
            client,
            req.mode,
            req.alpaca_api_key.trim(),
            req.alpaca_api_secret.trim(),
            &symbol,
        )
        .map(|orders| open_orders_for_symbol(&orders, &symbol))?;
        if let Some(abort) = abort.as_ref() {
            abort.check()?;
        }
    }

    match guard_open_orders_before_fresh_delta(
        &symbol,
        current_position_qty,
        target_position_qty,
        &open_orders,
    )? {
        OpenOrderDeltaGuard::Proceed => {}
        OpenOrderDeltaGuard::Reconciled {
            projected_position_qty,
            warning,
        } => {
            warnings.push(warning);
            return Ok(ExecuteStrategySignalResponse {
                symbol,
                mode: req.mode,
                desired_position: decision.desired_position,
                current_position_qty,
                target_position_qty,
                delta_qty: 0.0,
                order_submitted: false,
                order_id: None,
                client_order_id: None,
                action: "reconcile_open_orders".to_string(),
                last_bar_ts: decision.last_bar_ts,
                last_close: Some(decision.last_close),
                broker_reconciliation_events: Vec::new(),
                warnings: {
                    warnings.push(format!(
                        "Projected broker position after the open-order queue is {projected_position_qty:.6}."
                    ));
                    warnings
                },
            });
        }
    }

    let delta_qty = target_position_qty - current_position_qty;
    if delta_qty_is_hold(delta_qty) {
        return Ok(ExecuteStrategySignalResponse {
            symbol,
            mode: req.mode,
            desired_position: decision.desired_position,
            current_position_qty,
            target_position_qty,
            delta_qty: 0.0,
            order_submitted: false,
            order_id: None,
            client_order_id: None,
            action: "hold".to_string(),
            last_bar_ts: decision.last_bar_ts,
            last_close: Some(decision.last_close),
            broker_reconciliation_events: Vec::new(),
            warnings,
        });
    }

    let opening_new = should_attach_opening_bracket(current_position_qty, target_position_qty);
    let effective_policy =
        merged_execution_risk_policy_for_request(app_local_data_dir.as_deref(), &req.risk)?;
    let policy = evaluate_execution_policy(ExecutionPolicyInput {
        qty,
        current_position_qty,
        target_position_qty,
        delta_qty,
        last_close: decision.last_close,
        opening_new_position: opening_new,
        stop_loss_pct: decision.cfg.stop_loss_pct(),
        take_profit_pct: decision.cfg.take_profit_pct(),
        limits: execution_policy_limits_from_settings(&effective_policy),
    });
    if !policy.allowed {
        warnings.push(format!(
            "Execution policy blocked order: {}",
            policy.message
        ));
        return Ok(ExecuteStrategySignalResponse {
            symbol,
            mode: req.mode,
            desired_position: decision.desired_position,
            current_position_qty,
            target_position_qty,
            delta_qty,
            order_submitted: false,
            order_id: None,
            client_order_id: None,
            action: format!("policy_blocked:{}", policy.code),
            last_bar_ts: decision.last_bar_ts,
            last_close: Some(decision.last_close),
            broker_reconciliation_events: Vec::new(),
            warnings,
        });
    }
    warnings.extend(policy.warnings);

    if matches!(execution_target, ExecutionBrokerTarget::DryRun) {
        return Ok(ExecuteStrategySignalResponse {
            symbol,
            mode: req.mode,
            desired_position: decision.desired_position,
            current_position_qty,
            target_position_qty,
            delta_qty,
            order_submitted: false,
            order_id: None,
            client_order_id: None,
            action: "dry_run_signal_only".to_string(),
            last_bar_ts: decision.last_bar_ts,
            last_close: Some(decision.last_close),
            broker_reconciliation_events: Vec::new(),
            warnings,
        });
    }

    if matches!(execution_target, ExecutionBrokerTarget::LocalPaper) {
        if let Some(abort) = abort.as_ref() {
            abort.check()?;
        }
        let base = app_local_data_dir.as_deref().ok_or_else(|| {
            "app local data directory is required for local paper execution".to_string()
        })?;
        let side = paper_order_side_for_delta(delta_qty);
        if local_paper_bracket_child_orders_required_but_unsupported(&effective_policy, opening_new)
        {
            warnings.push(
                "Local paper cannot satisfy require_bracket=true because bracket child orders are not simulated."
                    .to_string(),
            );
            return Ok(ExecuteStrategySignalResponse {
                symbol,
                mode: req.mode,
                desired_position: decision.desired_position,
                current_position_qty,
                target_position_qty,
                delta_qty,
                order_submitted: false,
                order_id: None,
                client_order_id: None,
                action: "policy_blocked:local_paper_bracket_unsupported".to_string(),
                last_bar_ts: decision.last_bar_ts,
                last_close: Some(decision.last_close),
                broker_reconciliation_events: Vec::new(),
                warnings,
            });
        }
        if opening_new
            && (decision.cfg.stop_loss_pct().is_some() || decision.cfg.take_profit_pct().is_some())
        {
            warnings.push(
                "Local paper broker does not simulate bracket child orders yet; stop-loss/take-profit settings were not attached."
                    .to_string(),
            );
        }
        let execution = broker_paper::submit_market_order(
            base,
            PaperBrokerMarketOrder {
                symbol: symbol.clone(),
                side,
                qty: delta_qty.abs(),
                price: decision.last_close,
                at_unix_ms: now_unix_ms(),
                fee_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
            },
        )?;
        warnings.push(
            "Local paper fill used the latest strategy close as the simulated market price."
                .to_string(),
        );
        return Ok(ExecuteStrategySignalResponse {
            symbol,
            mode: req.mode,
            desired_position: decision.desired_position,
            current_position_qty,
            target_position_qty,
            delta_qty,
            order_submitted: true,
            order_id: Some(execution.order.id),
            client_order_id: None,
            action: side.as_str().to_string(),
            last_bar_ts: decision.last_bar_ts,
            last_close: Some(decision.last_close),
            broker_reconciliation_events: Vec::new(),
            warnings,
        });
    }

    if let Some(abort) = abort.as_ref() {
        abort.check()?;
    }
    let client = alpaca_client()?;
    let (mut payload, bracket_warnings) = if opening_new {
        alpaca_order_payload(&symbol, delta_qty, &decision.cfg, decision.last_close)?
    } else {
        (
            serde_json::json!({
                "symbol": symbol,
                "qty": format!("{:.6}", delta_qty.abs()),
                "side": if delta_qty > 0.0 { "buy" } else { "sell" },
                "type": "market",
                "time_in_force": "day"
            }),
            Vec::new(),
        )
    };
    warnings.extend(bracket_warnings);
    let client_order_id =
        alpaca_client_order_id(&req, &symbol, &decision, target_position_qty, delta_qty);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "client_order_id".to_string(),
            serde_json::json!(client_order_id.clone()),
        );
    }
    let intent_base = app_local_data_dir.as_deref().ok_or_else(|| {
        "app local data directory is required for Alpaca execution intent tracking".to_string()
    })?;
    block_unresolved_alpaca_intents_before_submit(intent_base, &symbol, &client_order_id)?;
    let pending_intent = persist_pending_alpaca_order_intent(
        intent_base,
        build_pending_alpaca_order_intent(
            &req,
            &symbol,
            &decision,
            current_position_qty,
            target_position_qty,
            delta_qty,
            &client_order_id,
            &payload,
            now_unix_ms(),
        ),
    )?;
    if pending_intent.attempt_count > 1 {
        warnings.push(format!(
            "Reusing pending Alpaca intent client_order_id={} from a prior attempt before broker submit.",
            pending_intent.client_order_id
        ));
    }
    if let Some(abort) = abort.as_ref() {
        abort.check()?;
    }

    let submitted_order = match submit_or_reuse_alpaca_order(
        client,
        req.mode,
        req.alpaca_api_key.trim(),
        req.alpaca_api_secret.trim(),
        &symbol,
        &payload,
        &client_order_id,
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            if let Err(update_error) = update_alpaca_order_intent_status(
                intent_base,
                &client_order_id,
                "submit_error_unconfirmed",
                None,
                Some(error.clone()),
                now_unix_ms(),
            ) {
                return Err(format!(
                    "{error}. Also failed to persist Alpaca intent submit_error_unconfirmed: {update_error}"
                ));
            }
            return Err(error);
        }
    };
    let submitted_status = if submitted_order.reused_existing {
        "reused_existing"
    } else {
        "submitted"
    };
    if let Err(update_error) = update_alpaca_order_intent_status(
        intent_base,
        &client_order_id,
        submitted_status,
        Some(submitted_order.order_id.clone()),
        None,
        now_unix_ms(),
    ) {
        warnings.push(format!(
            "Alpaca order was accepted, but intent status update failed: {update_error}"
        ));
    }
    let mut broker_reconciliation_events = Vec::new();
    match reconcile_alpaca_order_after_submit(
        client,
        req.mode,
        req.alpaca_api_key.trim(),
        req.alpaca_api_secret.trim(),
        &client_order_id,
        ALPACA_ORDER_RECONCILE_POLL_ATTEMPTS,
        Duration::from_millis(ALPACA_ORDER_RECONCILE_POLL_DELAY_MS),
    ) {
        Ok(outcome) => {
            if let Some(snapshot) = outcome.snapshot.as_ref() {
                let mut intent_status = alpaca_intent_status_for_order_snapshot(snapshot);
                let mut intent_error: Option<String> = None;
                if outcome.terminal {
                    warnings.push(format!(
                        "Alpaca order reconciled to terminal status {}.",
                        snapshot.status
                    ));
                    match fetch_alpaca_position_qty(
                        client,
                        req.mode,
                        req.alpaca_api_key.trim(),
                        req.alpaca_api_secret.trim(),
                        &symbol,
                    ) {
                        Ok(refreshed_qty) => {
                            if matches!(
                                classify_broker_order_status(&snapshot.status),
                                BrokerOrderStatusClass::TerminalFilled
                            ) {
                                intent_status = "filled".to_string();
                            }
                            broker_reconciliation_events.push(broker_position_refreshed_event(
                                Some(snapshot.id.clone()),
                                snapshot.client_order_id.clone(),
                                refreshed_qty,
                            ));
                        }
                        Err(error) => {
                            intent_error = Some(error.clone());
                            warnings.push(format!(
                                "Alpaca order reached terminal status, but position refresh failed: {error}"
                            ));
                        }
                    }
                } else if outcome.timed_out {
                    warnings.push(format!(
                        "Alpaca order is still {} after post-submit reconciliation; refresh broker review before another broker action.",
                        snapshot.status
                    ));
                }
                if let Err(update_error) = update_alpaca_order_intent_status(
                    intent_base,
                    &client_order_id,
                    &intent_status,
                    Some(snapshot.id.clone()),
                    intent_error,
                    now_unix_ms(),
                ) {
                    warnings.push(format!(
                        "Alpaca order was reconciled as {}, but intent status update failed: {update_error}",
                        snapshot.status
                    ));
                }
                broker_reconciliation_events.push(broker_reconciliation_event_from_snapshot(
                    snapshot,
                    outcome.terminal,
                    outcome.timed_out,
                ));
            } else {
                let _ = update_alpaca_order_intent_status(
                    intent_base,
                    &client_order_id,
                    "reconciliation_unknown",
                    Some(submitted_order.order_id.clone()),
                    None,
                    now_unix_ms(),
                );
                warnings.push(
                    "Alpaca order was accepted, but post-submit reconciliation did not return broker state; refresh broker review before another broker action."
                        .to_string(),
                );
                broker_reconciliation_events.push(broker_reconciliation_event(
                    "reconciliation_unknown",
                    "Alpaca order was accepted, but post-submit reconciliation did not return broker state.",
                    Some(submitted_order.order_id.clone()),
                    Some(submitted_order.client_order_id.clone()),
                ));
            }
        }
        Err(error) => {
            let _ = update_alpaca_order_intent_status(
                intent_base,
                &client_order_id,
                "reconciliation_error",
                Some(submitted_order.order_id.clone()),
                Some(error.clone()),
                now_unix_ms(),
            );
            warnings.push(format!(
                "Alpaca order was accepted, but post-submit reconciliation failed: {error}"
            ));
            broker_reconciliation_events.push(broker_reconciliation_event(
                "reconciliation_error",
                format!(
                    "Alpaca order was accepted, but post-submit reconciliation failed: {error}"
                ),
                Some(submitted_order.order_id.clone()),
                Some(submitted_order.client_order_id.clone()),
            ));
        }
    }
    if submitted_order.reused_existing {
        warnings.push(format!(
            "Reused existing Alpaca order for client_order_id={}.",
            submitted_order.client_order_id
        ));
    }

    Ok(ExecuteStrategySignalResponse {
        symbol,
        mode: req.mode,
        desired_position: decision.desired_position,
        current_position_qty,
        target_position_qty,
        delta_qty,
        order_submitted: true,
        order_id: Some(submitted_order.order_id),
        client_order_id: Some(submitted_order.client_order_id),
        action: if delta_qty > 0.0 {
            "buy".to_string()
        } else {
            "sell".to_string()
        },
        last_bar_ts: decision.last_bar_ts,
        last_close: Some(decision.last_close),
        broker_reconciliation_events,
        warnings,
    })
}

#[tauri::command]
pub async fn execute_strategy_signal_once(
    mut req: ExecuteStrategySignalRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ExecuteStrategySignalResponse, String> {
    let execution_target = normalize_execution_target_flags(&mut req);
    ensure_runner_inactive_for_manual_broker_execution(!execution_target.is_broker_armed())?;
    if execution_target.is_alpaca() {
        let (api_key, api_secret) = resolve_alpaca_credentials_for_app(
            &app.config().identifier,
            &req.alpaca_api_key,
            &req.alpaca_api_secret,
        )?
        .ok_or_else(|| {
            "alpaca_api_key and alpaca_api_secret are required unless dry_run=true".to_string()
        })?;
        req.alpaca_api_key = api_key;
        req.alpaca_api_secret = api_secret;
    }
    let (req, symbol, qty) = normalize_execute_request(req)?;
    validate_execution_source_preconditions(&req, execution_target)?;
    let data_id = strategy_data_id(&req.strategy).to_string();
    let strategy_kind = strategy_kind_label(&req.strategy);
    let candles = state.get_candles(&data_id)?;
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let req_for_session = req.clone();
    let symbol_for_session = symbol.clone();
    let response = tauri::async_runtime::spawn_blocking(move || {
        execute_strategy_signal_blocking(req, symbol, qty, candles, None, Some(app_local_data_dir))
    })
    .await
    .map_err(|e| e.to_string())??;
    if !req_for_session.dry_run {
        let audit_result = capture_manual_execution_live_session(
            &app,
            &req_for_session,
            &symbol_for_session,
            qty,
            &data_id,
            &strategy_kind,
            &response,
        );
        return apply_manual_execution_audit_result(response, audit_result);
    }
    Ok(response)
}

#[tauri::command]
pub async fn start_live_execution_runner(
    mut req: ExecuteStrategySignalRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<LiveExecutionRunnerStatus, String> {
    let execution_target = normalize_execution_target_flags(&mut req);
    if execution_target.is_alpaca() {
        let (api_key, api_secret) = resolve_alpaca_credentials_for_app(
            &app.config().identifier,
            &req.alpaca_api_key,
            &req.alpaca_api_secret,
        )?
        .ok_or_else(|| {
            "alpaca_api_key and alpaca_api_secret are required unless dry_run=true".to_string()
        })?;
        req.alpaca_api_key = api_key;
        req.alpaca_api_secret = api_secret;
    }
    let (req, symbol, qty) = normalize_execute_request(req)?;
    validate_live_execution_source_context(&req, execution_target)?;
    let data_id = strategy_data_id(&req.strategy).trim().to_string();
    if data_id.is_empty() {
        return Err("strategy data_id is required".to_string());
    }
    let _ = state.get_candles(&data_id)?;
    let live_stream = state.live_candle_stream_info()?;
    validate_live_runner_stream_preconditions(
        live_stream.as_ref(),
        &symbol,
        execution_target,
        now_unix_ms(),
    )?;
    let _ = finalize_active_live_session(&app, "superseded");

    let strategy_kind = strategy_kind_label(&req.strategy);
    let generation = LIVE_EXECUTION_RUNNER_GEN.fetch_add(1, Ordering::SeqCst);
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let cfg = LiveExecutionRunnerInternalCfg {
        req: req.clone(),
        symbol: symbol.clone(),
        qty,
        data_id: data_id.clone(),
        strategy_kind: strategy_kind.clone(),
        app_local_data_dir,
    };
    let runtime = LiveExecutionRunnerRuntime {
        generation,
        abort: RunnerAbortHandle::new(),
        pending_bar: None,
        cfg,
        inflight: false,
        started_at_unix_ms: now_unix_ms(),
        last_bar_open_unix_ms: None,
        last_triggered_at_unix_ms: None,
        last_completed_at_unix_ms: None,
        run_count: 0,
        skip_count: 0,
        error_count: 0,
        last_action: None,
        last_order_id: None,
        last_error: None,
    };

    {
        let mut slot = live_execution_runner_slot()
            .lock()
            .map_err(|_| "failed to lock live execution runner state".to_string())?;
        if let Some(prior) = slot.as_ref() {
            prior.abort.cancel();
        }
        *slot = Some(runtime);
    }

    let saved = match super::run_artifacts::create_live_session_artifact_blocking(
        build_live_session_seed(&req, &symbol, qty, &strategy_kind, &data_id),
        app.clone(),
    ) {
        Ok(saved) => saved,
        Err(error) => {
            rollback_runner_start(generation);
            return Err(error);
        }
    };
    let mut session_slot = live_session_audit_slot()
        .lock()
        .map_err(|_| "failed to lock live session audit state".to_string())?;
    *session_slot = Some(LiveSessionAuditRuntime {
        path: saved.path,
        symbol: symbol.clone(),
    });

    emit_live_execution_runner_event(
        &app,
        "started",
        format!(
            "auto-runner started for {symbol} ({strategy_kind}, {}, qty={qty})",
            if req.dry_run { "dry-run" } else { "broker" }
        ),
        symbol,
        None,
        None,
    );
    live_execution_runner_status()
}

fn stop_live_execution_runner_internal(
    app: tauri::AppHandle,
    finalize_session: bool,
) -> Result<StopLiveExecutionRunnerResponse, String> {
    let prior = {
        let mut slot = live_execution_runner_slot()
            .lock()
            .map_err(|_| "failed to lock live execution runner state".to_string())?;
        if let Some(runtime) = slot.as_ref() {
            runtime.abort.cancel();
        }
        slot.take().map(|r| runner_status_from_runtime(Some(&r)))
    };
    let stopped = prior.is_some();
    if stopped {
        let symbol = prior
            .as_ref()
            .and_then(|s| s.config.as_ref().map(|c| c.symbol.clone()))
            .unwrap_or_default();
        emit_live_execution_runner_event(
            &app,
            "stopped",
            "auto-runner stopped",
            symbol,
            None,
            None,
        );
    }
    let status = live_execution_runner_status()?;
    if finalize_session {
        let _ = finalize_active_live_session(&app, "stopped");
    }
    Ok(StopLiveExecutionRunnerResponse {
        stopped,
        prior,
        status,
    })
}

#[tauri::command]
pub fn stop_live_execution_runner(
    app: tauri::AppHandle,
) -> Result<StopLiveExecutionRunnerResponse, String> {
    stop_live_execution_runner_internal(app, true)
}

pub(crate) fn stop_live_execution_runner_for_interruption(
    app: tauri::AppHandle,
) -> Result<StopLiveExecutionRunnerResponse, String> {
    stop_live_execution_runner_internal(app, false)
}

#[tauri::command]
pub fn live_execution_runner_status() -> Result<LiveExecutionRunnerStatus, String> {
    let slot = live_execution_runner_slot()
        .lock()
        .map_err(|_| "failed to lock live execution runner state".to_string())?;
    Ok(runner_status_from_runtime(slot.as_ref()))
}

pub(crate) fn on_live_execution_bar(app: &tauri::AppHandle, bar: LiveExecutionRunnerBar) {
    let mut cfg_to_run: Option<LiveExecutionRunnerInternalCfg> = None;
    let mut generation_to_run: Option<u64> = None;
    let mut abort_to_run: Option<RunnerAbortHandle> = None;
    let mut queued_latest = false;

    {
        let Ok(mut slot) = live_execution_runner_slot().lock() else {
            return;
        };
        let Some(runtime) = slot.as_mut() else {
            return;
        };
        if !symbol_matches(&runtime.cfg.symbol, &bar.symbol) {
            return;
        }
        if runtime
            .last_bar_open_unix_ms
            .map(|t| bar.time_open_unix_ms <= t)
            .unwrap_or(false)
        {
            runtime.skip_count = runtime.skip_count.saturating_add(1);
            return;
        }
        if runtime.inflight {
            runtime.skip_count = runtime.skip_count.saturating_add(1);
            queued_latest = update_pending_bar(&mut runtime.pending_bar, &bar);
        } else {
            runtime.inflight = true;
            runtime.last_bar_open_unix_ms = Some(bar.time_open_unix_ms);
            runtime.last_triggered_at_unix_ms = Some(now_unix_ms());
            runtime.last_error = None;
            cfg_to_run = Some(runtime.cfg.clone());
            generation_to_run = Some(runtime.generation);
            abort_to_run = Some(runtime.abort.clone());
        }
    }

    if queued_latest {
        emit_live_execution_runner_event(
            app,
            "queued_latest",
            "runner queued latest live bar while prior run was still in flight",
            bar.symbol.clone(),
            Some(bar.time_open_unix_ms),
            None,
        );
        return;
    }

    let (Some(cfg), Some(generation), Some(abort)) = (cfg_to_run, generation_to_run, abort_to_run)
    else {
        return;
    };
    emit_live_execution_runner_event(
        app,
        "triggered",
        "runner triggered on new live bar",
        cfg.symbol.clone(),
        Some(bar.time_open_unix_ms),
        None,
    );

    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        if abort.check().is_err() {
            let next_bar = with_active_runner_generation(generation, |runtime| {
                runtime.inflight = false;
                runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                runtime.pending_bar.take()
            });
            if let Some(Some(next)) = next_bar {
                on_live_execution_bar(&app2, next);
            }
            return;
        }
        let state = app2.state::<AppState>();
        if let Err(e) = state.upsert_live_bar(
            &cfg.data_id,
            bar.time_open_unix_ms,
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume,
        ) {
            let next_bar = with_active_runner_generation(generation, |runtime| {
                runtime.inflight = false;
                runtime.error_count = runtime.error_count.saturating_add(1);
                runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                runtime.last_error = Some(e.clone());
                runtime.pending_bar.take()
            });
            if let Some(next_bar) = next_bar {
                emit_live_execution_runner_event(
                    &app2,
                    "error",
                    format!("runner failed to apply live bar: {e}"),
                    cfg.symbol.clone(),
                    Some(bar.time_open_unix_ms),
                    None,
                );
                if let Some(next) = next_bar {
                    on_live_execution_bar(&app2, next);
                }
            }
            return;
        }

        if abort.check().is_err() {
            let next_bar = with_active_runner_generation(generation, |runtime| {
                runtime.inflight = false;
                runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                runtime.pending_bar.take()
            });
            if let Some(Some(next)) = next_bar {
                on_live_execution_bar(&app2, next);
            }
            return;
        }

        let candles = match state.get_candles(&cfg.data_id) {
            Ok(c) => c,
            Err(e) => {
                let next_bar = with_active_runner_generation(generation, |runtime| {
                    runtime.inflight = false;
                    runtime.error_count = runtime.error_count.saturating_add(1);
                    runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                    runtime.last_error = Some(e.clone());
                    runtime.pending_bar.take()
                });
                if let Some(next_bar) = next_bar {
                    emit_live_execution_runner_event(
                        &app2,
                        "error",
                        format!("runner failed to load strategy candles: {e}"),
                        cfg.symbol.clone(),
                        Some(bar.time_open_unix_ms),
                        None,
                    );
                    if let Some(next) = next_bar {
                        on_live_execution_bar(&app2, next);
                    }
                }
                return;
            }
        };

        let req = cfg.req.clone();
        let symbol = cfg.symbol.clone();
        let qty = cfg.qty;
        let app_local_data_dir = cfg.app_local_data_dir.clone();
        let abort_for_blocking = abort.clone();
        let exec_result = tauri::async_runtime::spawn_blocking(move || {
            execute_strategy_signal_blocking(
                req,
                symbol,
                qty,
                candles,
                Some(abort_for_blocking),
                Some(app_local_data_dir),
            )
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);

        match exec_result {
            Ok(resp) => {
                let next_bar = with_active_runner_generation(generation, |runtime| {
                    runtime.inflight = false;
                    runtime.run_count = runtime.run_count.saturating_add(1);
                    runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                    runtime.last_action = Some(resp.action.clone());
                    runtime.last_order_id = resp.order_id.clone();
                    runtime.last_error = None;
                    runtime.pending_bar.take()
                });
                if let Some(next_bar) = next_bar {
                    let status = if resp.order_submitted {
                        "order_submitted"
                    } else {
                        "signal_processed"
                    };
                    let msg = if resp.order_submitted {
                        format!(
                            "runner submitted {} order{}",
                            resp.action,
                            resp.order_id
                                .as_deref()
                                .map(|id| format!(" (id={id})"))
                                .unwrap_or_default()
                        )
                    } else {
                        format!("runner processed signal action={}", resp.action)
                    };
                    emit_live_execution_runner_event(
                        &app2,
                        status,
                        msg,
                        cfg.symbol.clone(),
                        Some(bar.time_open_unix_ms),
                        Some(resp),
                    );
                    if let Some(next) = next_bar {
                        on_live_execution_bar(&app2, next);
                    }
                }
            }
            Err(e) => {
                if is_runner_cancelled_error(&e) {
                    let next_bar = with_active_runner_generation(generation, |runtime| {
                        runtime.inflight = false;
                        runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                        runtime.pending_bar.take()
                    });
                    if let Some(Some(next)) = next_bar {
                        on_live_execution_bar(&app2, next);
                    }
                    return;
                }

                let next_bar = with_active_runner_generation(generation, |runtime| {
                    runtime.inflight = false;
                    runtime.error_count = runtime.error_count.saturating_add(1);
                    runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                    runtime.last_error = Some(e.clone());
                    runtime.pending_bar.take()
                });
                if let Some(next_bar) = next_bar {
                    emit_live_execution_runner_event(
                        &app2,
                        "error",
                        format!("runner execution failed: {e}"),
                        cfg.symbol.clone(),
                        Some(bar.time_open_unix_ms),
                        None,
                    );
                    if let Some(next) = next_bar {
                        on_live_execution_bar(&app2, next);
                    }
                }
            }
        }
    });
}

#[tauri::command]
pub fn alpaca_get_credential_status(app: tauri::AppHandle) -> AlpacaCredentialStorageStatus {
    alpaca_credential_storage_status_for_app(&app.config().identifier)
}

#[tauri::command]
pub fn alpaca_set_api_credentials(
    req: SetAlpacaApiCredentialsRequest,
    app: tauri::AppHandle,
) -> Result<AlpacaCredentialStorageStatus, String> {
    store_alpaca_api_credentials(&app.config().identifier, &req.api_key, &req.api_secret)
}

#[tauri::command]
pub fn alpaca_clear_api_credentials(app: tauri::AppHandle) -> AlpacaCredentialStorageStatus {
    clear_alpaca_api_credentials_for_app(&app.config().identifier)
}

#[tauri::command]
pub fn get_execution_risk_policy(
    app: tauri::AppHandle,
) -> Result<ExecutionRiskPolicyResponse, String> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let policy = settings::configured_execution_risk_policy_settings(&app_local_data_dir)?;
    Ok(ExecutionRiskPolicyResponse { policy })
}

#[tauri::command]
pub fn set_execution_risk_policy(
    policy: ExecutionRiskPolicySettings,
    app: tauri::AppHandle,
) -> Result<ExecutionRiskPolicyResponse, String> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let settings = settings::set_execution_risk_policy_settings(&app_local_data_dir, &policy)?;
    Ok(ExecutionRiskPolicyResponse {
        policy: settings.execution_risk_policy,
    })
}

#[tauri::command]
pub fn reset_execution_risk_policy(
    app: tauri::AppHandle,
) -> Result<ExecutionRiskPolicyResponse, String> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let settings = settings::set_execution_risk_policy_settings(
        &app_local_data_dir,
        &ExecutionRiskPolicySettings::default(),
    )?;
    Ok(ExecutionRiskPolicyResponse {
        policy: settings.execution_risk_policy,
    })
}

#[tauri::command]
pub async fn preview_execution_policy_decision(
    mut req: ExecuteStrategySignalRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<PreviewExecutionPolicyDecisionResponse, String> {
    normalize_execution_target_flags(&mut req);
    let (req, symbol, qty) = normalize_execute_request(req)?;
    let data_id = strategy_data_id(&req.strategy).to_string();
    let candles = state.get_candles(&data_id)?;
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        preview_execution_policy_decision_blocking(req, symbol, qty, candles, app_local_data_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_local_paper_execution_snapshot(
    req: GetLocalPaperExecutionSnapshotRequest,
    app: tauri::AppHandle,
) -> Result<GetLiveExecutionSnapshotResponse, String> {
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let symbol = canonical_symbol_key(&req.symbol);
    let limit = req.limit.unwrap_or(200).clamp(1, 500);
    tauri::async_runtime::spawn_blocking(move || {
        local_paper_snapshot_blocking(&app_local_data_dir, &symbol, limit, now_unix_ms())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_execution_strategy_support_matrix() -> Vec<ExecutionStrategySupportRow> {
    execution_strategy_support_matrix()
}

#[tauri::command]
pub async fn reset_local_paper_broker(
    app: tauri::AppHandle,
) -> Result<ResetLocalPaperBrokerResponse, String> {
    ensure_runner_inactive_for_broker_safety_action()?;
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let snapshot = tauri::async_runtime::spawn_blocking(move || {
        broker_paper::reset(&app_local_data_dir, now_unix_ms())?;
        local_paper_snapshot_blocking(&app_local_data_dir, "", 200, now_unix_ms())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(ResetLocalPaperBrokerResponse { snapshot })
}

#[tauri::command]
pub async fn cancel_local_paper_open_orders(
    req: LiveExecutionSafetyActionRequest,
    app: tauri::AppHandle,
) -> Result<CancelLiveExecutionOpenOrdersResponse, String> {
    ensure_runner_inactive_for_broker_safety_action()?;
    let symbol = normalize_execution_safety_symbol(&req.symbol)?;
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(
        move || -> Result<CancelLiveExecutionOpenOrdersResponse, String> {
            let _ = broker_paper::load_or_default(&app_local_data_dir, now_unix_ms())?;
            Ok(CancelLiveExecutionOpenOrdersResponse {
                mode: LiveExecutionMode::Paper,
                symbol,
                open_order_count_before: 0,
                cancelled_count: 0,
                cancelled_order_ids: Vec::new(),
            })
        },
    )
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn flatten_local_paper_position(
    req: LiveExecutionSafetyActionRequest,
    app: tauri::AppHandle,
) -> Result<FlattenLiveExecutionPositionResponse, String> {
    ensure_runner_inactive_for_broker_safety_action()?;
    let symbol = normalize_execution_safety_symbol(&req.symbol)?;
    let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(
        move || -> Result<FlattenLiveExecutionPositionResponse, String> {
            let doc = broker_paper::load_or_default(&app_local_data_dir, now_unix_ms())?;
            let Some(position) = doc.positions.get(&symbol).cloned() else {
                return Ok(FlattenLiveExecutionPositionResponse {
                    mode: LiveExecutionMode::Paper,
                    symbol,
                    position_found: false,
                    open_order_count_before: 0,
                    order_submitted: false,
                    order_id: None,
                    action: "flat".to_string(),
                    qty: None,
                });
            };
            let qty = position.qty.abs();
            let side = if position.qty > 0.0 {
                PaperOrderSide::Sell
            } else {
                PaperOrderSide::Buy
            };
            let execution = broker_paper::submit_market_order(
                &app_local_data_dir,
                PaperBrokerMarketOrder {
                    symbol: symbol.clone(),
                    side,
                    qty,
                    price: position.market_price.max(0.01),
                    at_unix_ms: now_unix_ms(),
                    fee_bps: 0.0,
                    slippage_bps: 0.0,
                    client_order_id: None,
                },
            )?;
            Ok(FlattenLiveExecutionPositionResponse {
                mode: LiveExecutionMode::Paper,
                symbol,
                position_found: true,
                open_order_count_before: 0,
                order_submitted: true,
                order_id: Some(execution.order.id),
                action: side.as_str().to_string(),
                qty: Some(qty),
            })
        },
    )
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn test_alpaca_connection(
    req: TestAlpacaConnectionRequest,
    app: tauri::AppHandle,
) -> Result<TestAlpacaConnectionResponse, String> {
    let (api_key, api_secret) = resolve_alpaca_credentials_for_app(
        &app.config().identifier,
        &req.alpaca_api_key,
        &req.alpaca_api_secret,
    )?
    .ok_or_else(|| "alpaca_api_key and alpaca_api_secret are required".to_string())?;
    let mode = req.mode;
    let response = tauri::async_runtime::spawn_blocking(
        move || -> Result<TestAlpacaConnectionResponse, String> {
            let client = alpaca_client()?;
            let account = fetch_alpaca_account(client, mode, &api_key, &api_secret)?;
            let fetched_at_unix_ms = now_unix_ms();
            Ok(TestAlpacaConnectionResponse {
                mode,
                account_id: json_field_as_string(&account, "id"),
                account_number: json_field_as_string(&account, "account_number"),
                status: json_field_as_string(&account, "status"),
                currency: json_field_as_string(&account, "currency"),
                buying_power: json_field_as_string(&account, "buying_power"),
                cash: json_field_as_string(&account, "cash"),
                portfolio_value: json_field_as_string(&account, "portfolio_value"),
                fetched_at_unix_ms,
            })
        },
    )
    .await
    .map_err(|e| e.to_string())??;
    let symbol = active_live_session_path()
        .and_then(|_| {
            live_session_audit_slot()
                .lock()
                .ok()
                .and_then(|slot| slot.as_ref().map(|runtime| runtime.symbol.clone()))
        })
        .unwrap_or_default();
    let _ = record_live_session_event(
        &app,
        serde_json::json!({
            "provenance": "local_control",
            "status": "test_connection",
            "message": "broker account connectivity tested",
            "symbol": symbol,
            "at_unix_ms": response.fetched_at_unix_ms,
        }),
    );
    let _ = record_live_session_broker_snapshot(
        &app,
        serde_json::json!({
            "source_command": "test_alpaca_connection",
            "mode": match response.mode {
                LiveExecutionMode::Paper => "paper",
                LiveExecutionMode::Live => "live",
            },
            "symbol": symbol,
            "at_unix_ms": response.fetched_at_unix_ms,
            "account_status": response.status,
            "currency": response.currency,
            "position_state": "suppressed",
            "open_order_count": 0,
            "order_count": 0,
        }),
    );
    Ok(response)
}

#[tauri::command]
pub async fn get_live_execution_snapshot(
    req: GetLiveExecutionSnapshotRequest,
    app: tauri::AppHandle,
) -> Result<GetLiveExecutionSnapshotResponse, String> {
    let (api_key, api_secret) = resolve_alpaca_credentials_for_app(
        &app.config().identifier,
        &req.alpaca_api_key,
        &req.alpaca_api_secret,
    )?
    .ok_or_else(|| "alpaca_api_key and alpaca_api_secret are required".to_string())?;

    let mode = req.mode;
    let symbol = canonical_symbol_key(&req.symbol);
    let return_limit = req.limit.unwrap_or(200).clamp(1, 500);
    let include_account = req.include_account.unwrap_or(true);
    let include_orders = req.include_orders.unwrap_or(true);
    let include_position = req.include_position.unwrap_or(true);

    let (response, open_order_count) = tauri::async_runtime::spawn_blocking(
        move || -> Result<(GetLiveExecutionSnapshotResponse, usize), String> {
            let client = alpaca_client()?;
            let (
                account_id,
                account_number,
                account_status,
                currency,
                buying_power,
                cash,
                portfolio_value,
            ) = if include_account {
                let account = fetch_alpaca_account(client, mode, &api_key, &api_secret)?;
                (
                    json_field_as_string(&account, "id"),
                    json_field_as_string(&account, "account_number"),
                    json_field_as_string(&account, "status"),
                    json_field_as_string(&account, "currency"),
                    json_field_as_string(&account, "buying_power"),
                    json_field_as_string(&account, "cash"),
                    json_field_as_string(&account, "portfolio_value"),
                )
            } else {
                (None, None, None, None, None, None, None)
            };

            let order_scan_limit = if include_orders && !symbol.is_empty() {
                return_limit.max(500)
            } else {
                return_limit
            };
            let mut orders: Vec<LiveOrderSnapshot> = Vec::new();
            let mut open_order_count = 0usize;
            if include_orders {
                let orders_raw =
                    fetch_alpaca_orders(client, mode, &api_key, &api_secret, order_scan_limit)?;
                let (filtered_orders, filtered_open_order_count) =
                    summarize_snapshot_orders(&orders_raw, &symbol, return_limit as usize);
                orders = filtered_orders;
                open_order_count = filtered_open_order_count;
            }

            let mut position: Option<LivePositionSnapshot> = None;
            if include_position {
                if symbol.is_empty() {
                    let positions_raw =
                        fetch_alpaca_positions(client, mode, &api_key, &api_secret)?;
                    for p in &positions_raw {
                        let Some(pos) = normalize_position_snapshot(p) else {
                            continue;
                        };
                        if !symbol_matches(&pos.symbol, &symbol) {
                            continue;
                        }
                        position = Some(pos);
                        break;
                    }
                } else if let Some(position_raw) =
                    fetch_alpaca_position(client, mode, &api_key, &api_secret, &symbol)?
                {
                    position = normalize_position_snapshot(&position_raw);
                }
            }

            let fetched_at_unix_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);

            let response = GetLiveExecutionSnapshotResponse {
                mode,
                symbol,
                account_id,
                account_number,
                account_status,
                currency,
                buying_power,
                cash,
                portfolio_value,
                fetched_at_unix_ms,
                position,
                orders,
            };
            Ok((response, open_order_count))
        },
    )
    .await
    .map_err(|e| e.to_string())??;
    let _ = record_live_session_event(
        &app,
        serde_json::json!({
            "provenance": "local_control",
            "status": "refresh_snapshot",
            "message": "broker snapshot refreshed",
            "symbol": response.symbol,
            "at_unix_ms": response.fetched_at_unix_ms,
        }),
    );
    let position_state = if response.position.is_some() {
        "open"
    } else {
        "flat"
    };
    let position_qty = response.position.as_ref().map(|position| position.qty);
    let position_side = response
        .position
        .as_ref()
        .and_then(|position| position.side.clone())
        .unwrap_or_default();
    let _ = record_live_session_broker_snapshot(
        &app,
        serde_json::json!({
            "source_command": "get_live_execution_snapshot",
            "mode": match response.mode {
                LiveExecutionMode::Paper => "paper",
                LiveExecutionMode::Live => "live",
            },
            "symbol": response.symbol,
            "at_unix_ms": response.fetched_at_unix_ms,
            "account_status": response.account_status,
            "currency": response.currency,
            "position_state": position_state,
            "position_qty": position_qty,
            "position_side": position_side,
            "open_order_count": open_order_count,
            "order_count": response.orders.len(),
        }),
    );
    Ok(response)
}

#[tauri::command]
pub async fn cancel_live_execution_open_orders(
    req: LiveExecutionSafetyActionRequest,
    app: tauri::AppHandle,
) -> Result<CancelLiveExecutionOpenOrdersResponse, String> {
    ensure_runner_inactive_for_broker_safety_action()?;
    let symbol = normalize_execution_safety_symbol(&req.symbol)?;
    let (api_key, api_secret) = resolve_alpaca_credentials_for_app(
        &app.config().identifier,
        &req.alpaca_api_key,
        &req.alpaca_api_secret,
    )?
    .ok_or_else(|| "alpaca_api_key and alpaca_api_secret are required".to_string())?;
    let mode = req.mode;
    let response = tauri::async_runtime::spawn_blocking(
        move || -> Result<CancelLiveExecutionOpenOrdersResponse, String> {
            let client = alpaca_client()?;
            let orders = fetch_alpaca_open_orders_for_symbol(client, mode, &api_key, &api_secret, &symbol)?;
            let open_order_count_before = open_orders_for_symbol(&orders, &symbol).len();
            let open_orders = cancelable_orders_for_symbol(&orders, &symbol);
            let mut cancelled_order_ids = Vec::with_capacity(open_orders.len());
            for order in open_orders {
                if let Err(error) = cancel_alpaca_order(client, mode, &api_key, &api_secret, &order.id) {
                    if cancelled_order_ids.is_empty() {
                        return Err(format!("failed to cancel order {}: {error}", order.id));
                    }
                    return Err(format!(
                        "cancelled {} order(s) before failure on {}: {}. Refresh orders immediately.",
                        cancelled_order_ids.len(),
                        order.id,
                        error
                    ));
                }
                cancelled_order_ids.push(order.id);
            }
            Ok(CancelLiveExecutionOpenOrdersResponse {
                mode,
                symbol,
                open_order_count_before,
                cancelled_count: cancelled_order_ids.len(),
                cancelled_order_ids,
            })
        },
    )
    .await
    .map_err(|e| e.to_string())??;
    let _ = record_live_session_event(
        &app,
        serde_json::json!({
            "provenance": "local_control",
            "status": "cancel_open_orders",
            "message": format!(
                "cancel-open-orders requested for {} (cancelled={})",
                response.symbol,
                response.cancelled_count
            ),
            "symbol": response.symbol,
            "at_unix_ms": now_unix_ms(),
        }),
    );
    Ok(response)
}

#[tauri::command]
pub async fn flatten_live_execution_position(
    req: LiveExecutionSafetyActionRequest,
    app: tauri::AppHandle,
) -> Result<FlattenLiveExecutionPositionResponse, String> {
    ensure_runner_inactive_for_broker_safety_action()?;
    let symbol = normalize_execution_safety_symbol(&req.symbol)?;
    let (api_key, api_secret) = resolve_alpaca_credentials_for_app(
        &app.config().identifier,
        &req.alpaca_api_key,
        &req.alpaca_api_secret,
    )?
    .ok_or_else(|| "alpaca_api_key and alpaca_api_secret are required".to_string())?;
    let mode = req.mode;
    let response = tauri::async_runtime::spawn_blocking(
        move || -> Result<FlattenLiveExecutionPositionResponse, String> {
            let client = alpaca_client()?;
            let orders =
                fetch_alpaca_open_orders_for_symbol(client, mode, &api_key, &api_secret, &symbol)?;
            let open_order_count_before = open_orders_for_symbol(&orders, &symbol).len();
            if open_order_count_before > 0 {
                let plural = if open_order_count_before == 1 { "" } else { "s" };
                return Err(format!(
                    "Cancel {open_order_count_before} open order{plural} for {symbol} before flattening the position."
                ));
            }
            let position = fetch_alpaca_position(client, mode, &api_key, &api_secret, &symbol)?
                .and_then(|value| normalize_position_snapshot(&value))
                .filter(|position| position.qty.abs() > f64::EPSILON);
            let Some(position) = position else {
                return Ok(FlattenLiveExecutionPositionResponse {
                    mode,
                    symbol,
                    position_found: false,
                    open_order_count_before,
                    order_submitted: false,
                    order_id: None,
                    action: String::new(),
                    qty: None,
                });
            };
            let (action, payload) = flatten_position_order_payload(&symbol, position.qty)
                .ok_or_else(|| "position qty is invalid for flattening".to_string())?;
            let order = submit_alpaca_order(client, mode, &api_key, &api_secret, &payload)?;
            let order_id = json_field_as_string(&order, "id")
                .ok_or_else(|| "Alpaca order response missing id".to_string())?;
            Ok(FlattenLiveExecutionPositionResponse {
                mode,
                symbol,
                position_found: true,
                open_order_count_before,
                order_submitted: true,
                order_id: Some(order_id),
                action,
                qty: Some(position.qty.abs()),
            })
        },
    )
    .await
    .map_err(|e| e.to_string())??;
    let _ = record_live_session_event(
        &app,
        serde_json::json!({
            "provenance": "local_control",
            "status": "flatten_position",
            "message": if response.order_submitted {
                format!(
                    "flatten-position submitted {} order{}",
                    response.action,
                    response
                        .order_id
                        .as_deref()
                        .map(|id| format!(" (id={id})"))
                        .unwrap_or_default()
                )
            } else if response.position_found {
                format!("flatten-position requested for {}", response.symbol)
            } else {
                format!("flatten-position found no open position for {}", response.symbol)
            },
            "symbol": response.symbol,
            "at_unix_ms": now_unix_ms(),
            "action": response.action,
            "order_id": response.order_id,
            "delta_qty": response.qty,
            "order_submitted": response.order_submitted,
        }),
    );
    Ok(response)
}

#[tauri::command]
pub fn emergency_halt_live_execution(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<EmergencyHaltLiveExecutionResponse, String> {
    let live_before = state.live_candle_stream_info()?;
    let halt_symbol = live_before
        .as_ref()
        .map(|info| info.symbol.clone())
        .or_else(|| {
            live_execution_runner_slot()
                .lock()
                .ok()
                .and_then(|slot| slot.as_ref().map(|runtime| runtime.cfg.symbol.clone()))
        })
        .unwrap_or_default();
    let _ = record_live_session_event(
        &app,
        serde_json::json!({
            "provenance": "local_control",
            "status": "emergency_halt",
            "message": "emergency halt requested",
            "symbol": halt_symbol,
            "at_unix_ms": now_unix_ms(),
        }),
    );
    let runner = stop_live_execution_runner_internal(app.clone(), false)?;
    let live_prior = state.stop_live_candle_stream()?;
    let live_symbol = live_prior
        .as_ref()
        .map(|info| info.symbol.clone())
        .or_else(|| live_before.as_ref().map(|info| info.symbol.clone()))
        .unwrap_or_default();
    let _ = finalize_active_live_session(&app, "halted");
    Ok(EmergencyHaltLiveExecutionResponse {
        runner_stopped: runner.stopped,
        live_stopped: live_prior.is_some(),
        live_symbol,
        prior_runner: runner.prior,
        requires_manual_review: true,
        status_message: emergency_halt_status_message(runner.stopped, live_prior.is_some()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use ta_app_core::{DoubleMaDrilldownRequest, DoubleMaParamsResolved};
    use ta_strategies::double_ma::StrategyConfig;

    #[test]
    fn alpaca_keyring_service_name_scopes_to_app_identifier() {
        assert_eq!(
            alpaca_keyring_service_name("dev.vectoralpha.vectorgrid"),
            "dev.vectoralpha.vectorgrid.alpaca.execution"
        );
        assert_eq!(
            alpaca_keyring_service_name(""),
            "vectorgrid.alpaca.execution"
        );
    }

    #[test]
    fn alpaca_credential_storage_status_tracks_presence_bits() {
        let missing = build_alpaca_credential_storage_status(true, false, false, None);
        assert_eq!(missing.state, AlpacaCredentialStorageState::Missing);
        assert!(!missing.has_credentials);
        assert_eq!(missing.storage_backend.as_deref(), Some("keyring"));

        let incomplete = build_alpaca_credential_storage_status(true, true, false, None);
        assert_eq!(incomplete.state, AlpacaCredentialStorageState::Incomplete);
        assert!(!incomplete.has_credentials);

        let stored = build_alpaca_credential_storage_status(true, true, true, None);
        assert_eq!(stored.state, AlpacaCredentialStorageState::Stored);
        assert!(stored.has_credentials);

        let unavailable = build_alpaca_credential_storage_status(
            false,
            false,
            false,
            Some("Secure credential storage is unavailable on this system.".to_string()),
        );
        assert_eq!(unavailable.state, AlpacaCredentialStorageState::Unavailable);
        assert!(!unavailable.remember_available);
        assert!(unavailable.storage_backend.is_none());
        assert_eq!(
            unavailable.message.as_deref(),
            Some("Secure credential storage is unavailable on this system.")
        );
    }

    #[test]
    fn store_alpaca_api_credentials_rejects_empty_values_before_secure_storage() {
        let err = store_alpaca_api_credentials("dev.vectoralpha.vectorgrid.test", "", "secret")
            .expect_err("empty api key should fail");
        assert!(err.contains("Both Alpaca API key and secret are required."));
    }

    #[test]
    fn normalize_alpaca_typed_override_rejects_partial_pairs() {
        let err = normalize_alpaca_typed_override("key_only", "")
            .expect_err("partial override should fail");
        assert!(err.contains("Both alpaca_api_key and alpaca_api_secret are required"));
        assert_eq!(
            normalize_alpaca_typed_override("key", "secret").expect("full override"),
            Some(("key".to_string(), "secret".to_string()))
        );
        assert_eq!(
            normalize_alpaca_typed_override("", "").expect("blank override"),
            None
        );
    }

    #[test]
    fn build_live_session_seed_preserves_mode_and_source_run_context() {
        let mut req = test_execute_request(" aapl ", 2.5);
        req.mode = LiveExecutionMode::Live;
        req.dry_run = false;
        req.param_source = "best".to_string();
        req.source_run_artifact_path =
            "C:/runs/vectorgrid_run_2026-04-12T15-00-00Z.json".to_string();
        req.source_run_config_fingerprint = "cfg1-fnv1a64-seed".to_string();
        req.source_run_route_provenance = Some(json!({
            "requested_route": "GpuOnly",
            "resolved_route": "GpuKernel",
            "realized_route": "CpuOnly",
            "route_changed": true,
            "fallback_reason": "cuda unavailable",
        }));

        let seed = build_live_session_seed(&req, "AAPL", 2.5, "DoubleMa", "data_1");

        assert_eq!(seed.get("mode").and_then(Value::as_str), Some("live"));
        assert_eq!(seed.get("symbol").and_then(Value::as_str), Some("AAPL"));
        assert_eq!(seed.get("qty").and_then(Value::as_f64), Some(2.5));
        assert_eq!(seed.get("dry_run").and_then(Value::as_bool), Some(false));
        assert_eq!(
            seed.get("param_source").and_then(Value::as_str),
            Some("best")
        );
        assert_eq!(
            seed.get("source_run_artifact_path").and_then(Value::as_str),
            Some("C:/runs/vectorgrid_run_2026-04-12T15-00-00Z.json")
        );
        assert_eq!(
            seed.get("source_run_config_fingerprint")
                .and_then(Value::as_str),
            Some("cfg1-fnv1a64-seed")
        );
        assert_eq!(
            seed.pointer("/source_run_route_provenance/realized_route")
                .and_then(Value::as_str),
            Some("CpuOnly")
        );
        assert_eq!(
            seed.get("session_state").and_then(Value::as_str),
            Some("active")
        );
        assert!(seed
            .get("session_id")
            .and_then(Value::as_str)
            .is_some_and(|value| value.starts_with("live-session-")));
    }

    #[test]
    fn broker_open_order_status_matches_only_open_like_states() {
        for status in [
            "new",
            "accepted",
            "pending_new",
            "partially_filled",
            "pending_cancel",
            "pending_replace",
            "held",
            "stopped",
            "suspended",
            "calculated",
        ] {
            assert!(
                is_broker_open_order_status(status),
                "expected open-like status {status} to match"
            );
        }
        for status in ["filled", "canceled", "expired", "done_for_day", "replaced"] {
            assert!(
                !is_broker_open_order_status(status),
                "expected terminal status {status} to be excluded"
            );
        }
    }

    #[test]
    fn broker_cancelable_status_is_stricter_than_action_blocking_status() {
        assert!(!is_broker_cancelable_order_status("pending_cancel"));
        assert!(is_broker_open_order_status("pending_cancel"));
        assert!(!is_broker_cancelable_order_status("pending_replace"));
        assert!(is_broker_open_order_status("pending_replace"));
        assert!(!is_broker_cancelable_order_status("held"));
        assert!(is_broker_open_order_status("held"));
        assert!(is_broker_cancelable_order_status("partially_filled"));
    }

    #[test]
    fn broker_terminal_status_matches_alpaca_terminal_lifecycle() {
        for status in ["filled", "canceled", "expired", "rejected", "replaced"] {
            assert!(
                is_broker_terminal_order_status(status),
                "expected terminal status {status}"
            );
        }
        for status in ["new", "accepted", "partially_filled", "pending_cancel"] {
            assert!(
                !is_broker_terminal_order_status(status),
                "expected non-terminal status {status}"
            );
        }
    }

    #[test]
    fn broker_status_classifier_marks_review_required_states() {
        assert_eq!(
            classify_broker_order_status("pending_replace"),
            BrokerOrderStatusClass::OpenNonCancelable
        );
        assert_eq!(
            classify_broker_order_status("done_for_day"),
            BrokerOrderStatusClass::DoneForDayReviewRequired
        );
        assert_eq!(
            classify_broker_order_status("held"),
            BrokerOrderStatusClass::SuspendedReviewRequired
        );
        assert_eq!(
            classify_broker_order_status(""),
            BrokerOrderStatusClass::UnknownReviewRequired
        );
        assert!(broker_status_class_requires_review(
            BrokerOrderStatusClass::DoneForDayReviewRequired
        ));
        assert!(broker_status_class_requires_review(
            BrokerOrderStatusClass::UnknownReviewRequired
        ));
    }

    #[test]
    fn alpaca_intent_status_tracks_reconciled_order_lifecycle() {
        let mut order = LiveOrderSnapshot {
            id: "ord_1".to_string(),
            client_order_id: Some("vg-1".to_string()),
            symbol: "AAPL".to_string(),
            side: "buy".to_string(),
            status: "new".to_string(),
            order_type: "market".to_string(),
            qty: Some(1.0),
            filled_qty: Some(0.0),
            filled_avg_price: None,
            limit_price: None,
            stop_price: None,
            submitted_at: None,
            filled_at: None,
            updated_at: None,
            canceled_at: None,
        };

        assert_eq!(
            alpaca_intent_status_for_order_snapshot(&order),
            "broker_open"
        );
        order.status = "partially_filled".to_string();
        assert_eq!(
            alpaca_intent_status_for_order_snapshot(&order),
            "partial_fill"
        );
        order.status = "filled".to_string();
        assert_eq!(
            alpaca_intent_status_for_order_snapshot(&order),
            "terminal_filled_review_required"
        );
        order.status = "rejected".to_string();
        assert_eq!(
            alpaca_intent_status_for_order_snapshot(&order),
            "terminal_rejected_review_required"
        );
        order.status = "done_for_day".to_string();
        assert_eq!(
            alpaca_intent_status_for_order_snapshot(&order),
            "done_for_day_review_required"
        );
    }

    #[test]
    fn broker_reconciliation_event_captures_partial_and_terminal_order_state() {
        let mut order = LiveOrderSnapshot {
            id: "ord_1".to_string(),
            client_order_id: Some("vg-1".to_string()),
            symbol: "AAPL".to_string(),
            side: "buy".to_string(),
            status: "partially_filled".to_string(),
            order_type: "market".to_string(),
            qty: Some(2.0),
            filled_qty: Some(1.0),
            filled_avg_price: Some(101.25),
            limit_price: None,
            stop_price: None,
            submitted_at: None,
            filled_at: None,
            updated_at: None,
            canceled_at: None,
        };

        let partial = broker_reconciliation_event_from_snapshot(&order, false, true);
        assert_eq!(partial.provenance, "broker_reconciliation");
        assert_eq!(partial.status, "partial_fill");
        assert_eq!(
            partial.broker_order_status.as_deref(),
            Some("partially_filled")
        );
        assert_eq!(partial.filled_qty, Some(1.0));
        assert_eq!(partial.requires_broker_review, Some(true));
        assert_eq!(
            partial.rearm_blocked_reason.as_deref(),
            Some("partial_fill_requires_broker_review_before_rearm")
        );
        assert!(partial.timed_out);
        assert!(!partial.terminal);

        order.status = "filled".to_string();
        order.filled_qty = Some(2.0);
        let filled = broker_reconciliation_event_from_snapshot(&order, true, false);
        assert_eq!(filled.status, "terminal_filled_review_required");
        assert!(filled.terminal);
        assert!(!filled.timed_out);
        assert_eq!(filled.refreshed_position_qty, None);
        assert_eq!(filled.requires_broker_review, Some(true));

        let refreshed = broker_position_refreshed_event(
            Some("ord_1".to_string()),
            Some("vg-1".to_string()),
            2.0,
        );
        assert_eq!(refreshed.status, "position_refreshed_after_terminal");
        assert_eq!(refreshed.refreshed_position_qty, Some(2.0));
        assert_eq!(refreshed.requires_broker_review, Some(false));

        let value = broker_reconciliation_event_value("AAPL", "buy", &filled);
        assert_eq!(value.get("symbol").and_then(Value::as_str), Some("AAPL"));
        assert_eq!(value.get("action").and_then(Value::as_str), Some("buy"));
        assert_eq!(
            value.get("provenance").and_then(Value::as_str),
            Some("broker_reconciliation")
        );
    }

    #[test]
    fn open_orders_for_symbol_filters_to_open_current_symbol_rows() {
        let orders = vec![
            json!({ "id": "ord_1", "symbol": "AAPL", "status": "new", "side": "buy", "type": "market" }),
            json!({ "id": "ord_2", "symbol": "AAPL", "status": "filled", "side": "buy", "type": "market" }),
            json!({ "id": "ord_3", "symbol": "TSLA", "status": "accepted", "side": "sell", "type": "limit" }),
        ];

        let filtered = open_orders_for_symbol(&orders, "AAPL");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "ord_1");
        assert_eq!(filtered[0].symbol, "AAPL");
    }

    #[test]
    fn cancelable_orders_for_symbol_uses_cancelable_status_subset() {
        let orders = vec![
            json!({ "id": "ord_1", "symbol": "AAPL", "status": "pending_cancel", "side": "buy", "type": "market" }),
            json!({ "id": "ord_2", "symbol": "AAPL", "status": "held", "side": "buy", "type": "market" }),
        ];

        let filtered = cancelable_orders_for_symbol(&orders, "AAPL");

        assert!(filtered.is_empty());
    }

    #[test]
    fn guard_open_orders_before_fresh_delta_reconciles_target_already_in_flight() {
        let open_orders = vec![LiveOrderSnapshot {
            id: "ord_1".to_string(),
            client_order_id: None,
            symbol: "AAPL".to_string(),
            side: "buy".to_string(),
            status: "new".to_string(),
            order_type: "market".to_string(),
            qty: Some(3.0),
            filled_qty: Some(1.0),
            filled_avg_price: None,
            limit_price: None,
            stop_price: None,
            submitted_at: None,
            filled_at: None,
            updated_at: None,
            canceled_at: None,
        }];

        let guard = guard_open_orders_before_fresh_delta("AAPL", 1.0, 3.0, &open_orders)
            .expect("matching open order should reconcile instead of blocking");

        assert_eq!(
            guard,
            OpenOrderDeltaGuard::Reconciled {
                projected_position_qty: 3.0,
                warning: "Reusing 1 open order already working toward the target position for AAPL; no new market delta submitted.".to_string(),
            }
        );
    }

    #[test]
    fn guard_open_orders_before_fresh_delta_blocks_conflicting_open_orders() {
        let open_orders = vec![LiveOrderSnapshot {
            id: "ord_1".to_string(),
            client_order_id: None,
            symbol: "AAPL".to_string(),
            side: "sell".to_string(),
            status: "new".to_string(),
            order_type: "market".to_string(),
            qty: Some(1.0),
            filled_qty: Some(0.0),
            filled_avg_price: None,
            limit_price: None,
            stop_price: None,
            submitted_at: None,
            filled_at: None,
            updated_at: None,
            canceled_at: None,
        }];

        let err = guard_open_orders_before_fresh_delta("AAPL", 1.0, 2.0, &open_orders)
            .expect_err("conflicting open order should block a fresh delta");
        assert!(err.contains("Open orders already exist for AAPL."));
        assert!(err.contains("projected_position_qty_with_open_orders=0.000000"));
    }

    #[test]
    fn guard_open_orders_before_fresh_delta_blocks_when_qty_metadata_is_missing() {
        let open_orders = vec![LiveOrderSnapshot {
            id: "ord_1".to_string(),
            client_order_id: None,
            symbol: "AAPL".to_string(),
            side: "buy".to_string(),
            status: "new".to_string(),
            order_type: "market".to_string(),
            qty: None,
            filled_qty: Some(0.0),
            filled_avg_price: None,
            limit_price: None,
            stop_price: None,
            submitted_at: None,
            filled_at: None,
            updated_at: None,
            canceled_at: None,
        }];

        let err = guard_open_orders_before_fresh_delta("AAPL", 0.0, 1.0, &open_orders)
            .expect_err("missing broker qty metadata should block a fresh delta");
        assert!(err.contains("incomplete quantity metadata"));
    }

    #[test]
    fn alpaca_orders_url_can_scope_open_queries_to_a_symbol() {
        let url = build_alpaca_orders_url(LiveExecutionMode::Paper, "open", 900, Some("AAPL"))
            .expect("orders url");

        assert_eq!(url.as_str(), "https://paper-api.alpaca.markets/v2/orders?status=open&direction=desc&limit=500&symbols=AAPL");
    }

    #[test]
    fn alpaca_client_order_lookup_url_uses_query_parameter() {
        let url = build_alpaca_order_by_client_id_url(LiveExecutionMode::Paper, "vg-abc123")
            .expect("client-order lookup url");

        assert_eq!(
            url.as_str(),
            "https://paper-api.alpaca.markets/v2/orders:by_client_order_id?client_order_id=vg-abc123"
        );
    }

    #[test]
    fn alpaca_client_order_id_is_stable_for_same_execution_intent() {
        let mut req = test_execute_request("AAPL", 1.0);
        req.dry_run = false;
        req.execution_target = Some(ExecutionBrokerTarget::AlpacaPaper);
        req.source_run_config_fingerprint = "cfg1-fnv1a64-demo".to_string();
        let decision = SignalDecision {
            desired_position: 1,
            cfg: StrategyConfig::default(),
            last_bar_ts: Some(1_700_000_000_000),
            last_close: 100.0,
        };

        let first = alpaca_client_order_id(&req, "AAPL", &decision, 1.0, 1.0);
        let second = alpaca_client_order_id(&req, "AAPL", &decision, 1.0, 1.0);
        let changed_delta = alpaca_client_order_id(&req, "AAPL", &decision, 1.0, 0.5);

        assert_eq!(first, second);
        assert!(first.starts_with("vg-"));
        assert_ne!(first, changed_delta);
        assert!(first.len() <= 48);
    }

    #[test]
    fn alpaca_order_intent_is_persisted_without_credentials_and_can_update_status() {
        let mut req = test_execute_request("AAPL", 1.0);
        req.dry_run = false;
        req.execution_target = Some(ExecutionBrokerTarget::AlpacaPaper);
        req.alpaca_api_key = "key_should_not_be_written".to_string();
        req.alpaca_api_secret = "secret_should_not_be_written".to_string();
        req.source_run_config_fingerprint = "cfg1".to_string();
        let decision = SignalDecision {
            desired_position: 1,
            cfg: StrategyConfig::default(),
            last_bar_ts: Some(1_700_000_000_000),
            last_close: 100.0,
        };
        let payload = json!({
            "symbol": "AAPL",
            "qty": "1.000000",
            "side": "buy",
            "type": "market",
            "time_in_force": "day",
            "client_order_id": "vg-test-intent"
        });
        let base = unique_test_dir("alpaca-intent");
        fs::create_dir_all(&base).expect("create temp intent dir");

        let pending = persist_pending_alpaca_order_intent(
            &base,
            build_pending_alpaca_order_intent(
                &req,
                "AAPL",
                &decision,
                0.0,
                1.0,
                1.0,
                "vg-test-intent",
                &payload,
                100,
            ),
        )
        .expect("persist pending intent");

        assert_eq!(pending.status, "pending_submit");
        assert_eq!(pending.attempt_count, 1);
        let path = alpaca_intent_record_path(&base, "vg-test-intent");
        let raw = fs::read_to_string(&path).expect("read persisted intent");
        assert!(raw.contains("\"client_order_id\": \"vg-test-intent\""));
        assert!(!raw.contains("key_should_not_be_written"));
        assert!(!raw.contains("secret_should_not_be_written"));

        let updated = update_alpaca_order_intent_status(
            &base,
            "vg-test-intent",
            "submitted",
            Some("ord_123".to_string()),
            None,
            200,
        )
        .expect("update intent status");

        assert_eq!(updated.status, "submitted");
        assert_eq!(updated.broker_order_id.as_deref(), Some("ord_123"));
        assert_eq!(updated.submitted_at_unix_ms, Some(200));

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn alpaca_order_intent_attempt_count_increments_for_retry() {
        let mut req = test_execute_request("AAPL", 1.0);
        req.dry_run = false;
        req.execution_target = Some(ExecutionBrokerTarget::AlpacaPaper);
        let decision = SignalDecision {
            desired_position: 1,
            cfg: StrategyConfig::default(),
            last_bar_ts: Some(1_700_000_000_000),
            last_close: 100.0,
        };
        let payload = json!({
            "symbol": "AAPL",
            "qty": "1.000000",
            "side": "buy",
            "type": "market",
            "time_in_force": "day",
            "client_order_id": "vg-retry-intent"
        });
        let base = unique_test_dir("alpaca-intent-retry");
        fs::create_dir_all(&base).expect("create temp intent dir");

        let first = persist_pending_alpaca_order_intent(
            &base,
            build_pending_alpaca_order_intent(
                &req,
                "AAPL",
                &decision,
                0.0,
                1.0,
                1.0,
                "vg-retry-intent",
                &payload,
                100,
            ),
        )
        .expect("persist first intent");
        let retry = persist_pending_alpaca_order_intent(
            &base,
            build_pending_alpaca_order_intent(
                &req,
                "AAPL",
                &decision,
                0.0,
                1.0,
                1.0,
                "vg-retry-intent",
                &payload,
                150,
            ),
        )
        .expect("persist retry intent");

        assert_eq!(first.attempt_count, 1);
        assert_eq!(retry.attempt_count, 2);
        assert_eq!(retry.created_at_unix_ms, 100);
        assert_eq!(retry.updated_at_unix_ms, 150);

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn summarize_snapshot_orders_counts_full_filtered_open_set_before_truncation() {
        let orders = vec![
            json!({ "id": "ord_1", "symbol": "AAPL", "status": "new", "side": "buy", "type": "market" }),
            json!({ "id": "ord_2", "symbol": "AAPL", "status": "pending_cancel", "side": "buy", "type": "market" }),
            json!({ "id": "ord_3", "symbol": "AAPL", "status": "filled", "side": "buy", "type": "market" }),
        ];

        let (filtered, open_count) = summarize_snapshot_orders(&orders, "AAPL", 1);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "ord_1");
        assert_eq!(open_count, 2);
    }

    #[test]
    fn manual_execution_audit_failure_becomes_warning_only_after_submission() {
        let response = ExecuteStrategySignalResponse {
            symbol: "AAPL".to_string(),
            mode: LiveExecutionMode::Paper,
            desired_position: 1,
            current_position_qty: 0.0,
            target_position_qty: 1.0,
            delta_qty: 1.0,
            order_submitted: true,
            order_id: Some("ord_123".to_string()),
            client_order_id: None,
            action: "buy".to_string(),
            last_bar_ts: None,
            last_close: Some(123.0),
            broker_reconciliation_events: Vec::new(),
            warnings: Vec::new(),
        };

        let audited = apply_manual_execution_audit_result(
            response,
            Err("artifact append failed".to_string()),
        )
        .expect("submitted order should still succeed");

        assert_eq!(audited.order_id.as_deref(), Some("ord_123"));
        assert_eq!(audited.warnings.len(), 1);
        assert!(audited.warnings[0].contains("artifact append failed"));
    }

    #[test]
    fn rollback_runner_start_only_clears_matching_generation() {
        let _guard = runner_test_guard().lock().expect("runner test guard");
        {
            let mut slot = live_execution_runner_slot().lock().expect("runner slot");
            *slot = Some(test_runner_runtime(41));
        }

        rollback_runner_start(99);
        assert_eq!(
            live_execution_runner_slot()
                .lock()
                .expect("runner slot")
                .as_ref()
                .map(|runtime| runtime.generation),
            Some(41)
        );

        rollback_runner_start(41);
        assert!(live_execution_runner_slot()
            .lock()
            .expect("runner slot")
            .is_none());
    }

    #[test]
    fn flatten_position_order_payload_handles_long_and_short_positions() {
        let long_payload =
            flatten_position_order_payload("AAPL", 2.5).expect("long payload should exist");
        assert_eq!(long_payload.0, "sell");
        assert_eq!(
            long_payload.1.get("side").and_then(Value::as_str),
            Some("sell")
        );
        assert_eq!(
            long_payload.1.get("qty").and_then(Value::as_str),
            Some("2.500000")
        );

        let short_payload =
            flatten_position_order_payload("AAPL", -1.25).expect("short payload should exist");
        assert_eq!(short_payload.0, "buy");
        assert_eq!(
            short_payload.1.get("side").and_then(Value::as_str),
            Some("buy")
        );
        assert_eq!(
            short_payload.1.get("qty").and_then(Value::as_str),
            Some("1.250000")
        );

        assert!(flatten_position_order_payload("AAPL", 0.0).is_none());
    }

    #[test]
    fn normalize_execution_safety_symbol_requires_non_blank_symbol() {
        assert_eq!(
            normalize_execution_safety_symbol(" aapl ").expect("normalized symbol"),
            "AAPL"
        );
        assert!(normalize_execution_safety_symbol("   ").is_err());
    }

    #[test]
    fn emergency_halt_status_message_reflects_runner_and_feed_state() {
        assert!(
            emergency_halt_status_message(true, true).contains("Live feed and auto-runner stopped")
        );
        assert!(emergency_halt_status_message(true, false).contains("Auto-runner stopped"));
        assert!(emergency_halt_status_message(false, true).contains("Live feed stopped"));
        assert_eq!(
            emergency_halt_status_message(false, false),
            "Live automation was already disarmed."
        );
    }

    #[test]
    fn ensure_runner_inactive_for_broker_safety_action_rejects_active_runner() {
        let _guard = runner_test_guard().lock().expect("runner test guard");
        {
            let mut slot = live_execution_runner_slot().lock().expect("runner slot");
            *slot = Some(test_runner_runtime(99));
        }

        let err = ensure_runner_inactive_for_broker_safety_action()
            .expect_err("active runner should block broker safety actions");
        assert!(err.contains("Stop the auto-runner for AAPL before broker safety actions."));

        {
            let mut slot = live_execution_runner_slot().lock().expect("runner slot");
            *slot = None;
        }

        ensure_runner_inactive_for_broker_safety_action()
            .expect("inactive runner state should allow broker safety actions");
    }

    #[test]
    fn ensure_runner_inactive_for_manual_broker_execution_blocks_only_live_broker_runs() {
        let _guard = runner_test_guard().lock().expect("runner test guard");
        {
            let mut slot = live_execution_runner_slot().lock().expect("runner slot");
            *slot = Some(test_runner_runtime(77));
        }

        let err = ensure_runner_inactive_for_manual_broker_execution(false)
            .expect_err("active runner should block live manual broker execution");
        assert!(err.contains("Stop the auto-runner for AAPL before manual broker execution."));
        ensure_runner_inactive_for_manual_broker_execution(true)
            .expect("dry-run manual execution should remain allowed");

        {
            let mut slot = live_execution_runner_slot().lock().expect("runner slot");
            *slot = None;
        }

        ensure_runner_inactive_for_manual_broker_execution(false)
            .expect("inactive runner state should allow live manual broker execution");
    }

    fn unique_test_app_identifier(suffix: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos();
        format!("dev.vectoralpha.vectorgrid.test.{suffix}.{nanos}")
    }

    fn unique_test_dir(suffix: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("vectorgrid-{suffix}-{nanos}"))
    }

    #[test]
    fn resolve_alpaca_credentials_reads_stored_pair_by_default() {
        let app_identifier = unique_test_app_identifier("resolve-default");
        let _ = clear_alpaca_api_credentials_for_app(&app_identifier);
        store_alpaca_api_credentials(&app_identifier, "stored_key", "stored_secret")
            .expect("store secure credentials");

        let resolved =
            resolve_alpaca_credentials_for_app(&app_identifier, "", "").expect("resolve default");

        assert_eq!(
            resolved,
            Some(("stored_key".to_string(), "stored_secret".to_string()))
        );

        let _ = clear_alpaca_api_credentials_for_app(&app_identifier);
    }

    #[test]
    fn resolve_alpaca_credentials_prefers_complete_typed_override() {
        let app_identifier = unique_test_app_identifier("resolve-override");
        let _ = clear_alpaca_api_credentials_for_app(&app_identifier);
        store_alpaca_api_credentials(&app_identifier, "stored_key", "stored_secret")
            .expect("store secure credentials");

        let resolved =
            resolve_alpaca_credentials_for_app(&app_identifier, "typed_key", "typed_secret")
                .expect("resolve complete typed override");

        assert_eq!(
            resolved,
            Some(("typed_key".to_string(), "typed_secret".to_string()))
        );

        let _ = clear_alpaca_api_credentials_for_app(&app_identifier);
    }

    fn runner_test_guard() -> &'static Mutex<()> {
        static RUNNER_TEST_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        RUNNER_TEST_GUARD.get_or_init(|| Mutex::new(()))
    }

    fn test_execute_request(symbol: &str, qty: f64) -> ExecuteStrategySignalRequest {
        ExecuteStrategySignalRequest {
            symbol: symbol.to_string(),
            qty,
            mode: LiveExecutionMode::Paper,
            dry_run: true,
            execution_target: None,
            risk: LiveRiskOverride::default(),
            alpaca_api_key: String::new(),
            alpaca_api_secret: String::new(),
            param_source: "selected".to_string(),
            source_run_artifact_path: String::new(),
            source_run_config_fingerprint: String::new(),
            source_run_route_provenance: None,
            strategy: LiveStrategySpec::DoubleMa {
                req: DoubleMaDrilldownRequest {
                    data_id: "data_1".to_string(),
                    backend_hint: None,
                    followup_cache_key: None,
                    params: DoubleMaParamsResolved {
                        fast_len: 5,
                        slow_len: 20,
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "alma".to_string(),
                        fast_ma_params: None,
                        slow_ma_params: None,
                    },
                    ma_source: "close".to_string(),
                    fast_ma_params: None,
                    slow_ma_params: None,
                    strategy: StrategyConfig::default(),
                    halftrend_filter: None,
                    bins: None,
                },
            },
        }
    }

    #[test]
    fn alpaca_account_tradeable_validation_blocks_non_active_or_blocked_accounts() {
        validate_alpaca_account_tradeable(&json!({
            "status": "ACTIVE",
            "trading_blocked": false,
            "account_blocked": false
        }))
        .expect("active unblocked account should pass");

        let err = validate_alpaca_account_tradeable(&json!({ "status": "ACCOUNT_UPDATED" }))
            .expect_err("non-active account should block");
        assert!(err.contains("requires ACTIVE"));

        let err = validate_alpaca_account_tradeable(&json!({
            "status": "ACTIVE",
            "trading_blocked": true
        }))
        .expect_err("trading_blocked account should block");
        assert!(err.contains("trading_blocked"));
    }

    #[test]
    fn alpaca_market_clock_validation_requires_open_session() {
        validate_alpaca_market_clock_open(&json!({ "is_open": true }))
            .expect("open market clock should pass");

        let err = validate_alpaca_market_clock_open(&json!({
            "is_open": false,
            "next_open": "2026-04-24T13:30:00Z"
        }))
        .expect_err("closed market clock should block live broker submission");
        assert!(err.contains("market clock is closed"));
        assert!(err.contains("next_open=2026-04-24T13:30:00Z"));
        assert_eq!(
            build_alpaca_clock_url(LiveExecutionMode::Paper),
            "https://paper-api.alpaca.markets/v2/clock"
        );
    }

    #[test]
    fn live_execution_source_context_requires_fingerprint_for_alpaca_live() {
        let mut req = test_execute_request("AAPL", 1.0);
        req.dry_run = false;
        req.execution_target = Some(ExecutionBrokerTarget::AlpacaLive);

        let err = validate_live_execution_source_context(&req, ExecutionBrokerTarget::AlpacaLive)
            .expect_err("Alpaca live should require source fingerprint");
        assert!(err.contains("source-run config fingerprint"));

        req.source_run_config_fingerprint = "cfg1-fnv1a64-live".to_string();
        validate_execution_source_preconditions(&req, ExecutionBrokerTarget::AlpacaLive)
            .expect("fingerprinted live source should pass");
        req.source_run_config_fingerprint.clear();
        validate_execution_source_preconditions(&req, ExecutionBrokerTarget::AlpacaPaper)
            .expect("paper target should not require live validation fingerprint");
        validate_execution_source_preconditions(&req, ExecutionBrokerTarget::LocalPaper)
            .expect("local paper target should not require live validation fingerprint");
        validate_execution_source_preconditions(&req, ExecutionBrokerTarget::DryRun)
            .expect("dry-run target should not require live validation fingerprint");
    }

    #[test]
    fn execution_strategy_support_matrix_is_code_backed_and_fail_closed() {
        let matrix = execution_strategy_support_matrix();
        let supported: Vec<_> = matrix
            .iter()
            .filter(|row| row.supported)
            .map(|row| row.strategy_kind.as_str())
            .collect();
        assert_eq!(supported, vec!["DoubleMa", "CompositeCpu"]);
        assert!(matrix.iter().all(|row| !row.adapter.trim().is_empty()));
        assert!(matrix
            .iter()
            .all(|row| row.support_level == "signal_and_delta"));
    }

    #[test]
    fn unresolved_alpaca_intent_blocks_new_client_order_for_same_symbol() {
        let base = unique_test_dir("unresolved-intents");
        fs::create_dir_all(&base).expect("create temp intent dir");
        let mut req = test_execute_request("AAPL", 1.0);
        req.dry_run = false;
        req.execution_target = Some(ExecutionBrokerTarget::AlpacaPaper);
        let decision = SignalDecision {
            desired_position: 1,
            cfg: StrategyConfig::default(),
            last_bar_ts: Some(1_700_000_000_000),
            last_close: 100.0,
        };
        let payload = json!({
            "symbol": "AAPL",
            "qty": "1.000000",
            "side": "buy",
            "type": "market",
            "time_in_force": "day",
            "client_order_id": "vg-existing"
        });
        persist_pending_alpaca_order_intent(
            &base,
            build_pending_alpaca_order_intent(
                &req,
                "AAPL",
                &decision,
                0.0,
                1.0,
                1.0,
                "vg-existing",
                &payload,
                1_700_000_000_001,
            ),
        )
        .expect("persist unresolved intent");

        block_unresolved_alpaca_intents_before_submit(&base, "AAPL", "vg-existing")
            .expect("retrying same client order id remains allowed for idempotent lookup");

        let err = block_unresolved_alpaca_intents_before_submit(&base, "AAPL", "vg-new")
            .expect_err("different client order should be blocked while intent is unresolved");
        assert!(err.contains("Unresolved Alpaca intent exists"));
        assert!(err.contains("vg-existing"));

        update_alpaca_order_intent_status(
            &base,
            "vg-existing",
            "filled",
            Some("ord_1".to_string()),
            None,
            1_700_000_000_002,
        )
        .expect("mark terminal");
        block_unresolved_alpaca_intents_before_submit(&base, "AAPL", "vg-new")
            .expect("terminal prior intent should not block new client order");
    }

    #[test]
    fn unresolved_alpaca_intent_statuses_fail_closed_for_review_required_states() {
        for status in [
            "pending_submit",
            "submitted",
            "reused_existing",
            "broker_open",
            "partial_fill",
            "reconciliation_unknown",
            "reconciliation_error",
            "submit_error",
            "submit_error_unconfirmed",
            "done_for_day_review_required",
            "terminal_filled_review_required",
            "terminal_rejected_review_required",
            "terminal_canceled_review_required",
            "terminal_expired_review_required",
            "terminal_replaced_review_required",
            "suspended_review_required",
            "broker_unknown",
            "broker_surprise_status",
        ] {
            assert!(
                unresolved_alpaca_intent_status(status),
                "expected status {status} to remain unresolved"
            );
        }
        assert!(!unresolved_alpaca_intent_status("filled"));
    }

    #[test]
    fn submit_or_reuse_alpaca_order_avoids_duplicate_submit_when_lookup_finds_existing_order() {
        let mut lookup_count = 0usize;
        let mut submit_count = 0usize;

        let outcome = submit_or_reuse_alpaca_order_with_ops(
            "AAPL",
            "vg-existing",
            || {
                lookup_count += 1;
                Ok(Some(json!({
                    "id": "ord_existing",
                    "client_order_id": "vg-existing",
                    "symbol": "AAPL",
                    "side": "buy",
                    "status": "new",
                    "type": "market"
                })))
            },
            || {
                submit_count += 1;
                Ok(json!({ "id": "ord_duplicate", "client_order_id": "vg-existing" }))
            },
        )
        .expect("existing order should be reused");

        assert_eq!(lookup_count, 1);
        assert_eq!(submit_count, 0);
        assert!(outcome.reused_existing);
        assert_eq!(outcome.order_id, "ord_existing");
    }

    #[test]
    fn submit_or_reuse_alpaca_order_rechecks_after_submit_error_before_failing() {
        let mut lookup_count = 0usize;
        let mut submit_count = 0usize;

        let outcome = submit_or_reuse_alpaca_order_with_ops(
            "AAPL",
            "vg-after-error",
            || {
                lookup_count += 1;
                if lookup_count == 1 {
                    Ok(None)
                } else {
                    Ok(Some(json!({
                        "id": "ord_after_error",
                        "client_order_id": "vg-after-error",
                        "symbol": "AAPL",
                        "side": "buy",
                        "status": "accepted",
                        "type": "market"
                    })))
                }
            },
            || {
                submit_count += 1;
                Err("network dropped after submit".to_string())
            },
        )
        .expect("follow-up lookup should recover accepted order");

        assert_eq!(lookup_count, 2);
        assert_eq!(submit_count, 1);
        assert!(outcome.reused_existing);
        assert_eq!(outcome.order_id, "ord_after_error");
    }

    fn test_runner_runtime(generation: u64) -> LiveExecutionRunnerRuntime {
        let req = test_execute_request("AAPL", 1.0);
        LiveExecutionRunnerRuntime {
            generation,
            abort: RunnerAbortHandle::new(),
            pending_bar: None,
            cfg: LiveExecutionRunnerInternalCfg {
                req,
                symbol: "AAPL".to_string(),
                qty: 1.0,
                data_id: "data_1".to_string(),
                strategy_kind: "DoubleMa".to_string(),
                app_local_data_dir: std::env::temp_dir(),
            },
            inflight: false,
            started_at_unix_ms: 0,
            last_bar_open_unix_ms: None,
            last_triggered_at_unix_ms: None,
            last_completed_at_unix_ms: None,
            run_count: 0,
            skip_count: 0,
            error_count: 0,
            last_action: None,
            last_order_id: None,
            last_error: None,
        }
    }

    fn test_runner_bar(time_open_unix_ms: i64, close: f64) -> LiveExecutionRunnerBar {
        LiveExecutionRunnerBar {
            symbol: "AAPL".to_string(),
            time_open_unix_ms,
            open: close,
            high: close,
            low: close,
            close,
            volume: None,
        }
    }
    use serde_json::json;

    #[test]
    fn open_position_from_trades_uses_latest_open_trade() {
        let trades = vec![
            ta_strategies::double_ma::Trade {
                entry_t: 1,
                exit_t: 2,
                direction: 1,
                pnl: 0.01,
                bars: 1,
                open: false,
            },
            ta_strategies::double_ma::Trade {
                entry_t: 3,
                exit_t: 4,
                direction: -1,
                pnl: 0.02,
                bars: 1,
                open: true,
            },
        ];
        assert_eq!(open_position_from_trades(&trades), -1);
    }

    #[test]
    fn risk_override_sanitizes_values() {
        let mut cfg = ta_strategies::double_ma::StrategyConfig::default();
        let risk = LiveRiskOverride {
            stop_loss_pct: Some(-1.0),
            take_profit_pct: Some(1.5),
            ..LiveRiskOverride::default()
        };
        apply_risk_override(&mut cfg, &risk);
        assert_eq!(cfg.stop_loss_pct, None);
        assert_eq!(cfg.take_profit_pct, Some(0.95));
    }

    #[test]
    fn execution_risk_policy_merge_lets_request_overrides_tighten_persisted_policy() {
        let persisted = ExecutionRiskPolicySettings {
            max_order_notional: Some(1_000.0),
            max_position_notional: Some(2_000.0),
            allow_short: Some(true),
            require_bracket: Some(false),
        };
        let risk = LiveRiskOverride {
            max_order_notional: Some(500.0),
            max_position_notional: None,
            allow_short: Some(false),
            require_bracket: Some(true),
            ..LiveRiskOverride::default()
        };

        let merged = merge_execution_risk_policy(&persisted, &risk).unwrap();

        assert_eq!(merged.max_order_notional, Some(500.0));
        assert_eq!(merged.max_position_notional, Some(2_000.0));
        assert_eq!(merged.allow_short, Some(false));
        assert_eq!(merged.require_bracket, Some(true));
        assert_eq!(
            execution_policy_limits_from_settings(&merged).max_order_notional,
            Some(500.0)
        );
    }

    #[test]
    fn execution_risk_policy_merge_prevents_request_overrides_from_loosening_saved_policy() {
        let persisted = ExecutionRiskPolicySettings {
            max_order_notional: Some(1_000.0),
            max_position_notional: Some(2_000.0),
            allow_short: Some(false),
            require_bracket: Some(true),
        };
        let risk = LiveRiskOverride {
            max_order_notional: Some(5_000.0),
            max_position_notional: Some(10_000.0),
            allow_short: Some(true),
            require_bracket: Some(false),
            ..LiveRiskOverride::default()
        };

        let merged = merge_execution_risk_policy(&persisted, &risk).unwrap();

        assert_eq!(merged.max_order_notional, Some(1_000.0));
        assert_eq!(merged.max_position_notional, Some(2_000.0));
        assert_eq!(merged.allow_short, Some(false));
        assert_eq!(merged.require_bracket, Some(true));
    }

    #[test]
    fn execution_risk_policy_merge_rejects_invalid_request_caps() {
        let persisted = ExecutionRiskPolicySettings::default();
        let risk = LiveRiskOverride {
            max_order_notional: Some(0.0),
            ..LiveRiskOverride::default()
        };

        let err = merge_execution_risk_policy(&persisted, &risk).unwrap_err();

        assert!(err.contains("risk.max_order_notional"));
    }

    #[test]
    fn json_field_as_string_supports_scalar_types() {
        let v = json!({
            "str_v": "acct",
            "num_v": 123.45,
            "bool_v": true,
            "empty_v": ""
        });
        assert_eq!(json_field_as_string(&v, "str_v").as_deref(), Some("acct"));
        assert_eq!(json_field_as_string(&v, "num_v").as_deref(), Some("123.45"));
        assert_eq!(json_field_as_string(&v, "bool_v").as_deref(), Some("true"));
        assert_eq!(json_field_as_string(&v, "empty_v"), None);
        assert_eq!(json_field_as_string(&v, "missing"), None);
    }

    #[test]
    fn canonical_symbol_key_preserves_meaningful_punctuation() {
        assert_eq!(canonical_symbol_key(" brk.b "), "BRK.B");
        assert_eq!(canonical_symbol_key("brk b"), "BRKB");
        assert_eq!(canonical_symbol_key(" eur / usd "), "EUR/USD");
    }

    #[test]
    fn symbol_matching_does_not_collapse_punctuation_variants() {
        assert!(symbol_matches("BRK.B", "brk.b"));
        assert!(!symbol_matches("BRKB", "BRK.B"));
    }

    #[test]
    fn normalize_execute_request_rejects_missing_symbol_and_invalid_qty() {
        assert_eq!(
            normalize_execute_request(test_execute_request("   ", 1.0))
                .expect_err("blank symbol should fail"),
            "symbol is required"
        );
        assert_eq!(
            normalize_execute_request(test_execute_request("AAPL", 0.0))
                .expect_err("zero qty should fail"),
            "qty must be finite and > 0"
        );
        assert_eq!(
            normalize_execute_request(test_execute_request("AAPL", 0.00001))
                .expect_err("tiny qty should fail after normalization"),
            "qty is too small after normalization"
        );
    }

    #[test]
    fn normalize_execute_request_quantizes_qty_to_1e4_boundaries() {
        let (req, symbol, qty) = normalize_execute_request(test_execute_request(" aapl ", 0.00005))
            .expect("half-tick quantity should round up to 1e-4");
        assert_eq!(symbol, "AAPL");
        assert_eq!(req.symbol, "AAPL");
        assert_eq!(req.qty, 0.0001);
        assert_eq!(qty, 0.0001);

        let (_, _, qty) = normalize_execute_request(test_execute_request("AAPL", 0.000149))
            .expect("sub-midpoint quantity should round down");
        assert_eq!(qty, 0.0001);

        let (_, _, qty) = normalize_execute_request(test_execute_request("AAPL", 0.00015))
            .expect("midpoint quantity should round up");
        assert_eq!(qty, 0.0002);
    }

    #[test]
    fn delta_qty_hold_threshold_is_strictly_less_than_1e6() {
        assert!(delta_qty_is_hold(0.0000005));
        assert!(delta_qty_is_hold(-0.0000005));
        assert!(!delta_qty_is_hold(0.000001));
        assert!(!delta_qty_is_hold(-0.000001));
    }

    #[test]
    fn opening_bracket_threshold_uses_current_lt_and_target_gt_1e6() {
        assert!(should_attach_opening_bracket(0.0000009, 0.0000011));
        assert!(!should_attach_opening_bracket(0.000001, 0.0000011));
        assert!(!should_attach_opening_bracket(0.0000009, 0.000001));
        assert!(!should_attach_opening_bracket(0.0, 0.0));
    }

    #[test]
    fn local_paper_blocks_when_required_bracket_needs_child_orders() {
        let required = ExecutionRiskPolicySettings {
            require_bracket: Some(true),
            ..ExecutionRiskPolicySettings::default()
        };
        let optional = ExecutionRiskPolicySettings {
            require_bracket: Some(false),
            ..ExecutionRiskPolicySettings::default()
        };

        assert!(local_paper_bracket_child_orders_required_but_unsupported(
            &required, true
        ));
        assert!(!local_paper_bracket_child_orders_required_but_unsupported(
            &required, false
        ));
        assert!(!local_paper_bracket_child_orders_required_but_unsupported(
            &optional, true
        ));
    }

    fn test_live_stream_info(
        symbol: &str,
        delayed: bool,
        active: bool,
        status: &str,
        last_received_at_unix_ms: Option<i64>,
    ) -> crate::state::LiveCandleStreamInfo {
        crate::state::LiveCandleStreamInfo {
            market: "stocks".to_string(),
            symbol: symbol.to_string(),
            delayed,
            active,
            status: status.to_string(),
            last_message: Some(status.to_string()),
            last_status_at_unix_ms: Some(900_000),
            last_accepted_bar_open_unix_ms: last_received_at_unix_ms.map(|_| 800_000),
            last_accepted_bar_close_unix_ms: last_received_at_unix_ms.map(|_| 859_999),
            last_accepted_bar_received_at_unix_ms: last_received_at_unix_ms,
        }
    }

    #[test]
    fn live_runner_stream_preconditions_require_active_stream() {
        assert_eq!(
            validate_live_runner_stream_preconditions(
                None,
                "AAPL",
                ExecutionBrokerTarget::DryRun,
                1_000_000
            ),
            Err("Start live candles first. Auto-runner triggers on Polygon live bars.".to_string())
        );
        assert_eq!(
            validate_live_runner_stream_preconditions(
                Some(&test_live_stream_info(
                    "AAPL",
                    true,
                    false,
                    "closed",
                    Some(950_000)
                )),
                "AAPL",
                ExecutionBrokerTarget::DryRun,
                1_000_000,
            ),
            Err("Start live candles first. Auto-runner triggers on Polygon live bars.".to_string())
        );
    }

    #[test]
    fn live_runner_stream_preconditions_require_matching_symbol() {
        assert_eq!(
            validate_live_runner_stream_preconditions(
                Some(&test_live_stream_info(
                    "TSLA",
                    true,
                    true,
                    "started",
                    Some(950_000)
                )),
                "AAPL",
                ExecutionBrokerTarget::DryRun,
                1_000_000,
            ),
            Err("Live candles are active for TSLA, not AAPL.".to_string())
        );
    }

    #[test]
    fn live_runner_stream_preconditions_accept_canonical_symbol_match() {
        assert!(validate_live_runner_stream_preconditions(
            Some(&test_live_stream_info(
                " brk.b ",
                true,
                true,
                "started",
                Some(950_000)
            )),
            "BRK.B",
            ExecutionBrokerTarget::DryRun,
            1_000_000,
        )
        .is_ok());
    }

    #[test]
    fn live_runner_stream_preconditions_reject_live_broker_on_delayed_stream() {
        assert_eq!(
            validate_live_runner_stream_preconditions(
                Some(&test_live_stream_info(
                    "AAPL",
                    true,
                    true,
                    "started",
                    Some(950_000)
                )),
                "AAPL",
                ExecutionBrokerTarget::AlpacaLive,
                1_000_000,
            ),
            Err(
                "Live Alpaca automation requires a real-time Polygon stream. Turn delayed data off before arming the broker."
                    .to_string()
            )
        );
        assert!(
            validate_live_runner_stream_preconditions(
                Some(&test_live_stream_info(
                    "AAPL",
                    true,
                    true,
                    "started",
                    Some(950_000)
                )),
                "AAPL",
                ExecutionBrokerTarget::DryRun,
                1_000_000,
            )
            .is_ok(),
            "dry-run runner should still be allowed on delayed data"
        );
        assert!(
            validate_live_runner_stream_preconditions(
                Some(&test_live_stream_info(
                    "AAPL",
                    false,
                    true,
                    "started",
                    Some(950_000)
                )),
                "AAPL",
                ExecutionBrokerTarget::AlpacaLive,
                1_000_000,
            )
            .is_ok(),
            "real-time stream should allow live Alpaca automation"
        );
    }

    #[test]
    fn live_runner_stream_preconditions_require_recent_accepted_bar() {
        assert_eq!(
            validate_live_runner_stream_preconditions(
                Some(&test_live_stream_info("AAPL", false, true, "started", None)),
                "AAPL",
                ExecutionBrokerTarget::DryRun,
                1_000_000,
            ),
            Err(
                "Live candles are active but no accepted bar has been received yet. Wait for the next Polygon minute bar before arming the auto-runner."
                    .to_string()
            )
        );

        let stale = validate_live_runner_stream_preconditions(
            Some(&test_live_stream_info(
                "AAPL",
                false,
                true,
                "started",
                Some(700_000),
            )),
            "AAPL",
            ExecutionBrokerTarget::DryRun,
            1_000_000,
        )
        .expect_err("stale feed should block runner arming");
        assert!(stale.contains("Live candle feed is stale"));

        assert!(
            validate_live_runner_stream_preconditions(
                Some(&test_live_stream_info(
                    "AAPL",
                    true,
                    true,
                    "started",
                    Some(950_000)
                )),
                "AAPL",
                ExecutionBrokerTarget::DryRun,
                1_000_000,
            )
            .is_ok(),
            "received-time freshness should allow delayed dry-run streams with old bar opens"
        );
    }

    #[test]
    fn base_url_maps_paper_and_live_modes() {
        assert_eq!(
            base_url(LiveExecutionMode::Paper),
            "https://paper-api.alpaca.markets"
        );
        assert_eq!(
            base_url(LiveExecutionMode::Live),
            "https://api.alpaca.markets"
        );
    }

    #[test]
    fn normalize_order_snapshot_reduces_broker_payload_to_review_fields() {
        let snapshot = normalize_order_snapshot(&json!({
            "id": "ord_123",
            "client_order_id": "vg-abc123",
            "symbol": "aapl",
            "side": "buy",
            "status": "filled",
            "type": "market",
            "qty": "2.5",
            "filled_qty": "2.5",
            "filled_avg_price": "182.40",
            "submitted_at": "2026-04-10T15:31:00Z",
            "updated_at": "2026-04-10T15:32:00Z",
            "ignored_nested": { "foo": "bar" }
        }))
        .expect("snapshot should normalize");

        assert_eq!(snapshot.id, "ord_123");
        assert_eq!(snapshot.client_order_id.as_deref(), Some("vg-abc123"));
        assert_eq!(snapshot.symbol, "aapl");
        assert_eq!(snapshot.side, "buy");
        assert_eq!(snapshot.status, "filled");
        assert_eq!(snapshot.order_type, "market");
        assert_eq!(snapshot.qty, Some(2.5));
        assert_eq!(snapshot.filled_qty, Some(2.5));
        assert_eq!(snapshot.filled_avg_price, Some(182.40));
        assert_eq!(
            snapshot.submitted_at.as_deref(),
            Some("2026-04-10T15:31:00Z")
        );
        assert_eq!(snapshot.updated_at.as_deref(), Some("2026-04-10T15:32:00Z"));
    }

    #[test]
    fn normalize_position_snapshot_requires_symbol_and_preserves_position_fields() {
        assert!(normalize_position_snapshot(&json!({ "qty": "1.0" })).is_none());

        let snapshot = normalize_position_snapshot(&json!({
            "symbol": "AAPL",
            "qty": "3",
            "side": "long",
            "avg_entry_price": "180.10",
            "current_price": "181.25",
            "market_value": "543.75",
            "unrealized_pl": "3.45",
            "unrealized_plpc": "0.0064"
        }))
        .expect("position should normalize");

        assert_eq!(snapshot.symbol, "AAPL");
        assert_eq!(snapshot.qty, 3.0);
        assert_eq!(snapshot.side.as_deref(), Some("long"));
        assert_eq!(snapshot.avg_entry_price, Some(180.10));
        assert_eq!(snapshot.current_price, Some(181.25));
        assert_eq!(snapshot.market_value, Some(543.75));
        assert_eq!(snapshot.unrealized_pl, Some(3.45));
        assert_eq!(snapshot.unrealized_plpc, Some(0.0064));
    }

    #[test]
    fn alpaca_order_payload_attaches_bracket_only_when_both_risk_bounds_exist() {
        let mut cfg = StrategyConfig::default();
        cfg.stop_loss_pct = Some(0.05);
        cfg.take_profit_pct = Some(0.10);

        let (payload, warnings) =
            alpaca_order_payload("AAPL", 2.0, &cfg, 100.0).expect("bracket payload");
        assert!(warnings.is_empty());
        assert_eq!(
            payload.get("order_class").and_then(|value| value.as_str()),
            Some("bracket")
        );
        assert_eq!(
            payload
                .get("take_profit")
                .and_then(|value| value.get("limit_price"))
                .and_then(|value| value.as_str()),
            Some("110.00")
        );
        assert_eq!(
            payload
                .get("stop_loss")
                .and_then(|value| value.get("stop_price"))
                .and_then(|value| value.as_str()),
            Some("95.00")
        );

        cfg.take_profit_pct = None;
        let (payload, warnings) =
            alpaca_order_payload("AAPL", 2.0, &cfg, 100.0).expect("unbracketed payload");
        assert!(payload.get("order_class").is_none());
        assert_eq!(
            warnings,
            vec![
                "Live bracket was skipped: Alpaca bracket requires both stop_loss_pct and take_profit_pct."
                    .to_string()
            ]
        );
    }

    #[test]
    fn alpaca_price_formatting_uses_broker_precision_rules() {
        assert_eq!(
            format_alpaca_price_by_increment(123.456).expect("format above one"),
            "123.46"
        );
        assert_eq!(
            format_alpaca_price_by_increment(0.123456).expect("format below one"),
            "0.1235"
        );
        assert_eq!(
            format_alpaca_price_by_increment(0.99996).expect("format rounded to one"),
            "1.00"
        );
        assert!(format_alpaca_price_by_increment(0.0).is_err());
        assert!(format_alpaca_price_by_increment(f64::NAN).is_err());
    }

    #[test]
    fn alpaca_order_payload_rejects_bracket_prices_that_round_to_inversion() {
        let mut cfg = StrategyConfig::default();
        cfg.stop_loss_pct = Some(0.00001);
        cfg.take_profit_pct = Some(0.00001);

        let err = alpaca_order_payload("AAPL", 2.0, &cfg, 100.00)
            .expect_err("rounded buy bracket should invert to equal prices");
        assert!(err.contains("buy bracket would invert"));

        let err = alpaca_order_payload("AAPL", -2.0, &cfg, 100.00)
            .expect_err("rounded sell bracket should invert to equal prices");
        assert!(err.contains("sell bracket would invert"));
    }

    #[test]
    fn alpaca_order_payload_uses_four_decimals_below_one_dollar() {
        let mut cfg = StrategyConfig::default();
        cfg.stop_loss_pct = Some(0.05);
        cfg.take_profit_pct = Some(0.10);

        let (payload, warnings) =
            alpaca_order_payload("PENNY", 2.0, &cfg, 0.50).expect("below-dollar bracket payload");

        assert!(warnings.is_empty());
        assert_eq!(
            payload
                .get("take_profit")
                .and_then(|value| value.get("limit_price"))
                .and_then(|value| value.as_str()),
            Some("0.5500")
        );
        assert_eq!(
            payload
                .get("stop_loss")
                .and_then(|value| value.get("stop_price"))
                .and_then(|value| value.as_str()),
            Some("0.4750")
        );
    }

    #[test]
    fn runner_status_from_runtime_reports_config_and_counters() {
        let mut runtime = test_runner_runtime(77);
        runtime.inflight = true;
        runtime.started_at_unix_ms = 100;
        runtime.last_bar_open_unix_ms = Some(200);
        runtime.last_triggered_at_unix_ms = Some(300);
        runtime.last_completed_at_unix_ms = Some(400);
        runtime.run_count = 5;
        runtime.skip_count = 2;
        runtime.error_count = 1;
        runtime.last_action = Some("buy".to_string());
        runtime.last_order_id = Some("ord_123".to_string());
        runtime.last_error = Some("last error".to_string());

        let status = runner_status_from_runtime(Some(&runtime));
        assert!(status.active);
        assert!(status.inflight);
        assert_eq!(status.run_count, 5);
        assert_eq!(status.skip_count, 2);
        assert_eq!(status.error_count, 1);
        assert_eq!(status.started_at_unix_ms, Some(100));
        assert_eq!(status.last_bar_open_unix_ms, Some(200));
        assert_eq!(status.last_triggered_at_unix_ms, Some(300));
        assert_eq!(status.last_completed_at_unix_ms, Some(400));
        assert_eq!(status.last_action.as_deref(), Some("buy"));
        assert_eq!(status.last_order_id.as_deref(), Some("ord_123"));
        assert_eq!(status.last_error.as_deref(), Some("last error"));
        let cfg = status.config.expect("runner config");
        assert_eq!(cfg.symbol, "AAPL");
        assert_eq!(cfg.qty, 1.0);
        assert_eq!(cfg.mode, LiveExecutionMode::Paper);
        assert_eq!(cfg.data_id, "data_1");
        assert_eq!(cfg.strategy_kind, "DoubleMa");
        assert!(cfg.dry_run);
    }

    #[test]
    fn runner_status_from_runtime_none_is_inactive_and_zeroed() {
        let status = runner_status_from_runtime(None);

        assert!(!status.active);
        assert!(!status.inflight);
        assert!(status.config.is_none());
        assert_eq!(status.run_count, 0);
        assert_eq!(status.skip_count, 0);
        assert_eq!(status.error_count, 0);
        assert!(status.last_action.is_none());
        assert!(status.last_order_id.is_none());
        assert!(status.last_error.is_none());
        assert!(status.started_at_unix_ms.is_none());
        assert!(status.last_bar_open_unix_ms.is_none());
        assert!(status.last_triggered_at_unix_ms.is_none());
        assert!(status.last_completed_at_unix_ms.is_none());
    }

    #[test]
    fn pending_bar_coalesces_three_inflight_bars_to_latest_only() {
        let mut runtime = test_runner_runtime(17);
        runtime.inflight = true;

        assert!(update_pending_bar(
            &mut runtime.pending_bar,
            &test_runner_bar(1000, 1.0)
        ));
        assert!(update_pending_bar(
            &mut runtime.pending_bar,
            &test_runner_bar(2000, 2.0)
        ));
        assert!(update_pending_bar(
            &mut runtime.pending_bar,
            &test_runner_bar(3000, 3.0)
        ));

        let queued = runtime.pending_bar.take().expect("latest queued bar");
        assert_eq!(queued.time_open_unix_ms, 3000);
        assert_eq!(queued.close, 3.0);
        assert!(runtime.pending_bar.is_none());
    }

    #[test]
    fn pending_bar_rejects_stale_or_equal_timestamps() {
        let mut pending = Some(test_runner_bar(1000, 1.0));

        assert!(!update_pending_bar(
            &mut pending,
            &test_runner_bar(1000, 1.1)
        ));
        assert!(!update_pending_bar(
            &mut pending,
            &test_runner_bar(999, 0.9)
        ));

        let queued = pending.expect("pending bar should remain unchanged");
        assert_eq!(queued.time_open_unix_ms, 1000);
        assert_eq!(queued.close, 1.0);
    }

    #[test]
    fn generation_gate_ignores_stale_runtime_updates() {
        let _guard = runner_test_guard().lock().expect("lock runner test guard");
        let slot = live_execution_runner_slot();
        let mut guard = slot.lock().expect("lock runner slot");
        let prior = guard.take();
        *guard = Some(test_runner_runtime(42));
        drop(guard);

        let stale = with_active_runner_generation(7, |runtime| {
            runtime.skip_count = 99;
        });
        let current = with_active_runner_generation(42, |runtime| {
            runtime.skip_count = 5;
            runtime.skip_count
        });

        let mut guard = slot.lock().expect("relock runner slot");
        let active_skip_count = guard.as_ref().map(|runtime| runtime.skip_count);
        *guard = prior;
        drop(guard);

        assert!(stale.is_none());
        assert_eq!(current, Some(5));
        assert_eq!(active_skip_count, Some(5));
    }

    #[test]
    fn runner_abort_handle_sets_cancellation_marker() {
        let abort = RunnerAbortHandle::new();
        assert!(abort.check().is_ok());
        abort.cancel();
        assert!(is_runner_cancelled_error(
            abort
                .check()
                .expect_err("expected cancellation marker")
                .as_str()
        ));
    }

    #[test]
    fn with_active_runner_generation_ignores_stale_runtime_generations() {
        let _guard = runner_test_guard().lock().expect("lock runner test guard");
        let slot = live_execution_runner_slot();
        let mut guard = slot.lock().expect("lock live execution runner slot");
        let prior = guard.take();
        *guard = Some(test_runner_runtime(42));
        drop(guard);

        assert!(with_active_runner_generation(41, |_| ()).is_none());

        let seen_generation = with_active_runner_generation(42, |runtime| {
            runtime.inflight = true;
            runtime.generation
        });
        assert_eq!(seen_generation, Some(42));

        let guard = slot.lock().expect("re-lock live execution runner slot");
        assert!(guard.as_ref().expect("runtime present").inflight);
        drop(guard);

        let mut guard = slot.lock().expect("restore live execution runner slot");
        *guard = prior;
    }
}
