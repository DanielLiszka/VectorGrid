# Fourth-Pass Review: src-tauri/src/commands_execution.rs

- Review date: `2026-04-15`
- Source file: `src-tauri/src/commands_execution.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_execution.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_execution.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_execution.rs.review.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Safe-now hardening, with larger redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third pass correctly identified the `status=all` blind spot and correctly kept the true stop/lifecycle redesign deferred. It still left two important questions insufficiently sharpened: whether the current bucket was too defer-heavy for the next step, and whether an “open-only” query patch could be implemented naively and regress the operator-facing snapshot surface.

After re-reading the live file, the answer is yes on both counts. The file still has deferred architectural debt, but there is now a bounded hardening set with clear safety upside and limited blast radius. That changes the recommended next action even though it does **not** eliminate the need for the larger redesign later.

## Fourth-Pass Research Log

1. **Alpaca API Reference — Get All Orders** (`docs.alpaca.markets/reference/getallorders-1`)
   - Confirmed that `status` defaults to `open`, `limit` maxes at `500`, and `symbols` is a supported filter.
   - This matters because the code currently hardcodes `status=all` and omits `symbols` in `fetch_alpaca_orders` (`commands_execution.rs:1209-1239`).

2. **Alpaca Broker API FAQ** (`docs.alpaca.markets/docs/broker-api-faq`)
   - Confirmed the statuses Alpaca currently treats as open for cancellation: accepted, new, partially filled, calculated, pending new, pending cancel, and accepted for bidding.
   - This matters because `is_broker_open_order_status` (`commands_execution.rs:1349-1363`) omits `pending_cancel` and uses one predicate for both cancelability and “still blocks new action”.

3. **Alpaca docs — Working with /orders, Websocket Streaming, and Placing Orders** (`docs.alpaca.markets/docs/working-with-orders`, `docs.alpaca.markets/docs/websocket-streaming`, `docs.alpaca.markets/docs/orders-at-alpaca`)
   - Reconfirmed that `client_order_id` is a first-class correlation field and that `trade_updates` is the recommended way to maintain order state.
   - This keeps the larger redesign direction unchanged: order-lifecycle-aware control is still the long-term fix, not more position polling.

4. **Tokio `spawn_blocking` docs** (`docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html`)
   - Reconfirmed that `spawn_blocking` work cannot be aborted once it has started running.
   - This continues to justify deferring any claim that runner stop is a true broker-submission barrier.

5. **Alpaca Fractional Trading and Close a Position** (`docs.alpaca.markets/docs/fractional-trading`, `docs.alpaca.markets/reference/deleteopenposition-1`)
   - Confirmed that current Alpaca docs still allow up to `9` decimal places for fractional `qty` and close-position `qty`.
   - This is a secondary compatibility smell against the file’s 4-decimal normalization (`commands_execution.rs:816-832`), but it is not the first hardening target.

6. **Tauri command/state docs** (`v2.tauri.app/develop/calling-rust`, `v2.tauri.app/develop/state-management`)
   - Reconfirmed that this file is a public command/state boundary and should preserve response shapes unless a correctness issue forces change.
   - That supports using existing `warnings` fields where possible instead of churning IPC contracts.

## What Fourth Pass Sharpened

The current third-pass bucket should change. The file still contains the larger redesign work already identified, but the next action is no longer “defer unless doing a larger refactor.” There is enough bounded, internal hardening available now that the immediate bucket should be safe-now hardening, with the redesign explicitly left deferred.

The third-pass packet got several important things right. It correctly identified the one-page `status=all` blind spot in `fetch_alpaca_orders` (`commands_execution.rs:1209-1239`). It was also right to avoid per-bar REST amplification, and it was right to keep true order-lifecycle tracking, `client_order_id`, and a real stop/disarm barrier in the deferred bucket. None of that changes here.

What it missed is that the next bounded fix is more specific than “use an open-order query.” The file’s local status classifier is already wrong for safety work. `is_broker_open_order_status` omits `pending_cancel` entirely while including `pending_replace`, `held`, `stopped`, and `suspended` in a single catch-all predicate (`commands_execution.rs:1349-1363`). That is not just naming debt. `cancel_live_execution_open_orders`, `flatten_live_execution_position`, and snapshot open-order counting all flow through that one predicate (`commands_execution.rs:1365-1372`, `2317-2321`, `2344-2463`). So a naive third-pass implementation that only changes the REST query to `status=open` would still undercount `pending_cancel` and could still let flattening proceed while an order remains broker-open.

The safer bounded fix is to split this into at least two internal concepts: a **cancel-candidate** status set aligned to Alpaca’s documented open-for-cancel statuses, and a broader **still blocks new action** set that can conservatively include states like `pending_replace`. That is the concrete step the third-pass packet underweighted.

The third-pass packet also underweighted how this should be implemented without regressing the snapshot/history surface. `get_live_execution_snapshot` intentionally returns recent order rows for review, not just open orders (`commands_execution.rs:2202-2291`). If someone follows the third-pass advice by simply changing shared `fetch_alpaca_orders` to `status=open`, they would silently regress that operator-facing history view. The safe change is additive: keep the current broad history helper for history rows, and add a separate symbol-scoped open-order helper for safety actions and exact open-order counting.

That is also why the third-pass emphasis on immediate pagination was a little too strong. The first bounded gain is `status=open` **plus** `symbols=<symbol>`, with pagination only needed after that for pathological `>500` same-symbol open-order cases. The official Get All Orders docs already expose both fields, so the lower-regression first step is smaller than the third-pass packet made it sound.

A second missed issue is that snapshot counting is currently lossy even before the query bug is fixed. `get_live_execution_snapshot` truncates the returned `orders` vector to `return_limit` (`commands_execution.rs:2233-2247`), then computes `open_order_count` from that already-truncated list (`commands_execution.rs:2317-2321`). That means the broker snapshot artifact can report a smaller open-order count simply because the UI asked for fewer displayed rows. The response DTO does not currently expose that count, so this is mostly an audit-integrity issue today, but it is still a real undercount the third-pass packet did not call out.

The third-pass packet also underweighted how broad the audit-consistency problem is. It correctly called out `execute_strategy_signal_once` returning `Err` after a successful broker submission if live-session capture fails (`commands_execution.rs:1682-1697`, `745-806`). But the deeper persistence issue is wider: `finalize_active_live_session` clears the in-memory slot **before** it attempts blocking finalize I/O (`commands_execution.rs:701-715`), and multiple callers ignore the result (`commands_execution.rs:1727`, `1825-1827`, `2528`). A transient finalize failure therefore loses retryability and can strand an artifact in the wrong session state. That is a safe-now persistence hardening target because it does not need a new public API: keep the slot until finalize succeeds, or at minimum preserve the path for retry and log the failure.

There is also a smaller but real compatibility smell that should stay out of the immediate patch set but should now be documented. The command boundary normalizes `qty` to 4 decimal places (`commands_execution.rs:816-832`) even though Alpaca’s current fractional-share docs still allow up to 9 decimals, and the symbol helper deliberately preserves `/` punctuation in tests (`commands_execution.rs:2929-2934`) while the position helpers interpolate the symbol directly into a path segment (`commands_execution.rs:1108`, `1281`). That is evidence that the public DTO shape looks broader than the actual implementation. Because repo guidance still makes this live surface effectively stock-focused, this is better treated as deferred compatibility cleanup or explicit validation, not mixed into the first safety patch.

## Implementation Priority

The safe-now hardening set is now clear.

First, add a dedicated open-order retrieval path for symbol-specific safety work. It should use Alpaca’s open-order semantics rather than the current `status=all` history query, and it should use the broker’s symbol filter rather than relying on a 500-row client-side scan. Do **not** repurpose the existing shared history helper. The internal shape here should be something like “fetch symbol open orders” rather than “change fetch orders globally.”

Second, split the current order-status predicate. One predicate should answer “can/should this order still block flattening or new submissions?” and must include at least `pending_cancel`. A second predicate should answer “is this order a valid cancel candidate?” and should stay aligned with Alpaca’s documented open-for-cancel set. This keeps flatten protection conservative without overclaiming cancelability.

Third, fix result/audit coherence without changing public DTOs. `ExecuteStrategySignalResponse` already has a `warnings` field (`commands_execution.rs:255-273`), so post-submit audit failures can be surfaced there instead of being promoted to a command error after the broker action already succeeded. In the same patch family, `start_live_execution_runner` should not leave the runner armed if artifact creation fails (`commands_execution.rs:1756-1777`), and live-session finalization should not destroy the only retry handle before finalize succeeds.

Fourth, add the missing focused tests. The current test module is helpful, but it does not cover the highest-value bounded fixes: `pending_cancel` handling, exact open-order counting from non-truncated data, runner-start rollback on artifact failure, and non-destructive live-session finalize failure behavior.

The larger redesign should still remain deferred. That includes a true pre-submit disarm barrier that survives `spawn_blocking`, client-owned order intent via `client_order_id`, and an order-state-aware runner driven by `trade_updates` rather than by filled-position polling alone. Those are still the correct long-term direction, but they are not required to take the bounded hardening step above.

## Verification If Changed

At minimum, keep the current module test lane and add targeted cases around the new bounded helpers:

1. `cargo test -p ta_desktop_demo_app commands_execution -- --nocapture`
2. Add unit tests for:
   - `pending_cancel` being treated as broker-open for flatten/blocking logic
   - cancel-candidate status classification remaining separate from broader blocking logic
   - symbol-scoped open-order query construction using open-only broker semantics
   - snapshot open-order counting being computed from full filtered data rather than the truncated display slice
   - runner-start rollback when live-session artifact creation fails
   - finalize failure preserving enough state to retry or at least to log deterministically
   - manual execution returning success plus warning when broker submission succeeded but audit capture failed
3. `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`

If safety-action semantics or order-submission result handling changes, do a paper-account smoke test for cancel-open-orders, flatten-position, and manual one-shot execution. No GPU perf gate or validation gate is needed unless the change leaks into app-core strategy execution, which this patch set should avoid.

## Regression And Feature-Preservation Constraints

Do not change `LIVE_EXECUTION_RUNNER_EVENT` status strings or `LiveStrategySpec` labels in this patch. The safe-now hardening described here does not require any frontend contract churn.

Do not “fix” the third-pass query issue by changing shared `fetch_alpaca_orders` to open-only semantics globally. `get_live_execution_snapshot` uses that helper for recent-history display, and collapsing it to open-only would be a user-visible regression.

If adding `symbols` or other new query parameters, use reqwest query-building rather than ad hoc string concatenation. The file already preserves punctuation such as `.` and `/` in symbol helpers (`commands_execution.rs:2929-2934`), so the implementation should not create a new encoding bug while fixing the safety query.

Do not add per-bar open-order REST polling to the live runner. The third-pass packet was right to reject that path on both correctness and latency grounds.

Preserve the current explicit reqwest timeout posture and keep these changes runtime-local. There is no GPU ABI, optimizer, or strategy-validation surface that should move as part of this hardening.

Finally, do not accidentally widen asset-class support while touching symbol or order-query code. The current command DTOs look broader than the real implementation, but this patch should harden the stock-focused live surface, not quietly broaden it.

## Fourth-Pass Conclusion

The third-pass packet was directionally right about the architectural risks, but its bucket was too defer-heavy for the next step. The current third-pass bucket should change.

The file still has larger redesign work that should remain deferred: true stop/disarm fencing around `spawn_blocking`, order-lifecycle-aware control via `trade_updates`, and explicit client-owned order correlation. But there is now a clear, bounded hardening set that can land safely first: add a dedicated symbol-open query instead of relying on a one-page `status=all` scan, split cancelability from broader blocking status classification, compute open-order counts from non-truncated data, and make audit/result handling coherent without changing the public IPC surface.

So the right fourth-pass posture is: land safe-now hardening in this file, and keep the larger broker-lifecycle redesign explicitly deferred.
