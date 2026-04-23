# Bundle 15 implementation plan: live execution, Polygon streaming, and runtime-control responsiveness

This document turns the section 15 review surface into an implementation-ready plan with concrete code edits, code snippets, rollout order, risk notes, and a bottom checklist.

## Scope

This plan is intentionally limited to the section 15 surface from `gpt54_feature_review_map.md`:

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/executionService.ts`
- `web/src/services/runControlService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/tauri/events.ts`
- `web/src/services/executionService.test.ts`
- `web/src/services/runControlService.test.ts`

## Goals

1. Make live runner stop/restart behavior authoritative enough for real user control.
2. Reduce user-visible lag from avoidable broker snapshot and status round-trips.
3. Remove frontend state drift between commands, events, and actual runtime state.
4. Allow stop/status actions to remain responsive while another execution action is in flight.
5. Keep the changes parity-safe and incremental.

## Non-goals

- No new trading features.
- No new strategy logic.
- No new UI panels.
- No broad architecture rewrite.
- No speculative broker feature expansion.
- No GPU or optimizer redesign.

---

## 1. Surface understanding

This surface currently has three main moving parts:

1. **Backend execution control in `commands_execution.rs`**
   - one-shot signal execution
   - Alpaca connection test
   - broker snapshot refresh
   - auto-runner lifecycle
   - live-bar trigger path

2. **Polygon live stream lifecycle in `commands_data.rs`**
   - start/stop/status for live candle stream
   - websocket message normalization
   - Tauri event emission for live status and bars
   - handoff from live bars into `on_live_execution_bar(...)`

3. **Frontend orchestration in `executionService.ts` and `executionStore.ts`**
   - invoke-based command control
   - local state, busy flags, order table shaping
   - runtime hints
   - currently mostly command/result-driven rather than event-driven

The main user-visible latency and correctness problems in the provided files are:

- stop/restart is not authoritative for in-flight runner work
- the store’s global busy gate blocks stop/status exactly when the user needs them
- live/runtime Tauri events exist but are not wired into the provided execution store/service surface
- snapshot refresh does unnecessary work for the common “orders + position” view
- `runOnce()` triggers an immediate broker refresh even after a successful submit
- `startRunner()` only checks “a live stream exists”, not “the live stream symbol matches the runner symbol”
- `clearKeys()` fakes live/runner state reset even though it does not actually stop them

That means the fastest, safest improvement path is:

1. fix backend stop/restart correctness
2. reduce avoidable broker latency
3. wire runtime events into frontend state
4. replace the global busy lock with per-action pending flags
5. tighten tests around the changed behavior

---

## 2. Findings

## Finding 1

- **Severity:** critical
- **Type:** correctness / user responsiveness
- **Location:** `src-tauri/src/commands_execution.rs` — `start_live_execution_runner`, `stop_live_execution_runner`, `on_live_execution_bar`, `execute_strategy_signal_blocking`
- **Timing boundary affected:** app-path
- **Why it matters:** `stop_live_execution_runner()` currently removes the runner from the global slot, but an already spawned in-flight task can continue through live-bar application, candle reload, strategy recompute, position fetch, and broker order submission. A fast stop/restart on the same symbol can also let an old task mutate the new runtime because the current completion logic only checks symbol equality.
- **Recommended change:** add a runner generation plus cancellation token, cancel the previous runtime when replacing it, and require generation/token match before mutating runtime state or emitting completion events. Also add cancellation checks before expensive stages and before broker submission.

### Exact implementation plan

### 2.1.1 Add runner generation and cancellation state

Add imports near the top of `commands_execution.rs`:

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
```

Add constants and helpers near the existing runner statics:

```rust
const LIVE_EXECUTION_RUNNER_EVENT: &str = "live_execution_runner_event";
const RUNNER_CANCELLED_ERR: &str = "__live_execution_runner_cancelled__";

static LIVE_EXECUTION_RUNNER: OnceLock<Mutex<Option<LiveExecutionRunnerRuntime>>> = OnceLock::new();
static LIVE_EXECUTION_RUNNER_GEN: AtomicU64 = AtomicU64::new(1);

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
```

Extend the runtime:

```rust
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
```

### 2.1.2 Cancel the prior runtime on replace

In `start_live_execution_runner(...)`, allocate a fresh generation and cancellation handle, and explicitly cancel the prior runtime before overwriting it:

```rust
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
    let generation = LIVE_EXECUTION_RUNNER_GEN.fetch_add(1, Ordering::SeqCst);
    let runtime = LiveExecutionRunnerRuntime {
        generation,
        abort: RunnerAbortHandle::new(),
        pending_bar: None,
        cfg: LiveExecutionRunnerInternalCfg {
            req: req.clone(),
            symbol: symbol.clone(),
            qty,
            data_id: data_id.clone(),
            strategy_kind: strategy_kind.clone(),
        },
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
```

### 2.1.3 Make stop authoritative before removing the slot

In `stop_live_execution_runner(...)`, cancel before `take()`:

```rust
#[tauri::command]
pub fn stop_live_execution_runner(
    app: tauri::AppHandle,
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
    Ok(StopLiveExecutionRunnerResponse {
        stopped,
        prior,
        status,
    })
}
```

### 2.1.4 Add cancellation checks inside blocking execution

Update the signature:

```rust
fn execute_strategy_signal_blocking(
    req: ExecuteStrategySignalRequest,
    symbol: String,
    qty: f64,
    candles: Arc<Candles>,
    abort: Option<&RunnerAbortHandle>,
) -> Result<ExecuteStrategySignalResponse, String> {
    if let Some(abort) = abort {
        abort.check()?;
    }

    let decision = signal_decision_from_request(&req.strategy, candles.as_ref(), &req.risk)?;

    if let Some(abort) = abort {
        abort.check()?;
    }

    let target_position_qty = (decision.desired_position as f64) * qty;
    let mut warnings: Vec<String> = Vec::new();
    let mut current_position_qty = 0.0_f64;

    if !req.dry_run {
        if req.alpaca_api_key.trim().is_empty() || req.alpaca_api_secret.trim().is_empty() {
            return Err(
                "alpaca_api_key and alpaca_api_secret are required unless dry_run=true".to_string(),
            );
        }
        let client = alpaca_client()?;
        current_position_qty = fetch_alpaca_position_qty(
            &client,
            req.mode,
            req.alpaca_api_key.trim(),
            req.alpaca_api_secret.trim(),
            &symbol,
        )?;
    }

    if let Some(abort) = abort {
        abort.check()?;
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

    if let Some(abort) = abort {
        abort.check()?;
    }

    let client = alpaca_client()?;
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

    if let Some(abort) = abort {
        abort.check()?;
    }

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
```

Update the one-shot path to pass `None`:

```rust
tauri::async_runtime::spawn_blocking(move || {
    execute_strategy_signal_blocking(req, symbol, qty, candles, None)
})
```

### 2.1.5 Make `on_live_execution_bar(...)` generation-safe and pending-bar aware

The safest minimal behavior is:

- ignore bars older than `last_bar_open_unix_ms`
- if the runner is busy, keep **one pending latest bar** instead of dropping everything
- capture `generation` + `abort` in the spawned task
- only mutate runtime state if generation still matches
- do not emit a normal completion if the task was cancelled
- after completion, immediately trigger the latest pending bar if still current

Targeted patch shape:

