# Second-Pass Review: `src-tauri/src/commands_execution.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_execution.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_execution.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as broker-integrity-sensitive orchestration and flagged position-only delta calculation, weak stop barriers for in-flight blocking work, and the lack of stronger order-lifecycle state as reasons to defer any larger redesign rather than papering over the edges.

## Stale Check

The live source still has the same broad risk profile:

- stop/halt semantics still rely on shared state plus blocking broker calls
- order-state safety still leans on snapshot helpers rather than durable lifecycle tracking
- order submission still does not use a stable client-owned order identifier

The second pass did not weaken the first-pass defer bucket, but it did find sharper evidence around broker-side limits and audit durability.

## Second-Pass Research Log

1. Alpaca order documentation
   - Source: <https://docs.alpaca.markets/docs/trading/orders/>
   - Why consulted in second pass:
     - tighten the review around order lifecycle and recommended state-tracking behavior
   - Additional constraint informed:
     - order safety work should move toward order-lifecycle-aware state, not simply more position polling

2. Alpaca get-all-orders reference
   - Source: <https://docs.alpaca.markets/v1.1/reference/getallorders-1>
   - Why consulted in second pass:
     - confirm the effective limit and semantics behind the review/flatten/cancel safety actions
   - Additional constraint informed:
     - the current helpers are capped at Alpaca's documented `500` order page and do not paginate

3. Alpaca client-order guidance
   - Source: <https://docs.alpaca.markets/v1.3/docs/working-with-orders>
   - Why consulted in second pass:
     - assess whether missing `client_order_id` is merely ergonomics or materially weakens reconciliation
   - Additional constraint informed:
     - explicit client order identity remains a first-class part of a future-safe design

4. Reqwest blocking client builder
   - Source: <https://docs.rs/reqwest/latest/reqwest/blocking/struct.ClientBuilder.html>
   - Why consulted in second pass:
     - verify whether the current broker client already has bounded timeout behavior worth preserving
   - Additional constraint informed:
     - the explicit request/connect/pool timeout posture is a must-preserve safety guardrail

5. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in second pass:
     - sharpen the first-pass stop/cancel race analysis
   - Additional constraint informed:
     - blocking broker work cannot be hard-aborted once started

## Delta vs First Pass

## Stronger Safety Finding

### 1. Review, flatten, and cancel-all safety actions can undercount unusually busy accounts because open-order fetches stop at one 500-order page

- Evidence:
  - the broker helper clamps to Alpaca's documented page size and does not paginate
- Why this matters:
  - both "cancel all open orders" and "flatten only if no open orders exist" are only as complete as the fetched broker-visible order set
- Practical implication:
  - this is not the main architectural problem in the file, but it is a real safety blind spot that should be covered if the safety-action helpers are touched

## Stronger Audit Finding

### 2. Partial-success cancellation still has weak durable recovery because the exact canceled IDs are not recorded before the mixed-outcome error returns

- Evidence:
  - the mixed success/error branch returns before the durable audit event path used on full success
- Why this matters:
  - operator recovery is weaker when the session log cannot distinguish "nothing canceled" from "some IDs canceled before failure"
- Practical implication:
  - if this area changes, durable partial-outcome recording is a better local improvement than more UI polish

## First-Pass Direction Still Holds

### 3. The long-term fix is still client order identity plus order-lifecycle-aware state, not extra wrapper churn

- Evidence:
  - the file still lacks explicit `client_order_id`
  - the control plane still derives too much from snapshots
- Why this matters:
  - the second pass reinforces the same architectural direction with stronger broker-side evidence
- Practical implication:
  - keep the `Larger refactor but defer` bucket

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_execution -- --nocapture`

Add or extend mocked transport coverage for:

- `>500` open-order scenarios
- partial-success cancellation with durable audit recording
- flatten refusing when any open order exists across the full broker-visible set
- stop/halt behavior while a blocking broker path is already in flight

If order identity or lifecycle tracking changes:

- perform a paper-account smoke run against Alpaca paper trading

## Regression And Feature-Preservation Constraints

- Preserve explicit broker HTTP timeouts
- Preserve current runner generation and abort-marker behavior unless replaced by something stronger
- Do not "fix" live safety by amplifying per-bar REST traffic
- Treat any order-state redesign as broker-integrity work, not wrapper cleanup

## Second-Pass Conclusion

`Bucket unchanged, but broker-limit and partial-audit gaps are now clearer`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass adds two sharper findings: safety actions currently stop at one 500-order page, and mixed-outcome cancel flows still do not preserve a durable record of which order IDs actually got canceled.
