use crate::state::AppState;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tauri::Manager;
use vector_ta::utilities::data_loader::Candles;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveExecutionMode {
    Paper,
    Live,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LiveRiskOverride {
    #[serde(default)]
    pub stop_loss_pct: Option<f64>,
    #[serde(default)]
    pub take_profit_pct: Option<f64>,
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
    pub risk: LiveRiskOverride,
    #[serde(default)]
    pub alpaca_api_key: String,
    #[serde(default)]
    pub alpaca_api_secret: String,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveOrderSnapshot {
    pub id: String,
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
    pub account_status: Option<String>,
    #[serde(default)]
    pub fetched_at_unix_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<LivePositionSnapshot>,
    #[serde(default)]
    pub orders: Vec<LiveOrderSnapshot>,
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
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_bar_ts: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_close: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

const LIVE_EXECUTION_RUNNER_EVENT: &str = "live_execution_runner_event";

#[derive(Clone, Debug, Serialize)]
pub struct LiveExecutionRunnerConfig {
    pub symbol: String,
    pub qty: f64,
    pub mode: LiveExecutionMode,
    pub dry_run: bool,
    pub data_id: String,
    pub strategy_kind: String,
}

#[derive(Clone, Debug, Serialize)]
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
struct LiveExecutionRunnerInternalCfg {
    req: ExecuteStrategySignalRequest,
    symbol: String,
    qty: f64,
    data_id: String,
    strategy_kind: String,
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

fn finite_last(xs: &[f64]) -> Option<f64> {
    xs.iter().rev().copied().find(|v| v.is_finite() && *v > 0.0)
}

fn signal_decision_from_request(
    req: &LiveStrategySpec,
    candles: &vector_ta::utilities::data_loader::Candles,
    risk: &LiveRiskOverride,
) -> Result<SignalDecision, String> {
    match req {
        LiveStrategySpec::DoubleMa { req } => {
            let mut dd = req.clone();
            apply_risk_override(&mut dd.strategy, risk);
            let curves = ta_app_core::compute_double_ma_drilldown_blocking_with_candles(
                dd.clone(),
                candles,
            )?;
            let desired_position = open_position_from_trades(&curves.trades);
            let last_close =
                finite_last(&candles.close).ok_or_else(|| "missing finite close".to_string())?;
            Ok(SignalDecision {
                desired_position,
                cfg: dd.strategy,
                last_bar_ts: candles.timestamp.last().copied(),
                last_close,
            })
        }
        LiveStrategySpec::CompositeCpu { req } => {
            let mut dd = req.clone();
            apply_risk_override(&mut dd.strategy, risk);
            let curves = ta_app_core::compute_composite_cpu_drilldown_blocking_with_candles(
                dd.clone(),
                candles,
            )?;
            let desired_position = open_position_from_trades(&curves.curves.trades);
            let last_close =
                finite_last(&candles.close).ok_or_else(|| "missing finite close".to_string())?;
            Ok(SignalDecision {
                desired_position,
                cfg: dd.strategy,
                last_bar_ts: candles.timestamp.last().copied(),
                last_close,
            })
        }
    }
}

fn strategy_data_id(spec: &LiveStrategySpec) -> &str {
    match spec {
        LiveStrategySpec::DoubleMa { req } => &req.data_id,
        LiveStrategySpec::CompositeCpu { req } => &req.data_id,
    }
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

fn strategy_kind_label(spec: &LiveStrategySpec) -> String {
    match spec {
        LiveStrategySpec::DoubleMa { .. } => "DoubleMa",
        LiveStrategySpec::CompositeCpu { .. } => "CompositeCpu",
    }
    .to_string()
}

fn runner_config_view(cfg: &LiveExecutionRunnerInternalCfg) -> LiveExecutionRunnerConfig {
    LiveExecutionRunnerConfig {
        symbol: cfg.symbol.clone(),
        qty: cfg.qty,
        mode: cfg.req.mode,
        dry_run: cfg.req.dry_run,
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
    let _ = app.emit(LIVE_EXECUTION_RUNNER_EVENT, payload);
}

fn normalize_execute_request(
    mut req: ExecuteStrategySignalRequest,
) -> Result<(ExecuteStrategySignalRequest, String, f64), String> {
    let symbol = req.symbol.trim().to_uppercase();
    if symbol.is_empty() {
        return Err("symbol is required".to_string());
    }
    if !req.qty.is_finite() || req.qty <= 0.0 {
        return Err("qty must be finite and > 0".to_string());
    }
    let qty = (req.qty * 10_000.0).round() / 10_000.0;
    if qty <= 0.0 {
        return Err("qty is too small after normalization".to_string());
    }
    req.symbol = symbol.clone();
    req.qty = qty;
    Ok((req, symbol, qty))
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

fn normalize_symbol_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase()
}

fn symbol_matches(order_symbol: &str, target_symbol: &str) -> bool {
    if target_symbol.is_empty() {
        return true;
    }
    normalize_symbol_key(order_symbol) == normalize_symbol_key(target_symbol)
}

fn fetch_alpaca_orders(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    limit: u32,
) -> Result<Vec<serde_json::Value>, String> {
    let headers = build_alpaca_headers(api_key, api_secret)?;
    let url = format!(
        "{}/v2/orders?status=all&direction=desc&limit={}",
        base_url(mode),
        limit.clamp(1, 500)
    );
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

fn normalize_order_snapshot(v: &serde_json::Value) -> Option<LiveOrderSnapshot> {
    let id = json_field_as_string(v, "id")?;
    let symbol = json_field_as_string(v, "symbol").unwrap_or_default();
    let side = json_field_as_string(v, "side").unwrap_or_else(|| "unknown".to_string());
    let status = json_field_as_string(v, "status").unwrap_or_else(|| "unknown".to_string());
    let order_type = json_field_as_string(v, "type").unwrap_or_else(|| "unknown".to_string());
    Some(LiveOrderSnapshot {
        id,
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

fn alpaca_order_payload(
    symbol: &str,
    delta_qty: f64,
    cfg: &ta_strategies::double_ma::StrategyConfig,
    last_close: f64,
) -> (serde_json::Value, Vec<String>) {
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
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("order_class".to_string(), serde_json::json!("bracket"));
            obj.insert(
                "take_profit".to_string(),
                serde_json::json!({ "limit_price": format!("{take_px:.4}") }),
            );
            obj.insert(
                "stop_loss".to_string(),
                serde_json::json!({ "stop_price": format!("{stop_px:.4}") }),
            );
        }
    } else if stop.is_some() || take.is_some() {
        warnings.push(
            "Live bracket was skipped: Alpaca bracket requires both stop_loss_pct and take_profit_pct."
                .to_string(),
        );
    }

    (payload, warnings)
}

fn submit_alpaca_order(
    client: &Client,
    mode: LiveExecutionMode,
    api_key: &str,
    api_secret: &str,
    payload: &serde_json::Value,
) -> Result<String, String> {
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
    let v: serde_json::Value = resp
        .json()
        .map_err(|e| format!("failed to decode Alpaca order response: {e}"))?;
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| "Alpaca order response missing id".to_string())?;
    Ok(id.to_string())
}

fn execute_strategy_signal_blocking(
    req: ExecuteStrategySignalRequest,
    symbol: String,
    qty: f64,
    candles: Candles,
) -> Result<ExecuteStrategySignalResponse, String> {
    let decision = signal_decision_from_request(&req.strategy, &candles, &req.risk)?;
    let target_position_qty = (decision.desired_position as f64) * qty;
    let mut warnings: Vec<String> = Vec::new();

    let mut current_position_qty = 0.0_f64;
    if !req.dry_run {
        if req.alpaca_api_key.trim().is_empty() || req.alpaca_api_secret.trim().is_empty() {
            return Err(
                "alpaca_api_key and alpaca_api_secret are required unless dry_run=true".to_string(),
            );
        }
        let client = Client::new();
        current_position_qty = fetch_alpaca_position_qty(
            &client,
            req.mode,
            req.alpaca_api_key.trim(),
            req.alpaca_api_secret.trim(),
            &symbol,
        )?;
    }

    let delta_qty = target_position_qty - current_position_qty;
    if delta_qty.abs() < 1e-6 {
        return Ok(ExecuteStrategySignalResponse {
            symbol,
            mode: req.mode,
            desired_position: decision.desired_position,
            current_position_qty,
            target_position_qty,
            delta_qty: 0.0,
            order_submitted: false,
            order_id: None,
            action: "hold".to_string(),
            last_bar_ts: decision.last_bar_ts,
            last_close: Some(decision.last_close),
            warnings,
        });
    }

    if req.dry_run {
        return Ok(ExecuteStrategySignalResponse {
            symbol,
            mode: req.mode,
            desired_position: decision.desired_position,
            current_position_qty,
            target_position_qty,
            delta_qty,
            order_submitted: false,
            order_id: None,
            action: "dry_run_signal_only".to_string(),
            last_bar_ts: decision.last_bar_ts,
            last_close: Some(decision.last_close),
            warnings,
        });
    }

    let client = Client::new();
    let opening_new = current_position_qty.abs() < 1e-6 && target_position_qty.abs() > 1e-6;
    let (payload, bracket_warnings) = if opening_new {
        alpaca_order_payload(&symbol, delta_qty, &decision.cfg, decision.last_close)
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

    let order_id = submit_alpaca_order(
        &client,
        req.mode,
        req.alpaca_api_key.trim(),
        req.alpaca_api_secret.trim(),
        &payload,
    )?;

    Ok(ExecuteStrategySignalResponse {
        symbol,
        mode: req.mode,
        desired_position: decision.desired_position,
        current_position_qty,
        target_position_qty,
        delta_qty,
        order_submitted: true,
        order_id: Some(order_id),
        action: if delta_qty > 0.0 {
            "buy".to_string()
        } else {
            "sell".to_string()
        },
        last_bar_ts: decision.last_bar_ts,
        last_close: Some(decision.last_close),
        warnings,
    })
}

#[tauri::command]
pub async fn execute_strategy_signal_once(
    req: ExecuteStrategySignalRequest,
    state: tauri::State<'_, AppState>,
) -> Result<ExecuteStrategySignalResponse, String> {
    let (req, symbol, qty) = normalize_execute_request(req)?;
    let data_id = strategy_data_id(&req.strategy).to_string();
    let candles = state.get_candles(&data_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        execute_strategy_signal_blocking(req, symbol, qty, candles)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn start_live_execution_runner(
    req: ExecuteStrategySignalRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<LiveExecutionRunnerStatus, String> {
    let (req, symbol, qty) = normalize_execute_request(req)?;
    let data_id = strategy_data_id(&req.strategy).trim().to_string();
    if data_id.is_empty() {
        return Err("strategy data_id is required".to_string());
    }
    let _ = state.get_candles(&data_id)?;

    let strategy_kind = strategy_kind_label(&req.strategy);
    let cfg = LiveExecutionRunnerInternalCfg {
        req: req.clone(),
        symbol: symbol.clone(),
        qty,
        data_id: data_id.clone(),
        strategy_kind: strategy_kind.clone(),
    };
    let runtime = LiveExecutionRunnerRuntime {
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
        *slot = Some(runtime);
    }

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

#[tauri::command]
pub fn stop_live_execution_runner(
    app: tauri::AppHandle,
) -> Result<StopLiveExecutionRunnerResponse, String> {
    let prior = {
        let mut slot = live_execution_runner_slot()
            .lock()
            .map_err(|_| "failed to lock live execution runner state".to_string())?;
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
    Ok(StopLiveExecutionRunnerResponse {
        stopped,
        prior,
        status,
    })
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
    let mut skipped_busy = false;
    let symbol_norm = normalize_symbol_key(&bar.symbol);

    {
        let Ok(mut slot) = live_execution_runner_slot().lock() else {
            return;
        };
        let Some(runtime) = slot.as_mut() else {
            return;
        };
        if normalize_symbol_key(&runtime.cfg.symbol) != symbol_norm {
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
            skipped_busy = true;
        } else {
            runtime.inflight = true;
            runtime.last_bar_open_unix_ms = Some(bar.time_open_unix_ms);
            runtime.last_triggered_at_unix_ms = Some(now_unix_ms());
            runtime.last_error = None;
            cfg_to_run = Some(runtime.cfg.clone());
        }
    }

    if skipped_busy {
        emit_live_execution_runner_event(
            app,
            "skipped_busy",
            "runner skipped bar because previous run is still in flight",
            bar.symbol.clone(),
            Some(bar.time_open_unix_ms),
            None,
        );
        return;
    }

    let Some(cfg) = cfg_to_run else {
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
            if let Ok(mut slot) = live_execution_runner_slot().lock() {
                if let Some(runtime) = slot.as_mut() {
                    if normalize_symbol_key(&runtime.cfg.symbol)
                        == normalize_symbol_key(&cfg.symbol)
                    {
                        runtime.inflight = false;
                        runtime.error_count = runtime.error_count.saturating_add(1);
                        runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                        runtime.last_error = Some(e.clone());
                    }
                }
            }
            emit_live_execution_runner_event(
                &app2,
                "error",
                format!("runner failed to apply live bar: {e}"),
                cfg.symbol.clone(),
                Some(bar.time_open_unix_ms),
                None,
            );
            return;
        }

        let candles = match state.get_candles(&cfg.data_id) {
            Ok(c) => c,
            Err(e) => {
                if let Ok(mut slot) = live_execution_runner_slot().lock() {
                    if let Some(runtime) = slot.as_mut() {
                        if normalize_symbol_key(&runtime.cfg.symbol)
                            == normalize_symbol_key(&cfg.symbol)
                        {
                            runtime.inflight = false;
                            runtime.error_count = runtime.error_count.saturating_add(1);
                            runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                            runtime.last_error = Some(e.clone());
                        }
                    }
                }
                emit_live_execution_runner_event(
                    &app2,
                    "error",
                    format!("runner failed to load strategy candles: {e}"),
                    cfg.symbol.clone(),
                    Some(bar.time_open_unix_ms),
                    None,
                );
                return;
            }
        };

        let req = cfg.req.clone();
        let symbol = cfg.symbol.clone();
        let qty = cfg.qty;
        let exec_result = tauri::async_runtime::spawn_blocking(move || {
            execute_strategy_signal_blocking(req, symbol, qty, candles)
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);

        match exec_result {
            Ok(resp) => {
                if let Ok(mut slot) = live_execution_runner_slot().lock() {
                    if let Some(runtime) = slot.as_mut() {
                        if normalize_symbol_key(&runtime.cfg.symbol)
                            == normalize_symbol_key(&cfg.symbol)
                        {
                            runtime.inflight = false;
                            runtime.run_count = runtime.run_count.saturating_add(1);
                            runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                            runtime.last_action = Some(resp.action.clone());
                            runtime.last_order_id = resp.order_id.clone();
                            runtime.last_error = None;
                        }
                    }
                }
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
            }
            Err(e) => {
                if let Ok(mut slot) = live_execution_runner_slot().lock() {
                    if let Some(runtime) = slot.as_mut() {
                        if normalize_symbol_key(&runtime.cfg.symbol)
                            == normalize_symbol_key(&cfg.symbol)
                        {
                            runtime.inflight = false;
                            runtime.error_count = runtime.error_count.saturating_add(1);
                            runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                            runtime.last_error = Some(e.clone());
                        }
                    }
                }
                emit_live_execution_runner_event(
                    &app2,
                    "error",
                    format!("runner execution failed: {e}"),
                    cfg.symbol.clone(),
                    Some(bar.time_open_unix_ms),
                    None,
                );
            }
        }
    });
}

#[tauri::command]
pub async fn test_alpaca_connection(
    req: TestAlpacaConnectionRequest,
) -> Result<TestAlpacaConnectionResponse, String> {
    let api_key = req.alpaca_api_key.trim().to_string();
    let api_secret = req.alpaca_api_secret.trim().to_string();
    if api_key.is_empty() || api_secret.is_empty() {
        return Err("alpaca_api_key and alpaca_api_secret are required".to_string());
    }
    let mode = req.mode;
    tauri::async_runtime::spawn_blocking(move || {
        let client = Client::new();
        let account = fetch_alpaca_account(&client, mode, &api_key, &api_secret)?;
        Ok(TestAlpacaConnectionResponse {
            mode,
            account_id: json_field_as_string(&account, "id"),
            account_number: json_field_as_string(&account, "account_number"),
            status: json_field_as_string(&account, "status"),
            currency: json_field_as_string(&account, "currency"),
            buying_power: json_field_as_string(&account, "buying_power"),
            cash: json_field_as_string(&account, "cash"),
            portfolio_value: json_field_as_string(&account, "portfolio_value"),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_live_execution_snapshot(
    req: GetLiveExecutionSnapshotRequest,
) -> Result<GetLiveExecutionSnapshotResponse, String> {
    let api_key = req.alpaca_api_key.trim().to_string();
    let api_secret = req.alpaca_api_secret.trim().to_string();
    if api_key.is_empty() || api_secret.is_empty() {
        return Err("alpaca_api_key and alpaca_api_secret are required".to_string());
    }

    let mode = req.mode;
    let symbol = req.symbol.trim().to_uppercase();
    let limit = req.limit.unwrap_or(200).clamp(1, 500);

    tauri::async_runtime::spawn_blocking(move || {
        let client = Client::new();
        let account = fetch_alpaca_account(&client, mode, &api_key, &api_secret)?;
        let account_status = json_field_as_string(&account, "status");

        let orders_raw = fetch_alpaca_orders(&client, mode, &api_key, &api_secret, limit)?;
        let mut orders: Vec<LiveOrderSnapshot> = Vec::new();
        for o in &orders_raw {
            let Some(ord) = normalize_order_snapshot(o) else {
                continue;
            };
            if !symbol_matches(&ord.symbol, &symbol) {
                continue;
            }
            orders.push(ord);
        }

        let positions_raw = fetch_alpaca_positions(&client, mode, &api_key, &api_secret)?;
        let mut position: Option<LivePositionSnapshot> = None;
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

        let fetched_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        Ok(GetLiveExecutionSnapshotResponse {
            mode,
            symbol,
            account_status,
            fetched_at_unix_ms,
            position,
            orders,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
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
        };
        apply_risk_override(&mut cfg, &risk);
        assert_eq!(cfg.stop_loss_pct, None);
        assert_eq!(cfg.take_profit_pct, Some(0.95));
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
}