```rust
pub(crate) fn on_live_execution_bar(app: &tauri::AppHandle, bar: LiveExecutionRunnerBar) {
    let mut cfg_to_run: Option<LiveExecutionRunnerInternalCfg> = None;
    let mut generation_to_run: Option<u64> = None;
    let mut abort_to_run: Option<RunnerAbortHandle> = None;
    let mut queued_latest = false;
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
            match runtime.pending_bar.as_ref() {
                Some(existing) if existing.time_open_unix_ms >= bar.time_open_unix_ms => {}
                _ => runtime.pending_bar = Some(bar.clone()),
            }
            queued_latest = true;
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

    let (Some(cfg), Some(generation), Some(abort)) = (cfg_to_run, generation_to_run, abort_to_run) else {
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
            emit_live_execution_runner_event(
                &app2,
                "cancelled",
                "runner cancelled before live-bar processing",
                cfg.symbol.clone(),
                Some(bar.time_open_unix_ms),
                None,
            );
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
            let mut next_bar: Option<LiveExecutionRunnerBar> = None;
            if let Ok(mut slot) = live_execution_runner_slot().lock() {
                if let Some(runtime) = slot.as_mut() {
                    if runtime.generation == generation {
                        runtime.inflight = false;
                        runtime.error_count = runtime.error_count.saturating_add(1);
                        runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                        runtime.last_error = Some(e.clone());
                        next_bar = runtime.pending_bar.take();
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
            if let Some(next) = next_bar {
                on_live_execution_bar(&app2, next);
            }
            return;
        }

        if abort.check().is_err() {
            emit_live_execution_runner_event(
                &app2,
                "cancelled",
                "runner cancelled before candle reload",
                cfg.symbol.clone(),
                Some(bar.time_open_unix_ms),
                None,
            );
            return;
        }

        let candles = match state.get_candles(&cfg.data_id) {
            Ok(c) => c,
            Err(e) => {
                let mut next_bar: Option<LiveExecutionRunnerBar> = None;
                if let Ok(mut slot) = live_execution_runner_slot().lock() {
                    if let Some(runtime) = slot.as_mut() {
                        if runtime.generation == generation {
                            runtime.inflight = false;
                            runtime.error_count = runtime.error_count.saturating_add(1);
                            runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                            runtime.last_error = Some(e.clone());
                            next_bar = runtime.pending_bar.take();
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
                if let Some(next) = next_bar {
                    on_live_execution_bar(&app2, next);
                }
                return;
            }
        };

        let req = cfg.req.clone();
        let symbol = cfg.symbol.clone();
        let qty = cfg.qty;
        let abort2 = abort.clone();

        let exec_result = tauri::async_runtime::spawn_blocking(move || {
            execute_strategy_signal_blocking(req, symbol, qty, candles, Some(&abort2))
        })
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);

        let mut next_bar: Option<LiveExecutionRunnerBar> = None;

        match exec_result {
            Ok(resp) => {
                if let Ok(mut slot) = live_execution_runner_slot().lock() {
                    if let Some(runtime) = slot.as_mut() {
                        if runtime.generation == generation {
                            runtime.inflight = false;
                            runtime.run_count = runtime.run_count.saturating_add(1);
                            runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                            runtime.last_action = Some(resp.action.clone());
                            runtime.last_order_id = resp.order_id.clone();
                            runtime.last_error = None;
                            next_bar = runtime.pending_bar.take();
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
            Err(e) if e == RUNNER_CANCELLED_ERR => {
                if let Ok(mut slot) = live_execution_runner_slot().lock() {
                    if let Some(runtime) = slot.as_mut() {
                        if runtime.generation == generation {
                            runtime.inflight = false;
                            runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                            runtime.last_error = Some("cancelled".to_string());
                            runtime.pending_bar = None;
                        }
                    }
                }
                emit_live_execution_runner_event(
                    &app2,
                    "cancelled",
                    "runner cancelled before order submission",
                    cfg.symbol.clone(),
                    Some(bar.time_open_unix_ms),
                    None,
                );
            }
            Err(e) => {
                if let Ok(mut slot) = live_execution_runner_slot().lock() {
                    if let Some(runtime) = slot.as_mut() {
                        if runtime.generation == generation {
                            runtime.inflight = false;
                            runtime.error_count = runtime.error_count.saturating_add(1);
                            runtime.last_completed_at_unix_ms = Some(now_unix_ms());
                            runtime.last_error = Some(e.clone());
                            next_bar = runtime.pending_bar.take();
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

        if let Some(next) = next_bar {
            on_live_execution_bar(&app2, next);
        }
    });
}
```

### Notes

- This does **not** guarantee cancellation of a broker POST already in flight; it makes stop authoritative **before** the submit boundary and narrows the remaining window with timeouts.
- This also fixes the same-symbol restart hazard because generation checks replace symbol-only matching for stale task completion.

---

## Finding 2

- **Severity:** high
- **Type:** performance / user responsiveness
- **Location:** `src-tauri/src/commands_execution.rs` — `execute_strategy_signal_blocking`, `test_alpaca_connection`, `get_live_execution_snapshot`
- **Timing boundary affected:** app-path
- **Why it matters:** the current code repeatedly creates `reqwest::blocking::Client::new()` with no explicit timeout configuration. Slow DNS, TLS, or broker response time can stall blocking threads and make UI operations feel hung.
- **Recommended change:** use one shared, timeout-configured Alpaca client.

### Exact implementation plan

Add a shared Alpaca client helper near the other networking helpers:

```rust
static ALPACA_HTTP_CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();

fn alpaca_client() -> Result<Client, String> {
    ALPACA_HTTP_CLIENT
        .get_or_init(|| {
            Client::builder()
                .user_agent("vectorgrid/0.1 (tauri)")
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(12))
                .pool_idle_timeout(Duration::from_secs(90))
                .build()
                .map_err(|e| format!("failed to build Alpaca HTTP client: {e}"))
        })
        .as_ref()
        .map(Clone::clone)
        .map_err(Clone::clone)
}
```

Then replace all `Client::new()` call sites with `alpaca_client()?`.

Examples:

```rust
let client = alpaca_client()?;
```

Change in `test_alpaca_connection(...)`:

```rust
tauri::async_runtime::spawn_blocking(move || {
    let client = alpaca_client()?;
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
```

Change in `get_live_execution_snapshot(...)` similarly.

### Why this is a safe change

- no feature behavior changes
- no payload changes
- no route changes
- only connection reuse and bounded network latency

---

## Finding 3

- **Severity:** high
- **Type:** performance
- **Location:** `src-tauri/src/commands_execution.rs` — `get_live_execution_snapshot`; `web/src/services/executionService.ts` — `refreshOrders`
- **Timing boundary affected:** app-path
- **Why it matters:** the common UI “refresh orders” path always pays for account status, order list, and full positions list even though the frontend only uses orders + position. That is avoidable work.
- **Recommended change:** add request flags to `GetLiveExecutionSnapshotRequest`, skip account fetch for `refreshOrders`, and use the single-symbol position endpoint when `symbol` is provided.

### Exact implementation plan

### 2.3.1 Extend the request

```rust
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
```

### 2.3.2 Add single-symbol position fetch

```rust
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

    let v: serde_json::Value = resp
        .json()
        .map_err(|e| format!("failed to decode Alpaca position response: {e}"))?;
    Ok(Some(v))
}
```

### 2.3.3 Use flags in `get_live_execution_snapshot(...)`

```rust
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
    let include_account = req.include_account.unwrap_or(true);
    let include_orders = req.include_orders.unwrap_or(true);
    let include_position = req.include_position.unwrap_or(true);

    tauri::async_runtime::spawn_blocking(move || {
        let client = alpaca_client()?;

        let account_status = if include_account {
            let account = fetch_alpaca_account(&client, mode, &api_key, &api_secret)?;
            json_field_as_string(&account, "status")
        } else {
            None
        };

        let orders = if include_orders {
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
            orders
        } else {
            Vec::new()
        };

        let position = if include_position {
            if symbol.is_empty() {
                let positions_raw = fetch_alpaca_positions(&client, mode, &api_key, &api_secret)?;
                positions_raw
                    .iter()
                    .filter_map(normalize_position_snapshot)
                    .next()
            } else {
                fetch_alpaca_position(&client, mode, &api_key, &api_secret, &symbol)?
                    .as_ref()
                    .and_then(normalize_position_snapshot)
            }
        } else {
            None
        };

        let fetched_at_unix_ms = now_unix_ms();

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
```

### 2.3.4 Frontend change for `refreshOrders(...)`

In `executionService.ts`, change the invoke payload:

```ts
const resp = (await invokeCommand('get_live_execution_snapshot', {
  req: {
    mode,
    alpaca_api_key: api.apiKey,
    alpaca_api_secret: api.apiSecret,
    symbol,
    limit: 200,
    include_account: false,
    include_orders: true,
    include_position: true,
  },
})) as ExecutionSnapshotResponse
```

### Optional follow-up

If you later identify another caller that truly needs account status, let that call opt in with `include_account: true` rather than making every refresh pay for it.

