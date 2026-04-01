# Addendum to the section 15 implementation plan

This document supplements `live_execution_polygon_runtime_control_improvement_plan.md`.

It focuses on items that are either:

1. important enough that I would add them to the implementation plan now,
2. correctness/performance gaps that were not emphasized enough in the first pass, or
3. optional improvements that go a bit beyond the original section-15 prompt but still fit your stated priority: **performance, optimization, and correctness without removing features or breaking functionality**.

---

## 1. High-priority additions I would add now

## 1.1 Add an authoritative `symbol <-> data_id` guard

### Why this matters

The current execution request shape carries two independent identity anchors:

- `req.symbol` — the broker/live-stream symbol
- `strategy.data_id` — the loaded candle dataset used for signal computation

In the provided files, `execute_strategy_signal_once(...)` and `start_live_execution_runner(...)` validate that `data_id` exists, but they do **not** validate that the loaded dataset behind `data_id` is actually for the same symbol as `req.symbol`.

That means a user can accidentally:

- compute signals on dataset **MSFT**,
- submit orders for **AAPL**,
- or start a live AAPL runner that appends AAPL bars into a dataset that originally belongs to another symbol.

This is a **bigger correctness risk than any latency issue** in the provided files.

### Minimum recommended implementation

Add an authoritative backend guard in the adjacent loaded-dataset registry (`AppState` / loaded data metadata surface) and call it from both:

- `execute_strategy_signal_once(...)`
- `start_live_execution_runner(...)`

### Suggested backend shape

In an adjacent state file, expose something like:

```rust
pub fn dataset_symbol_for_data_id(&self, data_id: &str) -> Result<Option<String>, String>;
```

Then add a helper in `commands_execution.rs`:

```rust
fn canonical_symbol(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn ensure_strategy_symbol_matches_data(
    state: &AppState,
    data_id: &str,
    requested_symbol: &str,
) -> Result<(), String> {
    let Some(dataset_symbol) = state.dataset_symbol_for_data_id(data_id)? else {
        return Ok(());
    };

    if canonical_symbol(&dataset_symbol) != canonical_symbol(requested_symbol) {
        return Err(format!(
            "requested symbol '{}' does not match loaded dataset symbol '{}' for data_id='{}'",
            requested_symbol,
            dataset_symbol,
            data_id,
        ));
    }

    Ok(())
}
```

Then apply it in both command entrypoints:

