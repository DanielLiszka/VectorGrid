# Final Consolidated Review: `src-tauri/src/commands_execution.rs`

- Review date: `2026-04-16`
- Source file: [commands_execution.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_execution.rs:1>)
- First-pass packet: [commands_execution.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_execution.rs.review.md:1>)
- Second-pass packet: [commands_execution.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_execution.rs.review.md:1>)
- Third-pass packet: [commands_execution.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_execution.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/037__src-tauri__src__commands_execution.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid and bucket changed`
- Canonical final action bucket: `Safe-now hardening; larger redesign still deferred`
- External research anchors:
  - <https://docs.alpaca.markets/reference/getallorders-1>
  - <https://docs.alpaca.markets/docs/working-with-orders>
  - <https://docs.alpaca.markets/docs/websocket-streaming>
  - <https://docs.alpaca.markets/reference/deleteopenposition-1>
  - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>

## Consolidated Judgment

The earlier passes were right that this file still has larger structural debt around broker lifecycle tracking, `client_order_id`, and a true stop/disarm barrier. But the fourth pass correctly changes the immediate bucket. The next action is no longer "defer unless doing a larger rewrite." There is a bounded, current hardening set available now with clear safety upside and limited blast radius.

The key correction is scope. The safe-now work is not "globally change order fetches to open-only" or "retrofit true cancellation into `spawn_blocking` broker flows." The real bounded hardening is symbol-scoped open-order retrieval for safety actions, a split between cancel-candidate and still-blocks-new-action statuses, exact open-order counting from non-truncated data, and coherent post-submit audit/finalize behavior. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Add a dedicated symbol-scoped open-order retrieval path for safety actions and exact open-order counting instead of reusing the existing broad history query.
- Split the current order-status predicate into:
  - a cancel-candidate set aligned to Alpaca's documented open-for-cancel statuses
  - a broader still-blocks-new-action set that still includes states such as `pending_cancel`
- Compute snapshot open-order counts from the full filtered order set, not from the truncated display slice.
- Treat post-submit audit capture failures as success-plus-warning when the broker submission already succeeded, instead of returning a command error after a real submission.
- Preserve enough live-session state to retry or deterministically log finalize failures rather than clearing the only in-memory handle before finalize succeeds.

## Deferred Work

- Defer a true stop/disarm barrier around broker submission; `spawn_blocking` still cannot be force-aborted once running.
- Defer the `client_order_id` / `trade_updates` redesign and order-lifecycle-aware runner state.
- Defer broader quantity-precision or non-stock asset support cleanup unless the product surface is intentionally widened.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_execution -- --nocapture`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
- Add targeted unit tests for:
  - `pending_cancel` in blocking versus cancel-candidate logic
  - symbol-scoped open-order query construction
  - exact open-order counting from non-truncated data
  - runner-start rollback when artifact creation fails
  - finalize failure preserving retryable state or deterministic logging
  - success-plus-warning behavior after broker submission when audit capture fails
- If safety-action semantics change, run a paper-account smoke for cancel-open-orders, flatten-position, and manual one-shot execution.

## Regression And Feature-Preservation Constraints

- Do not change `LIVE_EXECUTION_RUNNER_EVENT` status strings or public IPC labels in this patch set.
- Do not "fix" the query issue by globally changing the shared history fetch to open-only semantics.
- Do not add per-bar REST polling to the runner.
- Use reqwest query building for any new `symbols` or status-filter query parameters.
- Keep these changes runtime-local; they should not leak into app-core strategy execution or validation surfaces.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- move `commands_execution.rs` to `Safe-now hardening; larger redesign still deferred`
- land the bounded broker-safety and audit-coherence fixes now
- keep the larger order-lifecycle redesign explicitly deferred
- avoid global query narrowing or fake cancellation claims while hardening the current public surface