---

## Finding 4

- **Severity:** high
- **Type:** correctness / user responsiveness
- **Location:** `web/src/stores/executionStore.ts`
- **Timing boundary affected:** app-path
- **Why it matters:** the store uses one global `isBusy` gate. When any action is active, stop and status-refresh actions are blocked. If `runOnce`, `refreshOrders`, or `testConnection` is slow, the user loses the ability to stop or refresh runtime state.
- **Recommended change:** replace the global gate with per-action pending flags and derive availability from those flags. Keep stop actions and status refresh available while another action is running.

### Exact implementation plan

### 2.4.1 Add per-action pending state

Add this near the store types:

```ts
interface ExecutionPendingState {
  refreshStatuses: boolean
  testConnection: boolean
  runOnce: boolean
  refreshOrders: boolean
  startLive: boolean
  stopLive: boolean
  startRunner: boolean
  stopRunner: boolean
  clearKeys: boolean
}
```

Add helpers:

```ts
function isAnyPending(state: ExecutionState): boolean {
  return Object.values(state.pending).some(Boolean)
}

function beginPending(
  state: ExecutionState,
  key: keyof ExecutionPendingState,
  statusLine: string,
): void {
  if (statusLine.trim()) state.statusLine = statusLine
  state.pending[key] = true
  setAvailability(state)
  markVue(state)
}

function finishPending(
  state: ExecutionState,
  key: keyof ExecutionPendingState,
): void {
  state.pending[key] = false
  setAvailability(state)
  markVue(state)
}
```

Extend `ExecutionState`:

```ts
interface ExecutionState {
  // existing fields...
  pending: ExecutionPendingState
  isBusy: boolean
  // ...
}
```

Initialize it in `state()`:

```ts
pending: {
  refreshStatuses: false,
  testConnection: false,
  runOnce: false,
  refreshOrders: false,
  startLive: false,
  stopLive: false,
  startRunner: false,
  stopRunner: false,
  clearKeys: false,
},
isBusy: false,
```

### 2.4.2 Rewrite `setAvailability(...)`

Replace the current implementation with something like:

```ts
function setAvailability(state: ExecutionState): void {
  const primaryBusy =
    state.pending.testConnection ||
    state.pending.runOnce ||
    state.pending.refreshOrders ||
    state.pending.startLive ||
    state.pending.startRunner ||
    state.pending.clearKeys

  state.isBusy = isAnyPending(state)

  state.canTestConnection = !primaryBusy && !state.pending.testConnection
  state.canRunOnce = !primaryBusy && !state.pending.runOnce
  state.canRefreshOrders = !primaryBusy && !state.pending.refreshOrders

  state.canStartLive =
    !primaryBusy &&
    !state.pending.startLive &&
    !state.pending.stopLive &&
    !state.liveActive

  state.canStopLive =
    state.liveActive &&
    !state.pending.stopLive

  state.canStartRunner =
    !primaryBusy &&
    !state.pending.startRunner &&
    !state.pending.stopRunner &&
    !state.runnerActive

  state.canStopRunner =
    state.runnerActive &&
    !state.pending.stopRunner

  state.canClearKeys =
    !primaryBusy &&
    !state.pending.clearKeys
}
```

### 2.4.3 Change action wrappers to use action keys

Replace `beginAction(...)` / `finishAction()` with keyed versions:

```ts
beginAction(key: keyof ExecutionPendingState, statusLine: string): void {
  beginPending(this, key, statusLine)
},

finishAction(key: keyof ExecutionPendingState): void {
  finishPending(this, key)
},
```

Then update each action:

```ts
async refreshStatuses(): Promise<void> {
  if (this.pending.refreshStatuses) return
  this.beginAction('refreshStatuses', 'Refreshing execution status...')
  try {
    const result = await refreshStatusesAction()
    this.applyActionResult(result)
  } catch (error) {
    this.failAction('Execution status refresh failed', error)
  } finally {
    this.finishAction('refreshStatuses')
  }
},
```

```ts
runOnce(): void {
  if (this.pending.runOnce) return
  this.beginAction('runOnce', 'Executing signal...')
  void runOnceAction(this.buildActionOptions())
    .then((result) => this.applyActionResult(result))
    .catch((error) => this.failAction('Execution failed', error))
    .finally(() => this.finishAction('runOnce'))
},
```

Apply the same pattern to `testConnection`, `refreshOrders`, `startLive`, `stopLive`, `startRunner`, `stopRunner`, and `clearKeys`.

### Why this change is required before event wiring

If you wire runtime events into the store **without** fixing pending state, concurrent control actions can cause `finishAction()` from one path to incorrectly re-enable the whole surface while another action is still active.

---

## Finding 5

- **Severity:** high
- **Type:** user responsiveness / maintainability
- **Location:** `web/src/services/executionService.ts`, `web/src/stores/executionStore.ts`, optionally `web/src/tauri/events.ts`
- **Timing boundary affected:** app-path
- **Why it matters:** the backend already emits `polygon_live_status` and `live_execution_runner_event`, but the provided frontend execution surface is still primarily command/result-driven. That creates state lag and requires manual refreshes.
- **Recommended change:** subscribe to runtime events, feed them into `executionStore`, and let events reconcile live/runner state in near-real time.

### Exact implementation plan

### 2.5.1 Add richer live-status typing in `executionService.ts`

Replace the thin live-status interface with one that preserves stream metadata:

```ts
interface LiveCandleStreamInfoResponse {
  symbol?: unknown
  delayed?: unknown
  market?: unknown
}

interface LiveCandleStreamStatusResponse {
  active?: unknown
  stream?: unknown
}
```

Add event payload interfaces:

```ts
export interface PolygonLiveStatusEventPayload {
  status?: unknown
  message?: unknown
  market?: unknown
  symbol?: unknown
  delayed?: unknown
  channel?: unknown
  at_unix_ms?: unknown
}

export interface LiveExecutionRunnerEventPayload {
  status?: unknown
  message?: unknown
  symbol?: unknown
  at_unix_ms?: unknown
  bar_open_unix_ms?: unknown
  execution?: unknown
}
```

### 2.5.2 Add runtime subscription helper in `executionService.ts`

Add this near the bottom of the file:

```ts
import { listenEvent } from '../tauri/events'

export async function subscribeExecutionRuntimeEvents(handlers: {
  onPolygonLiveStatus?: (payload: PolygonLiveStatusEventPayload) => void
  onLiveExecutionRunnerEvent?: (payload: LiveExecutionRunnerEventPayload) => void
}): Promise<() => void> {
  const offs: Array<(() => void) | null> = []

  offs.push(
    await listenEvent<PolygonLiveStatusEventPayload>('polygon_live_status', (payload) => {
      handlers.onPolygonLiveStatus?.(payload)
    }),
  )

  offs.push(
    await listenEvent<LiveExecutionRunnerEventPayload>('live_execution_runner_event', (payload) => {
      handlers.onLiveExecutionRunnerEvent?.(payload)
    }),
  )

  return () => {
    for (const off of offs) {
      try {
        off?.()
      } catch {
        // no-op
      }
    }
  }
}
```

### 2.5.3 Optional helper in `events.ts`

The current `listenEvent(...)` helper is fine. If you want cleaner bulk subscription management, add an optional aggregator helper:

```ts
export async function listenEvents(
  entries: Array<{
    event: string
    handler: (payload: unknown, meta: { event: string; id?: number }) => void
  }>,
): Promise<() => void> {
  const offs = await Promise.all(
    entries.map(({ event, handler }) => listenEvent(event, handler)),
  )
  return () => {
    for (const off of offs) {
      try {
        off?.()
      } catch {
        // no-op
      }
    }
  }
}
```

This change is optional; the service helper above is sufficient.

### 2.5.4 Feed runtime events into `executionStore.ts`

At module scope:

```ts
import {
  clearKeys as clearKeysAction,
  loadPersistedExecutionKeys,
  refreshOrders as refreshOrdersAction,
  refreshStatuses as refreshStatusesAction,
  runOnce as runOnceAction,
  startLive as startLiveAction,
  startRunner as startRunnerAction,
  stopLive as stopLiveAction,
  stopRunner as stopRunnerAction,
  subscribeExecutionRuntimeEvents,
  testConnection as testConnectionAction,
  type ExecutionActionResult,
  type LiveExecutionRunnerEventPayload,
  type PolygonLiveStatusEventPayload,
} from '../services/executionService'

let runtimeEventsOff: (() => void) | null = null
```