```rust
#[tauri::command]
pub async fn execute_strategy_signal_once(
    req: ExecuteStrategySignalRequest,
    state: tauri::State<'_, AppState>,
) -> Result<ExecuteStrategySignalResponse, String> {
    let (req, symbol, qty) = normalize_execute_request(req)?;
    let data_id = strategy_data_id(&req.strategy).to_string();
    ensure_strategy_symbol_matches_data(state.inner(), &data_id, &symbol)?;
    let candles = state.get_candles(&data_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        execute_strategy_signal_blocking(req, symbol, qty, candles)
    })
    .await
    .map_err(|e| e.to_string())?
}
```

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

    ensure_strategy_symbol_matches_data(state.inner(), &data_id, &symbol)?;
    let _ = state.get_candles(&data_id)?;

    // ... rest of existing start logic ...
}
```

### Frontend preflight guard

If the currently loaded/optimized run context already exposes dataset symbol metadata in an adjacent file, add the same guard in `executionService.ts` so the user gets a fast rejection before Tauri invoke.

### Priority

**Do this in the same phase as stop-authority work.** It is a correctness guard, not a nice-to-have.

---

## 1.2 Fix symbol canonicalization collisions

### Why this matters

The current Rust helper:

```rust
fn normalize_symbol_key(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase()
}
```

is used in the live runner and snapshot symbol matching paths.

That strips punctuation and can collapse distinct valid stock symbols into the same key, for example:

- `BRK.B` -> `BRKB`
- `BRKB` -> `BRKB`

The same issue exists conceptually for other punctuation-bearing symbols.

### Recommended change

Replace the current “strip everything but alphanumeric” logic with a canonicalizer that:

- uppercases,
- trims,
- removes whitespace,
- but **preserves meaningful punctuation** like `.`, `-`, `_`, `/`, and `:`.

### Rust helper

Use one shared helper in both `commands_execution.rs` and `commands_data.rs`:

```rust
fn canonical_symbol(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn symbol_matches(a: &str, b: &str) -> bool {
    canonical_symbol(a) == canonical_symbol(b)
}
```

Then replace all uses of `normalize_symbol_key(...)` equality checks with `symbol_matches(...)`.

### TypeScript helper

Mirror the same logic in `executionService.ts` / `executionStore.ts`:

```ts
function canonicalSymbol(raw: unknown): string {
  return String(raw ?? '')
    .trim()
    .replace(/\s+/g, '')
    .toUpperCase()
}
```

### Priority

This should be treated as a **real correctness patch**, not just cleanup.

---

## 1.3 Couple runner lifecycle to live-stream replacement and manual stop

### Why this matters

The original plan already covers `startRunner()` accepting the wrong live-stream symbol.

The adjacent risk is still present after that fix:

- user starts live stream for `AAPL`
- user starts runner for `AAPL`
- user later replaces live stream with `MSFT` or manually stops the stream
- runner remains marked active but no longer has a matching live source

That creates silent inert runtime state.

### Recommended minimum behavior

For **manual** live-stream transitions, use strong coupling:

1. **manual stop of live stream** should also stop the runner
2. **replacing live stream with a different symbol** should also stop the runner

For **unexpected disconnects**, you can choose between:

- leaving the runner configured but visibly suspended, or
- auto-stopping it

The safer first implementation is:

- **manual stop / manual replacement** -> auto-stop runner
- **unexpected websocket error/closed** -> keep runner configured but surface a degraded/suspended status in events/store

### Implementation sketch for command path

In `polygon_stop_live_candle_stream(...)`, after stopping the live stream, stop the runner too:

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

        // Optional but recommended: manual live stop should stop the runner.
        let _ = super::commands_execution::stop_live_execution_runner(app.clone());
    }

    Ok(PolygonStopLiveCandleStreamResponse {
        stopped: prior.is_some(),
        prior,
    })
}
```

For stream replacement, compare the prior stream symbol with the new symbol. If they differ, stop the runner after replacement.

If you do not want the backend to enforce this automatically, then at minimum emit an explicit event/status saying:

- `runner inactive because live stream symbol changed`

But auto-stop on manual replacement is the cleaner correctness default.

---

## 1.4 Tighten store availability and track live symbol explicitly

### Why this matters

Even after the original per-action pending refactor, the store should not advertise “Start runner” when the prerequisites are visibly false.

Today, `executionStore.ts` computes availability from `liveActive` / `runnerActive` and busy state, but it does not model the live symbol directly.

### Recommended additions

Add store state:

```ts
liveSymbol: string
liveDelayed: boolean
```

Update them from:

- `startLive()` result
- `polygon_live_status` events
- `stopLive()` result

Then tighten availability:

```ts
state.canStartRunner =
  !primaryBusy &&
  !state.pending.startRunner &&
  !state.pending.stopRunner &&
  !state.runnerActive &&
  state.liveActive &&
  !!canonicalSymbol(state.liveSymbol) &&
  canonicalSymbol(state.liveSymbol) === canonicalSymbol(state.symbol)
```

This removes a whole class of “the button was available but the command rejected” drift.

### Recommended UX copy

When live is active but on the wrong symbol, show something like:

```ts
state.runnerStatusHint = `Auto-runner: waiting for matching live stream (${state.symbol || '-'})`
```

This is not a new feature. It is a state-truth improvement.

---

## 1.5 Abort the spawned websocket task if registration into state fails

### Why this matters

`polygon_start_live_candle_stream(...)` spawns the websocket task **before** it registers the task into state.

If `state.replace_live_candle_stream(...)` fails after the spawn succeeds, the command returns an error but the websocket task is already running.

That leaves an orphan runtime task.

### Recommended change

Abort the spawned task on registration failure.

Because `JoinHandle` is moved into `LiveCandleStreamTask` before the current `replace_live_candle_stream(...)` call returns, the clean implementation needs **one small adjacent API change** in the state layer.

### Safe implementation options

#### Option A — make the state API return the task back on failure

Change the adjacent state method so ownership is returned on error:

```rust
pub fn replace_live_candle_stream(
    &self,
    task: LiveCandleStreamTask,
) -> Result<Option<LiveCandleStreamInfo>, (String, LiveCandleStreamTask)>;
```

Then the command code becomes:

```rust
let handle = tauri::async_runtime::spawn(async move {
    run_polygon_live_candle_stream(app2, ws_url2, api_key, market2, symbol2, delayed, channel2)
        .await;
});

let task = LiveCandleStreamTask {
    handle,
    market: market.clone(),
    symbol: symbol.clone(),
    delayed,
};

let replaced_prior = match state.replace_live_candle_stream(task) {
    Ok(v) => v,
    Err((e, task)) => {
        task.handle.abort();
        return Err(e);
    }
};
```

#### Option B — wrap the handle in an abort-on-drop helper

If you do not want to change the state API signature, wrap the join handle in a small guard type that aborts on `Drop` unless it has been explicitly disarmed after successful registration.

That way, if `replace_live_candle_stream(...)` returns an error and the temporary task is dropped, the background task is cancelled rather than detached.

### Why I prefer Option A

It is explicit, easy to test, and avoids introducing extra wrapper behavior around join handles.
### Priority

Medium. It is a small safety fix.

---

## 1.6 Add websocket connect/auth/subscribe timeouts

### Why this matters

The original plan already adds timeouts for Alpaca HTTP calls.

The Polygon websocket path still has no explicit timeout around:

- `connect_async(...)`
- auth send
- subscribe send

A slow or half-broken network can leave the task in a long “connecting” state.

### Recommended change

Wrap those stages in `tokio::time::timeout(...)`.

### Example

```rust
use tokio::time::{timeout, Duration};

async fn run_polygon_live_candle_stream(
    app: tauri::AppHandle,
    ws_url: String,
    api_key: String,
    market: String,
    symbol: String,
    delayed: bool,
    channel: String,
) {
    emit_polygon_live_status(
        &app,
        &market,
        &symbol,
        delayed,
        &channel,
        "connecting",
        format!("connecting to {ws_url}"),
    );

    let ws = match timeout(Duration::from_secs(10), connect_async(ws_url.as_str())).await {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            emit_polygon_live_status(
                &app,
                &market,
                &symbol,
                delayed,
                &channel,
                "error",
                format!("connect failed: {e}"),
            );
            return;
        }
        Err(_) => {
            emit_polygon_live_status(
                &app,
                &market,
                &symbol,
                delayed,
                &channel,
                "error",
                "connect timed out",
            );
            return;
        }
    };

    let (mut stream, _) = ws;

    let auth_msg = serde_json::json!({
        "action": "auth",
        "params": api_key,
    })
    .to_string();

    if timeout(Duration::from_secs(5), stream.send(Message::Text(auth_msg.into())))
        .await
        .is_err()
    {
        emit_polygon_live_status(&app, &market, &symbol, delayed, &channel, "error", "auth send timed out");
        return;
    }

    let sub_msg = serde_json::json!({
        "action": "subscribe",
        "params": channel,
    })
    .to_string();

    if timeout(Duration::from_secs(5), stream.send(Message::Text(sub_msg.into())))
        .await
        .is_err()
    {
        emit_polygon_live_status(&app, &market, &symbol, delayed, &channel, "error", "subscribe send timed out");
        return;
    }

    // ... existing read loop ...
}
```

### Priority

Medium. It complements the HTTP timeout work nicely.

---

## 1.7 Tighten symbol-filtered order refresh correctness

### Why this matters

The original plan already recommends slimming broker snapshots.

There is one more subtle correctness issue in the current snapshot behavior:

- orders are fetched with a global limit,
- then filtered client-side by symbol.

That means if the account has heavy order activity across other symbols, the requested symbol’s older rows can be truncated **before** filtering happens.

### Minimum safe change

If the broker API still cannot filter by symbol server-side in your current implementation path, then at minimum:

- raise the scan limit from `200` to `500` when `symbol` is present, and/or
- separate `scan_limit` from `return_limit`, and
- stop describing the result as “all recent orders for symbol X” when it is only “symbol-filtered subset of first N global orders”.

### Better change

If you keep the current endpoint shape, add iterative scan logic until one of these is true:

- enough matching symbol rows were found,
- API page budget is exhausted,
- or hard scan cap is reached.

That is a correctness improvement first and a performance optimization second.

---

## 2. Optional improvements that go a bit beyond the original prompt

These are **not** required for the first safe pass, but they are good follow-up candidates if you want to keep improving responsiveness without changing the feature set.

## 2.1 Add a combined runtime-status command

### Why

`refreshStatuses()` currently issues two separate invokes:

- `live_execution_runner_status`
- `polygon_live_candle_stream_status`

That is already acceptable, but a single backend command would:

- reduce Tauri round-trips,
- reduce state skew between two snapshots taken at slightly different moments,
- simplify frontend refresh logic.

### Suggested shape

```rust
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionRuntimeStatusResponse {
    pub runner: LiveExecutionRunnerStatus,
    pub live: PolygonLiveCandleStreamStatusResponse,
    pub fetched_at_unix_ms: i64,
}
```

Expose:

```rust
#[tauri::command]
pub fn execution_runtime_status(
    state: tauri::State<'_, AppState>,
) -> Result<ExecutionRuntimeStatusResponse, String>
```

This is not essential once event wiring is in place, but it is a clean low-risk improvement.

---

## 2.2 Merge `upsert_live_bar(...)` and `get_candles(...)` if the adjacent state layer makes that easy

### Why

The live-bar path currently does:

1. `upsert_live_bar(...)`
2. `get_candles(...)`
3. recompute strategy

If the adjacent `AppState` layer can cheaply return the updated candle handle from the mutation path, you can remove one state lookup and one lock boundary from every live bar.

### Example adjacent API shape

```rust
pub fn upsert_live_bar_and_get_candles(
    &self,
    data_id: &str,
    ts: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<f64>,
) -> Result<Arc<Candles>, String>;
```

### Priority

Optional. Nice if the state internals already make it easy.

---

## 2.3 Add broker idempotency via `client_order_id`

### Why

This goes slightly beyond the original prompt, but it is one of the best correctness hardening changes you can make for live execution.

If a submit path is retried, times out ambiguously, or a restart occurs near the submit boundary, deterministic `client_order_id` values reduce duplicate-order risk.

### Suggested shape

Derive `client_order_id` from stable inputs such as:

- symbol
- mode
- runner generation or one-shot marker
- `last_bar_ts`
- desired action

Example:

```rust
let client_order_id = format!(
    "vg_{}_{}_{}_{}",
    symbol,
    match req.mode { LiveExecutionMode::Paper => "paper", LiveExecutionMode::Live => "live" },
    decision.last_bar_ts.unwrap_or(0),
    if delta_qty > 0.0 { "buy" } else { "sell" },
);
```

Then add it to the payload object.

### Priority

Optional but highly recommended if you intend to rely on live execution more heavily.

---

## 2.4 Distinguish permanent runner errors from transient ones

### Why

Today, repeated errors can leave the runner active but effectively non-functional.

Examples:

- bad broker credentials
- missing dataset for `data_id`
- repeated live/data mismatch

### Recommended second-pass improvement

Classify a few errors as “terminal until user intervention” and either:

- auto-stop the runner after N consecutive terminal errors, or
- keep it configured but emit a `suspended` event/status and disable new triggers

### Example fields

Optional additions to `LiveExecutionRunnerStatus`:

```rust
pub consecutive_error_count: u64,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub health: Option<String>, // "active" | "suspended" | "stopped"
```

This is beyond the original plan, but it helps keep runtime behavior truthful.

---

## 2.5 Only mark live stream “active” after stronger evidence if you want stricter truthfulness

### Why

The current start path returns successfully once the task is launched, not once the websocket is authenticated and subscribed.

That is acceptable if “active” means “task exists”, but not if “active” means “usable market data is flowing”.

### Optional refinement

If you want stricter truth, split the notion into:

- `requested`
- `connected`
- `subscribed`
- `streaming`

You do **not** need a new UI panel for this. Even a single internal enum or better store hinting is enough.

### Priority

Optional. Only do this if runtime truthfulness is still confusing after event wiring.

---

## 3. What I do *not* think the original plan materially missed

The original plan already covered the most important first-order work:

- authoritative runner stop via generation/cancel logic
- shared timeout-configured Alpaca client
- slimmer broker snapshot paths
- event wiring into the store
- per-action pending state instead of one global busy lock
- latest-bar coalescing instead of unconditional busy-drop behavior
- stronger tests around runner stop, stream status, and store responsiveness

So I would not rewrite the whole original plan.

I would **augment** it with the additions above, especially:

1. `symbol <-> data_id` guard
2. canonical symbol fix
3. runner/live coupling on manual stop or replacement
4. store availability gating on live symbol

Those are the places where the original plan still had real room to improve.

---

## 4. Updated implementation order

If you want the cleanest rollout, I would now use this order:

1. **Runner correctness + dataset/symbol correctness together**
   - generation/cancel token
   - `symbol <-> data_id` guard
   - canonical symbol helper

2. **Live-stream lifecycle truthfulness**
   - started/stopped status events
   - manual stop/replacement -> runner stop
   - abort spawned websocket task on registration failure
   - websocket stage timeouts

3. **Frontend state truthfulness**
   - event wiring
   - live symbol tracking
   - `canStartRunner` depends on matching live symbol
   - `clearKeys()` no longer lies about runtime state

4. **Broker latency tightening**
   - shared timed client
   - slim snapshot path
   - better symbol-filtered order refresh behavior
   - remove immediate post-submit full refresh from `runOnce()`

5. **Optional second pass**
   - combined runtime status command
   - `upsert_live_bar_and_get_candles(...)`
   - `client_order_id`
   - terminal error classification / suspended runner mode

---

## 5. Supplemental implementation checklist

- [ ] Add backend `symbol <-> data_id` validation for `execute_strategy_signal_once(...)`
- [ ] Add backend `symbol <-> data_id` validation for `start_live_execution_runner(...)`
- [ ] Add or expose dataset-symbol metadata from the adjacent loaded-data registry if it is not already available
- [ ] Replace punctuation-stripping symbol matching with whitespace-trimming uppercase canonical matching
- [ ] Reuse the same canonical symbol logic across `commands_execution.rs`, `commands_data.rs`, and frontend execution helpers
- [ ] Add coverage for punctuation-bearing symbols such as `BRK.B`
- [ ] Auto-stop the runner on **manual** live-stream stop
- [ ] Auto-stop the runner when live stream is manually replaced with a different symbol
- [ ] Decide whether unexpected websocket disconnect should auto-stop runner or mark it suspended
- [ ] Add `liveSymbol` state to `executionStore.ts`
- [ ] Add `liveDelayed` state to `executionStore.ts`
- [ ] Tighten `canStartRunner` so it requires `liveActive === true`
- [ ] Tighten `canStartRunner` so it requires `liveSymbol === requested symbol`
- [ ] Abort the spawned websocket task if `replace_live_candle_stream(...)` fails after spawn
- [ ] Add websocket connect timeout
- [ ] Add websocket auth-send timeout
- [ ] Add websocket subscribe-send timeout
- [ ] Tighten symbol-filtered order refresh so client-side filtering cannot silently hide older symbol rows as easily
- [ ] If needed, separate `scan_limit` from `return_limit` for broker order refresh
- [ ] Optionally add a combined `execution_runtime_status` command
- [ ] Optionally merge `upsert_live_bar(...)` + `get_candles(...)` into one adjacent state call
- [ ] Optionally add deterministic `client_order_id` for broker submissions
- [ ] Optionally classify terminal runner errors and auto-stop or suspend after repeated failures
- [ ] Add tests for dataset-symbol mismatch rejection
- [ ] Add tests for punctuation-preserving symbol matching
- [ ] Add tests for auto-stop runner on manual live stop/replacement
- [ ] Add tests for `canStartRunner` availability requiring matching live symbol
- [ ] Add tests for websocket-start registration failure cleanup
- [ ] Add tests for websocket timeout status emission