Add actions:

```ts
async ensureRuntimeSubscriptions(): Promise<void> {
  if (runtimeEventsOff) return

  runtimeEventsOff = await subscribeExecutionRuntimeEvents({
    onPolygonLiveStatus: (payload) => this.applyPolygonLiveStatusEvent(payload),
    onLiveExecutionRunnerEvent: (payload) => this.applyLiveExecutionRunnerEvent(payload),
  })
},
```

```ts
applyPolygonLiveStatusEvent(payload: PolygonLiveStatusEventPayload): void {
  const status = String(payload.status ?? '').trim().toLowerCase()
  const message = String(payload.message ?? '').trim()
  const symbol = String(payload.symbol ?? '').trim().toUpperCase()
  const delayed = !!payload.delayed

  const label = symbol
    ? `${symbol}${delayed ? ' [delayed]' : ''}`
    : 'live candles'

  this.liveStatusHint = message
    ? `${label}: ${message}`
    : `${label}: ${status || 'updated'}`

  if (status === 'stopped' || status === 'closed' || status === 'error') {
    this.liveActive = false
  } else if (
    status === 'started' ||
    status === 'connecting' ||
    status === 'connected' ||
    status === 'auth_success' ||
    status === 'subscribed'
  ) {
    this.liveActive = true
  }

  this.statusLine = this.liveStatusHint
  setAvailability(this)
  markVue(this)
},
```

```ts
applyLiveExecutionRunnerEvent(payload: LiveExecutionRunnerEventPayload): void {
  const status = String(payload.status ?? '').trim().toLowerCase()
  const message = String(payload.message ?? '').trim()
  const symbol = String(payload.symbol ?? '').trim().toUpperCase()

  this.runnerStatusHint = symbol
    ? `Auto-runner (${symbol}): ${message || status || 'updated'}`
    : `Auto-runner: ${message || status || 'updated'}`

  this.statusLine = this.runnerStatusHint

  if (status === 'started') {
    this.runnerActive = true
  } else if (status === 'stopped' || status === 'cancelled') {
    this.runnerActive = false
  }

  const execution =
    payload.execution && typeof payload.execution === 'object'
      ? (payload.execution as Record<string, unknown>)
      : null

  const orderId = execution ? String(execution.order_id ?? '').trim() : ''
  const action = execution ? String(execution.action ?? '').trim() : ''
  const deltaQty = execution ? Number(execution.delta_qty ?? Number.NaN) : Number.NaN

  if (status === 'order_submitted' && orderId) {
    const optimisticRow = markRaw({
      cells: markRaw([
        new Date(Number(payload.at_unix_ms || Date.now())).toISOString(),
        action || '-',
        'submitted',
        Number.isFinite(deltaQty) ? String(Math.abs(deltaQty)) : '-',
        '-',
        '-',
        orderId,
      ]),
    })
    this.orderRows = markRaw([optimisticRow, ...this.orderRows].slice(0, 200))
    this.orderCount = this.orderRows.length
  }

  setAvailability(this)
  markVue(this)
},
```

### 2.5.5 Ensure subscriptions exist before the surface is used

The best place is the workspace mount/bootstrap path, but that file was not provided here. Within the provided store surface, the safe minimum is to call `ensureRuntimeSubscriptions()` before refresh/status and control flows.

Example:

```ts
async refreshStatuses(): Promise<void> {
  await this.ensureRuntimeSubscriptions()
  if (this.pending.refreshStatuses) return
  this.beginAction('refreshStatuses', 'Refreshing execution status...')
  try {
    const result = await refreshStatusesAction()
    this.applyActionResult(result)
  } catch (error) {
    this.failAction('Execution status refresh failed', error)
  } finally {
    this.finishAction('refreshStatuses')
  }
},
```

For fire-and-forget actions:

```ts
startRunner(): void {
  void this.ensureRuntimeSubscriptions()
  if (this.pending.startRunner) return
  this.beginAction('startRunner', 'Starting auto-runner...')
  void startRunnerAction(this.buildActionOptions())
    .then((result) => this.applyActionResult(result))
    .catch((error) => this.failAction('Auto-runner start failed', error))
    .finally(() => this.finishAction('startRunner'))
},
```

---

## Finding 6

- **Severity:** high
- **Type:** correctness / user responsiveness
- **Location:** `web/src/services/executionService.ts` — `startRunner`
- **Timing boundary affected:** app-path
- **Why it matters:** `startRunner()` currently only checks whether *some* live stream is active. It does not check whether the stream symbol matches the runner symbol. That lets the runner start successfully but never receive matching bars.
- **Recommended change:** verify live stream symbol before starting the runner.

### Exact implementation plan

Add a small helper in `executionService.ts`:

```ts
function parseLiveStreamStatus(raw: LiveCandleStreamStatusResponse): {
  active: boolean
  symbol: string
  delayed: boolean
} {
  const active = !!raw?.active
  const stream = isRecord(raw?.stream) ? raw.stream as LiveCandleStreamInfoResponse : null
  return {
    active,
    symbol: text(stream?.symbol).toUpperCase(),
    delayed: !!stream?.delayed,
  }
}
```

Use it in `refreshLiveStatusHint()`:

```ts
async function refreshLiveStatusHint(): Promise<{ hint: string; active: boolean }> {
  const raw = (await invokeCommand('polygon_live_candle_stream_status')) as LiveCandleStreamStatusResponse
  const live = parseLiveStreamStatus(raw)

  return {
    hint: live.active
      ? `Live candles active (${live.symbol || '-'}${live.delayed ? ' [delayed]' : ''}).`
      : 'Live candles: off',
    active: live.active,
  }
}
```

Use it in `startRunner()`:

```ts
export async function startRunner(options: ExecutionActionOptions): Promise<ExecutionActionResult> {
  const payload = await buildExecutionInvokePayload(options)
  const req = payload.req
  if (!req.dry_run && (!text(req.alpaca_api_key) || !text(req.alpaca_api_secret))) {
    throw new Error('API key + secret are required unless dry-run is enabled.')
  }

  const liveRaw = (await invokeCommand('polygon_live_candle_stream_status')) as LiveCandleStreamStatusResponse
  const live = parseLiveStreamStatus(liveRaw)

  if (!live.active) {
    throw new Error('Start live candles first. Auto-runner triggers on Polygon live bars.')
  }

  const runnerSymbol = text(req.symbol).toUpperCase()
  if (live.symbol && live.symbol !== runnerSymbol) {
    throw new Error(
      `Live candles are currently running for ${live.symbol}, but the auto-runner is configured for ${runnerSymbol}. Start live candles for the same symbol first.`,
    )
  }

  const statusRaw = await invokeCommand('start_live_execution_runner', { req })
  syncPersistedExecutionKeys(options)
  const summary = summarizeRunnerStatus(statusRaw)
  return {
    statusLine: `Auto-runner started for ${text(req.symbol) || '-'}.`,
    runnerStatusHint: summary.text,
    runnerActive: summary.active,
  }
}
```

---

## Finding 7

- **Severity:** medium
- **Type:** performance
- **Location:** `web/src/services/executionService.ts` — `runOnce`
- **Timing boundary affected:** app-path
- **Why it matters:** a successful broker submit triggers a second broker round-trip immediately through `refreshOrders()`. That makes the success path slower and blocks the UI longer.
- **Recommended change:** remove the immediate broker snapshot from `runOnce()` and use optimistic UI state. Keep the explicit Refresh Orders action for authoritative broker state.

### Exact implementation plan

### 2.7.1 Extend the response typing

In `executionService.ts`:

```ts
interface ExecuteSignalResponse {
  action?: unknown
  order_submitted?: unknown
  order_id?: unknown
  warnings?: unknown
  delta_qty?: unknown
}
```

### 2.7.2 Add optimistic order-row helper

```ts
function optimisticOrderRowFromExecution(resp: ExecuteSignalResponse): ExecutionOrderRow[] {
  const orderId = text(resp?.order_id)
  if (!orderId) return []

  const action = text(resp?.action) || '-'
  const deltaQty = Math.abs(toNumber(resp?.delta_qty))
  const qtyText = Number.isFinite(deltaQty) ? trimTrailingZeros(deltaQty) : '-'

  return [
    {
      cells: [
        new Date().toISOString(),
        action,
        'submitted',
        qtyText,
        '-',
        '-',
        orderId,
      ],
    },
  ]
}
```

### 2.7.3 Change `runOnce(...)`

Replace the post-submit refresh block:

```ts
if (submitted && !req.dry_run) {
  try {
    const snapshot = await refreshOrders(options)
    if (Array.isArray(snapshot.orderRows)) result.orderRows = snapshot.orderRows
    if (typeof snapshot.orderCount === 'number') result.orderCount = snapshot.orderCount
    if (typeof snapshot.positionMeta === 'string') result.positionMeta = snapshot.positionMeta
    if (typeof snapshot.chartMeta === 'string') result.chartMeta = snapshot.chartMeta
  } catch {
    // keep run-once success non-fatal when refresh fails
  }
}
```

with:

```ts
if (submitted && !req.dry_run) {
  const optimisticRows = optimisticOrderRowFromExecution(resp)
  if (optimisticRows.length) {
    result.orderRows = optimisticRows
    result.orderCount = optimisticRows.length
    result.positionMeta = 'Position: broker snapshot pending'
    result.chartMeta = chartMeta(req.symbol, optimisticRows.length)
  }
  result.statusHint = `${statusHint} Broker snapshot not refreshed automatically. Use Refresh Orders for authoritative broker state.`
}
```

### Why this is the right tradeoff

- removes the second broker round-trip from the hot success path
- still shows immediate user feedback
- preserves explicit broker refresh for authoritative order fill/position state

If you later decide you want optional auto-refresh, make it explicit and opt-in.

---

## Finding 8

- **Severity:** medium
- **Type:** correctness / state drift
- **Location:** `web/src/services/executionService.ts` — `clearKeys`; `web/src/stores/executionStore.ts` — `clearKeys`
- **Timing boundary affected:** app-path
- **Why it matters:** `clearKeys()` currently returns `liveActive=false`, `runnerActive=false`, and “Live candles: off / Auto-runner: off” even though it only clears local key storage. That creates false runtime state.
- **Recommended change:** make `clearKeys()` clear only keys and key-related output, not runtime state.

### Exact implementation plan

Change `clearKeys()` in `executionService.ts` from:

```ts
export async function clearKeys(): Promise<ExecutionActionResult> {
  try {
    removeStorageKey(EXEC_KEYS_STORAGE_KEY)
  } catch {
    // best-effort local key cleanup only
  }
  return {
    statusLine: 'Cleared execution API keys.',
    statusHint: 'Cleared execution API keys.',
    liveStatusHint: 'Live candles: off',
    runnerStatusHint: 'Auto-runner: off',
    outputPreview: '-',
    orderRows: [],
    orderCount: 0,
    positionMeta: 'Position: -',
    chartMeta: 'Load a dataset and refresh orders to overlay broker events.',
    liveActive: false,
    runnerActive: false,
  }
}
```

to:

```ts
export async function clearKeys(): Promise<ExecutionActionResult> {
  try {
    removeStorageKey(EXEC_KEYS_STORAGE_KEY)
  } catch {
    // best-effort local key cleanup only
  }
  return {
    statusLine: 'Cleared execution API keys.',
    statusHint: 'Cleared execution API keys.',
    outputPreview: '-',
  }
}
```

The store action can still clear the text inputs immediately:

```ts
clearKeys(): void {
  if (this.pending.clearKeys) return
  this.beginAction('clearKeys', 'Clearing execution keys...')
  this.apiKey = ''
  this.apiSecret = ''
  void clearKeysAction()
    .then((result) => this.applyActionResult(result))
    .catch((error) => this.failAction('Clear keys failed', error))
    .finally(() => this.finishAction('clearKeys'))
},
```

This keeps runtime truth separate from credential storage.

---

## Finding 9

- **Severity:** medium
- **Type:** user responsiveness
- **Location:** `web/src/services/runControlService.ts`
- **Timing boundary affected:** app-path
- **Why it matters:** cancellation is probed serially. If the first command is slow to reject or hangs, the background-task cancellation path waits unnecessarily.
- **Recommended change:** start the background-task probe in parallel while still preserving the primary/fallback logic.

### Exact implementation plan

Replace the current implementation with:

```ts
import { invokeCommand } from '../tauri/invoke'

export interface CancelCurrentRunResult {
  requested: boolean
  usedLegacyFallback: boolean
  backgroundTaskRequested: boolean
}

async function tryInvoke(command: string): Promise<boolean> {
  try {
    await invokeCommand(command)
    return true
  } catch {
    return false
  }
}

export async function requestCancelCurrentRun(): Promise<CancelCurrentRunResult> {
  const backgroundTaskPromise = tryInvoke('cancel_background_task')

  const primaryRequested = await tryInvoke('cancel_current_run')
  const legacyRequested = primaryRequested
    ? false
    : await tryInvoke('cancel_double_ma_optimization')

  const backgroundTaskRequested = await backgroundTaskPromise
  const requested = primaryRequested || legacyRequested || backgroundTaskRequested

  if (!requested) {
    throw new Error('Cancellation command is unavailable.')
  }

  return {
    requested,
    usedLegacyFallback: !primaryRequested && legacyRequested,
    backgroundTaskRequested,
  }
}
```

### Optional add-on

If you see frequent hangs at the invoke layer, add a small timeout wrapper:

```ts
async function tryInvoke(command: string, timeoutMs = 2500): Promise<boolean> {
  try {
    await Promise.race([
      invokeCommand(command),
      new Promise((_, reject) => {
        setTimeout(() => reject(new Error('timeout')), timeoutMs)
      }),
    ])
    return true
  } catch {
    return false
  }
}
```

Keep that optional unless you have evidence of invoke-level hangs.

---

## Finding 10

- **Severity:** medium
- **Type:** user responsiveness / event completeness
- **Location:** `src-tauri/src/commands_data.rs` — `polygon_start_live_candle_stream`, `polygon_stop_live_candle_stream`
- **Timing boundary affected:** app-path
- **Why it matters:** the websocket task emits status events during connection/runtime, but the explicit start/stop commands do not currently emit a matching “started” / “stopped” status event. Command-returning surfaces cope, but event-driven surfaces benefit from symmetric runtime events.
- **Recommended change:** emit `started` and `stopped` live-status events directly from the command handlers.

### Exact implementation plan

### 2.10.1 Emit “started” in `polygon_start_live_candle_stream(...)`

Just before returning:

```rust
emit_polygon_live_status(
    &app,
    &market,
    &symbol,
    delayed,
    &channel,
    "started",
    "live candle stream requested",
);
```

### 2.10.2 Emit “stopped” in `polygon_stop_live_candle_stream(...)`

Change the signature to accept `app: tauri::AppHandle`:

```rust
#[tauri::command]
pub fn polygon_stop_live_candle_stream(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<PolygonStopLiveCandleStreamResponse, String> {
    let prior = state.stop_live_candle_stream()?;
    if let Some(info) = prior.as_ref() {
        let channel = format!("AM.{}", info.symbol);
        emit_polygon_live_status(
            &app,
            &info.market,
            &info.symbol,
            info.delayed,
            &channel,
            "stopped",
            "live candle stream stopped",
        );
    }
    Ok(PolygonStopLiveCandleStreamResponse {
        stopped: prior.is_some(),
        prior,
    })
}
```

No frontend invoke changes are required for this Tauri command signature expansion.

### Open question to verify in adjacent `state.rs`

The provided files do **not** show whether finished websocket tasks clear their stored runtime info in `state.rs`. Verify `live_candle_stream_info()` and the live-stream task cleanup path.

If finished tasks are not cleared automatically, add the adjacent fix there so `polygon_live_candle_stream_status()` cannot return `active=true` after an unexpected stream exit.

This is the one meaningful section-15 item that cannot be confirmed from the provided files alone.

---

## Finding 11

- **Severity:** likely medium
- **Type:** performance / correctness
- **Location:** `src-tauri/src/commands_execution.rs` — per-bar strategy recompute path
- **Timing boundary affected:** app-path
- **Why it matters:** every live bar reloads candles and recomputes strategy state over the current candle set. On large histories, that can push runner work past the next bar boundary.
- **Recommended change:** do not address this first. Fix stop correctness, busy gating, snapshot cost, and event wiring first. Then measure whether the remaining latency justifies an incremental live evaluator.

### Practical guidance

For this bundle, treat this as **deferred**. The first pass should stay parity-safe and small.

If later measurement still shows runner work near or beyond the one-minute bar cadence, the next step is:

- cap the live runner to the trailing lookback/warmup window required by the strategy, or
- build a live-only incremental evaluator

Do not start with that change.

---

## Finding 12

- **Severity:** open question / higher-risk opportunity
- **Type:** user responsiveness / resiliency
- **Location:** `src-tauri/src/commands_data.rs` — `run_polygon_live_candle_stream`
- **Timing boundary affected:** app-path
- **Why it matters:** the websocket loop exits on transient connection failure and does not attempt reconnect/backoff in the provided file.
- **Recommended change:** treat reconnect/backoff as a second-pass improvement, not a first-pass fix.

### Optional second-pass reconnect loop

If you choose to implement reconnect, wrap the connection flow in a bounded retry loop:

```rust
async fn run_polygon_live_candle_stream(
    app: tauri::AppHandle,
    ws_url: String,
    api_key: String,
    market: String,
    symbol: String,
    delayed: bool,
    channel: String,
) {
    let mut attempt: u32 = 0;

    loop {
        emit_polygon_live_status(
            &app,
            &market,
            &symbol,
            delayed,
            &channel,
            if attempt == 0 { "connecting" } else { "reconnecting" },
            format!("connecting to {ws_url}"),
        );

        let ws = connect_async(ws_url.as_str()).await;
        let (mut stream, _) = match ws {
            Ok(v) => v,
            Err(e) => {
                emit_polygon_live_status(
                    &app,
                    &market,
                    &symbol,
                    delayed,
                    &channel,
                    "error",
                    format!("connect failed: {e}"),
                );
                attempt = attempt.saturating_add(1);
                let backoff_ms = (1_000u64.saturating_mul(1u64 << attempt.min(5))).min(30_000);
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                continue;
            }
        };

        // existing auth + subscribe + read loop here

        break;
    }
}
```

Only do this after the first-pass fixes are working and measured.

---

## 3. Safe optimization opportunities

These are the safe, high-value changes to implement first.

### Priority A — required

1. **Authoritative runner stop/restart**
   - generation + cancellation token
   - pending latest bar coalescing
   - generation-safe completion updates

2. **Shared timed Alpaca client**
   - connection pooling
   - bounded latency
   - reused across test/run/snapshot

3. **Slim snapshot refresh**
   - request flags
   - skip account on order refresh
   - single-symbol position fetch

4. **Store pending flags**
   - stop/status available during long actions
   - no more global busy lock

5. **Runtime event wiring**
   - subscribe to `polygon_live_status`
   - subscribe to `live_execution_runner_event`
   - reconcile live/runner state without manual refresh

6. **Runner start symbol guard**
   - reject mismatch between live stream symbol and runner symbol

7. **Fix `clearKeys()` state drift**
   - clear only credentials
   - do not fake runtime state

### Priority B — still safe, but after A

8. **Optimistic `runOnce()` UI**
   - remove immediate broker snapshot
   - use manual refresh for authoritative broker state

9. **Parallel best-effort cancellation in `runControlService.ts`**
   - background cancellation no longer waits on the primary path

10. **Emit explicit live start/stop events**
    - improves event-driven frontend consistency

---

## 4. Higher-risk optimization opportunities

These are valid, but they should come **after** the safe changes are shipped and measured.

### 4.1 Websocket reconnect/backoff

Good value if users are seeing frequent disconnects, but it changes lifecycle semantics and should be measured after event wiring lands.

### 4.2 Incremental live strategy evaluation

Potentially large payoff, but higher parity risk than the rest of this bundle.

### 4.3 Async broker HTTP rewrite

You could convert the Alpaca surface from blocking `reqwest::blocking` + `spawn_blocking` to async `reqwest`, but the shared timeout-configured client already removes a large portion of the practical pain with far less risk.

Do **not** start here.

---

## 5. Benchmark and perf-gate review

There is no dedicated section-15 benchmark harness in the provided file set. That means the correct evidence boundary for this bundle is **app-path measurement**, not Criterion or Nsight.

### What to measure before and after

Capture:

1. **`testConnection()` latency**
   - median
   - p95
   - failure latency

2. **`refreshOrders()` latency**
   - median
   - p95
   - compare before vs after `include_account=false`

3. **`runOnce()` success path latency**
   - before and after removing auto-refresh

4. **`stopRunner()` responsiveness**
   - time from click to UI state update
   - confirm no post-stop order submission when stop happens before submit boundary

5. **same-symbol restart safety**
   - start runner
   - trigger a slow in-flight bar
   - stop + restart same symbol
   - confirm old task cannot mutate new runtime

6. **busy-bar handling**
   - simulate a slow run while new live bars arrive
   - confirm latest pending bar is processed after completion

### Success thresholds

Reasonable section-15 acceptance targets:

- stop/status controls remain clickable while another action is running
- `refreshOrders()` gets materially faster after skipping account fetch
- `runOnce()` feels immediate on submit because it no longer blocks on broker snapshot
- no stale same-symbol task can update the new runner state
- no silent “runner started but wrong stream symbol” situation remains

### Minimal diagnostic instrumentation (optional)

If you want quick timing visibility without building a new harness, add temporary timing logs around the backend command bodies.

Example in Rust:

```rust
let started = std::time::Instant::now();
// existing body...
eprintln!(
    "[execution] get_live_execution_snapshot symbol={} elapsed_ms={}",
    symbol,
    started.elapsed().as_millis()
);
```

Use this only as temporary evidence if you do not already have another app-path timing surface.

---

## 6. Test review

The provided tests cover only a subset of the section-15 surface.

### Current gaps

- no test for live-stream symbol mismatch when starting runner
- no test for slimmer snapshot request flags
- no test for `clearKeys()` state-truthfulness
- no test for new optimistic `runOnce()` path
- no test for event subscription/update behavior
- no test for per-action pending flags in `executionStore.ts`
- no Rust-side test coverage for generation/cancellation/pending-bar helper behavior

### Required test updates

## 6.1 `web/src/services/executionService.test.ts`

Add imports:

```ts
import {
  clearKeys,
  loadPersistedExecutionKeys,
  refreshOrders,
  runOnce,
  startRunner,
  testConnection,
} from './executionService'
```

### Test: `refreshOrders()` requests the slim snapshot

```ts
it('refreshes orders without fetching account status', async () => {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'get_live_execution_snapshot') {
      return {
        symbol: 'AAPL',
        position: null,
        orders: [],
      }
    }
    return {}
  })

  await refreshOrders(baseOptions())

  expect(invokeCommandMock).toHaveBeenCalledWith('get_live_execution_snapshot', {
    req: expect.objectContaining({
      mode: 'paper',
      symbol: 'AAPL',
      include_account: false,
      include_orders: true,
      include_position: true,
    }),
  })
})
```

### Test: `startRunner()` rejects mismatched live-stream symbol

```ts
it('rejects auto-runner start when live candles are for a different symbol', async () => {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'polygon_live_candle_stream_status') {
      return {
        active: true,
        stream: { symbol: 'TSLA', delayed: true, market: 'stocks' },
      }
    }
    return {}
  })

  await expect(startRunner(baseOptions())).rejects.toThrow(
    'Live candles are currently running for TSLA',
  )
})
```

### Test: `runOnce()` no longer auto-refreshes broker snapshot

```ts
it('uses optimistic order feedback and does not auto-refresh broker snapshot after submit', async () => {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'execute_strategy_signal_once') {
      return {
        action: 'buy',
        order_submitted: true,
        order_id: 'ord_123',
        delta_qty: 1,
        warnings: [],
      }
    }
    throw new Error(`unexpected command ${command}`)
  })

  const result = await runOnce({
    ...baseOptions(),
    dryRun: false,
  })

  expect(invokeCommandMock).toHaveBeenCalledTimes(1)
  expect(result.statusLine).toContain('Submitted BUY order')
  expect(result.orderCount).toBe(1)
  expect(result.orderRows?.[0]?.cells?.[6]).toBe('ord_123')
})
```

### Test: `clearKeys()` does not fake runtime reset

```ts
it('clears persisted execution keys without forcing live or runner state off', async () => {
  const removeItem = vi.fn()
  vi.stubGlobal('localStorage', {
    getItem: vi.fn(() => null),
    setItem: vi.fn(),
    removeItem,
  })

  const result = await clearKeys()

  expect(removeItem).toHaveBeenCalledWith(EXEC_KEYS_STORAGE_KEY)
  expect(result.statusLine).toContain('Cleared execution API keys')
  expect(result.liveActive).toBeUndefined()
  expect(result.runnerActive).toBeUndefined()
  expect(result.liveStatusHint).toBeUndefined()
  expect(result.runnerStatusHint).toBeUndefined()
})
```

## 6.2 `web/src/services/runControlService.test.ts`

Because cancellation becomes partially parallel, remove strict nth-call ordering assertions and assert call presence instead.

### Test: primary + background cancel

```ts
it('requests primary and background cancellation when available', async () => {
  invokeCommandMock.mockResolvedValue(undefined)

  const result = await requestCancelCurrentRun()

  const commands = invokeCommandMock.mock.calls.map(([command]) => command)
  expect(commands).toContain('cancel_current_run')
  expect(commands).toContain('cancel_background_task')
  expect(result).toEqual({
    requested: true,
    usedLegacyFallback: false,
    backgroundTaskRequested: true,
  })
})
```

### Test: legacy fallback still works

```ts
it('falls back to legacy cancellation when current-run cancellation fails', async () => {
  invokeCommandMock.mockImplementation(async (command: string) => {
    if (command === 'cancel_current_run') {
      throw new Error('missing')
    }
    return undefined
  })

  const result = await requestCancelCurrentRun()

  const commands = invokeCommandMock.mock.calls.map(([command]) => command)
  expect(commands).toContain('cancel_current_run')
  expect(commands).toContain('cancel_double_ma_optimization')
  expect(commands).toContain('cancel_background_task')
  expect(result).toEqual({
    requested: true,
    usedLegacyFallback: true,
    backgroundTaskRequested: true,
  })
})
```

### Test: total failure still throws

```ts
it('throws when no cancellation command can be requested', async () => {
  invokeCommandMock.mockRejectedValue(new Error('missing'))

  await expect(requestCancelCurrentRun()).rejects.toThrow(
    'Cancellation command is unavailable.',
  )
})
```

## 6.3 New recommended test file: `web/src/stores/executionStore.test.ts`

This file is not in the provided bundle, but it is the right place to test the new store behavior.

Add tests for:

- `stopRunner()` remains callable while `runOnce()` is pending
- `refreshStatuses()` remains callable while another action is pending
- `applyPolygonLiveStatusEvent()` toggles `liveActive` correctly on `started` / `closed`
- `applyLiveExecutionRunnerEvent()` prepends an optimistic row for `order_submitted`

## 6.4 Rust-side helper tests in `commands_execution.rs`

The current file is hard to integration-test directly because the live path depends on Tauri state and app handles. The best low-risk move is to extract pure helper logic for:

- pending-bar replacement
- duplicate/older-bar rejection
- cancellation error detection

Example helper shape:

```rust
fn update_pending_bar(
    current: &mut Option<LiveExecutionRunnerBar>,
    incoming: &LiveExecutionRunnerBar,
) {
    match current.as_ref() {
        Some(existing) if existing.time_open_unix_ms >= incoming.time_open_unix_ms => {}
        _ => *current = Some(incoming.clone()),
    }
}
```

Then add tests:

```rust
#[test]
fn pending_bar_keeps_latest_bar() {
    let mut pending = None;
    update_pending_bar(
        &mut pending,
        &LiveExecutionRunnerBar {
            symbol: "AAPL".to_string(),
            time_open_unix_ms: 1000,
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: None,
        },
    );
    update_pending_bar(
        &mut pending,
        &LiveExecutionRunnerBar {
            symbol: "AAPL".to_string(),
            time_open_unix_ms: 2000,
            open: 2.0,
            high: 2.0,
            low: 2.0,
            close: 2.0,
            volume: None,
        },
    );
    assert_eq!(pending.as_ref().map(|b| b.time_open_unix_ms), Some(2000));
}
```

---

## 7. Suggested improvement plan

## Phase 1 — backend correctness first

**Files**
- `src-tauri/src/commands_execution.rs`

**Do now**
1. add runner generation + cancel handle
2. cancel prior runner on replace
3. cancel current runner before stop/take
4. add cancellation checks in blocking execution
5. change completion/update logic to generation-safe matching
6. add pending latest-bar coalescing

**Why first**
This removes the highest-risk correctness issue and prevents stale tasks from acting after stop/restart.

---

## Phase 2 — reduce avoidable broker latency

**Files**
- `src-tauri/src/commands_execution.rs`
- `web/src/services/executionService.ts`

**Do now**
1. add shared timed Alpaca client
2. extend snapshot request with include flags
3. skip account fetch in `refreshOrders()`
4. use single-symbol position fetch when possible

**Why second**
This is low-risk and immediately improves the most common broker refresh path.

---

## Phase 3 — make frontend controls responsive

**Files**
- `web/src/stores/executionStore.ts`

**Do now**
1. replace global busy lock with per-action pending flags
2. keep stop/status actions available while other actions are in flight
3. recompute availability from pending flags + runtime state

**Why third**
This fixes the most obvious user-facing control lag.

---

## Phase 4 — wire runtime events into store state

**Files**
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- optionally `web/src/tauri/events.ts`
- `src-tauri/src/commands_data.rs`

**Do now**
1. add runtime event subscription helper
2. subscribe to `polygon_live_status`
3. subscribe to `live_execution_runner_event`
4. emit explicit `started` / `stopped` live status events
5. update store hints/liveActive/runnerActive from runtime events

**Why fourth**
After Phases 1–3, this is the biggest “feel faster” improvement because the UI stops depending on manual refresh to stay truthful.

---

## Phase 5 — tighten service behavior

**Files**
- `web/src/services/executionService.ts`
- `web/src/services/runControlService.ts`

**Do now**
1. reject `startRunner()` when live stream symbol does not match
2. remove immediate broker snapshot from `runOnce()`
3. fix `clearKeys()` so it does not fake runtime shutdown
4. overlap background cancellation with primary cancellation request

---

## Phase 6 — update tests

**Files**
- `web/src/services/executionService.test.ts`
- `web/src/services/runControlService.test.ts`
- new `web/src/stores/executionStore.test.ts`
- `src-tauri/src/commands_execution.rs` helper tests

**Do now**
1. update existing tests for new snapshot flags and optimistic submit behavior
2. update cancellation tests for non-ordered concurrent probing
3. add store tests for pending flags and runtime event handling
4. add pure helper tests for pending-bar replacement logic

---

## 8. File-by-file edit summary

## `src-tauri/src/commands_execution.rs`

### Required
- shared Alpaca client with timeouts
- runner generation + cancel handle
- cancel-before-stop and cancel-before-replace
- cancellation checks in blocking execution
- pending latest-bar coalescing
- generation-safe completion updates
- snapshot include flags
- single-symbol position fetch

### Optional
- tiny helper extraction for unit-testable pending-bar logic
- temporary timing logs during rollout

---

## `src-tauri/src/commands_data.rs`

### Required
- emit `started` status event on live stream start
- emit `stopped` status event on live stream stop

### Verify in adjacent file
- confirm `state.rs` clears finished websocket tasks correctly

### Optional
- bounded reconnect/backoff loop after the first-pass fixes are stable

---

## `src-tauri/src/polygon.rs`

### Required for section 15
- none

### Recommended note
This file already demonstrates the right HTTP posture:
- explicit builder
- timeout
- retry/backoff
- `Retry-After` support

Mirror that approach in `commands_execution.rs`. Do not spend section-15 time rewriting this file unless you want a shared Polygon client helper for consistency.

---

## `web/src/services/executionService.ts`

### Required
- parse `stream` metadata from live status
- verify live symbol before starting runner
- send slim snapshot flags in `refreshOrders()`
- add runtime event subscription helper
- remove fake runtime reset from `clearKeys()`
- remove immediate broker snapshot from `runOnce()`
- optionally prepend optimistic order row

---

## `web/src/services/runControlService.ts`

### Required
- overlap background cancellation probe with primary/legacy cancel path

### Optional
- add invoke timeout wrapper if you observe hung invokes in practice

---

## `web/src/stores/executionStore.ts`

### Required
- add per-action pending flags
- replace global busy gate
- keep stop/status responsive
- subscribe to runtime events
- update hints and activity flags from runtime events

### Optional
- add explicit `destroyRuntimeSubscriptions()` if your app unmounts/remounts the workspace frequently

---

## `web/src/tauri/events.ts`

### Required
- none, if you keep subscription aggregation inside `executionService.ts`

### Optional
- add `listenEvents(...)` bulk helper for cleaner multi-event teardown

---

## `web/src/services/executionService.test.ts`

### Required
- new test for slim snapshot request
- new test for mismatched live-stream symbol
- update `runOnce()` test for optimistic path
- update `clearKeys()` test for truthful runtime state

---

## `web/src/services/runControlService.test.ts`

### Required
- remove nth-call ordering assumptions
- assert presence of background + primary/legacy calls instead

---

## 9. Comprehensive implementation checklist

Use this checklist as the execution order.

### Backend runner correctness

- [ ] Add `RunnerAbortHandle` to `src-tauri/src/commands_execution.rs`
- [ ] Add `LIVE_EXECUTION_RUNNER_GEN` atomic
- [ ] Extend `LiveExecutionRunnerRuntime` with `generation`, `abort`, and `pending_bar`
- [ ] Cancel the prior runtime before replacing it in `start_live_execution_runner(...)`
- [ ] Cancel the current runtime before removing it in `stop_live_execution_runner(...)`
- [ ] Update `execute_strategy_signal_blocking(...)` to accept optional abort handle
- [ ] Add cancellation checks before/after expensive execution stages
- [ ] Update one-shot execution path to pass `None` for abort
- [ ] Update live-runner execution path to pass the abort handle into `spawn_blocking`
- [ ] Replace symbol-only runtime mutation checks with generation-safe checks
- [ ] Emit `cancelled` events instead of generic `error` when stop wins before submit
- [ ] Add one-slot latest-bar coalescing instead of hard-dropping all busy bars
- [ ] Trigger the pending latest bar after current completion when still valid

### Backend broker latency

- [ ] Add `alpaca_client()` shared helper using `OnceLock`
- [ ] Add connect timeout
- [ ] Add total request timeout
- [ ] Replace all `Client::new()` call sites with `alpaca_client()?`
- [ ] Extend `GetLiveExecutionSnapshotRequest` with `include_account`
- [ ] Extend `GetLiveExecutionSnapshotRequest` with `include_orders`
- [ ] Extend `GetLiveExecutionSnapshotRequest` with `include_position`
- [ ] Add `fetch_alpaca_position(...)` single-symbol helper
- [ ] Skip account fetch when `include_account=false`
- [ ] Skip orders fetch when `include_orders=false`
- [ ] Skip position fetch when `include_position=false`
- [ ] Use single-symbol position path when `symbol` is provided
- [ ] Keep current response shape backward-compatible where possible

### Polygon stream lifecycle

- [ ] Emit a `started` live-status event from `polygon_start_live_candle_stream(...)`
- [ ] Emit a `stopped` live-status event from `polygon_stop_live_candle_stream(...)`
- [ ] Verify whether `state.rs` clears finished websocket tasks after unexpected disconnects
- [ ] If needed, add adjacent state cleanup so `polygon_live_candle_stream_status()` cannot lie after task exit
- [ ] Defer reconnect/backoff until after first-pass fixes are measured
- [ ] If reconnect/backoff is needed, keep it bounded and observable via live-status events

### Frontend execution service

- [ ] Expand `LiveCandleStreamStatusResponse` typing to include `stream`
- [ ] Add `parseLiveStreamStatus(...)`
- [ ] Update `refreshLiveStatusHint()` to show symbol and delayed/live mode
- [ ] Reject `startRunner()` when live stream symbol differs from runner symbol
- [ ] Update `refreshOrders()` invoke payload to request `include_account=false`
- [ ] Add `delta_qty` to `ExecuteSignalResponse`
- [ ] Add `optimisticOrderRowFromExecution(...)`
- [ ] Remove immediate `refreshOrders()` call from `runOnce()` success path
- [ ] Keep manual Refresh Orders as the authoritative broker state action
- [ ] Add `subscribeExecutionRuntimeEvents(...)`
- [ ] Subscribe to `polygon_live_status`
- [ ] Subscribe to `live_execution_runner_event`
- [ ] Remove fake runtime reset from `clearKeys()`

### Frontend execution store

- [ ] Add `ExecutionPendingState`
- [ ] Add `pending` to store state
- [ ] Replace global busy lock with per-action pending flags
- [ ] Recompute `isBusy` from `pending`
- [ ] Keep `stopLive` available while primary actions are running
- [ ] Keep `stopRunner` available while primary actions are running
- [ ] Keep `refreshStatuses` available while primary actions are running
- [ ] Convert `beginAction(...)` and `finishAction(...)` to keyed versions
- [ ] Update all actions to use keyed pending flags
- [ ] Add `ensureRuntimeSubscriptions()` to the store
- [ ] Add `applyPolygonLiveStatusEvent(...)`
- [ ] Add `applyLiveExecutionRunnerEvent(...)`
- [ ] Update `liveActive` from live-status events
- [ ] Update `runnerActive` from runner events
- [ ] Update status hints from runtime events
- [ ] Optionally prepend optimistic order rows from runner `order_submitted` events
- [ ] Ensure `clearKeys()` only clears credentials, not runtime truth

### Runtime-control service

- [ ] Refactor `requestCancelCurrentRun()` to overlap background cancellation with the primary path
- [ ] Preserve legacy fallback to `cancel_double_ma_optimization`
- [ ] Preserve best-effort background cancellation semantics
- [ ] Optionally add a timeout wrapper only if invoke hangs are observed

### Tests

- [ ] Update `executionService.test.ts` to assert `include_account=false`
- [ ] Add `executionService.test.ts` coverage for mismatched live-stream symbol
- [ ] Update `executionService.test.ts` for optimistic `runOnce()` behavior
- [ ] Update `executionService.test.ts` so `clearKeys()` no longer implies live/runner stop
- [ ] Update `runControlService.test.ts` to stop depending on nth-call ordering
- [ ] Add `runControlService.test.ts` coverage for parallel background cancellation path
- [ ] Add new `executionStore.test.ts` for per-action pending behavior
- [ ] Add new `executionStore.test.ts` for runtime event handling
- [ ] Add helper-level Rust tests for pending latest-bar replacement
- [ ] Add helper-level Rust tests for cancellation marker handling

### Verification and rollout

- [ ] Measure baseline `testConnection()` latency
- [ ] Measure baseline `refreshOrders()` latency
- [ ] Measure baseline `runOnce()` submit latency
- [ ] Measure stop responsiveness before code changes
- [ ] Ship backend correctness changes first
- [ ] Re-measure stop/restart behavior
- [ ] Ship slim snapshot path and re-measure `refreshOrders()`
- [ ] Ship store pending flags and verify stop/status controls stay responsive
- [ ] Ship runtime event wiring and verify state updates without manual refresh
- [ ] Ship optimistic `runOnce()` UI and verify faster perceived completion
- [ ] Only after that, decide whether reconnect/backoff is still worth doing

### Deferred / optional

- [ ] Consider bounded websocket reconnect/backoff
- [ ] Consider live-only incremental strategy evaluation if per-bar latency remains too high after the first-pass fixes
- [ ] Consider an async broker HTTP rewrite only if timed blocking client reuse is still insufficient
