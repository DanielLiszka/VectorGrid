# Third-Pass Review: `src-tauri/src/commands_execution.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_execution.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_execution.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_execution.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Third Pass

This file remained one of the highest-ROI review surfaces because it owns broker-facing safety actions. Another pass was warranted to see whether the second-pass findings were still under-specific for implementation sequencing.

## Third-Pass Research Log

1. Alpaca Get All Orders
   - Source: <https://docs.alpaca.markets/v1.1/reference/getallorders-1>
   - Why consulted in third pass:
     - confirm whether the current order fetch still inspects only one page and how query parameters affect the returned set
   - Third-pass constraint informed:
     - a one-page fetch combined with `status=all` can miss older still-open orders

2. Alpaca working with orders
   - Source: <https://docs.alpaca.markets/v1.3/docs/working-with-orders>
   - Why consulted in third pass:
     - keep the longer-term order-identity direction grounded in current broker guidance
   - Third-pass constraint informed:
     - lifecycle-aware state remains the architectural direction, not more snapshot heuristics

3. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in third pass:
     - reassess whether point-in-time safety checks around blocking broker actions remain race-free
   - Third-pass constraint informed:
     - wrapper-level cancellation or stop checks cannot fully protect a blocking broker action once it begins

## What Third Pass Sharpened

### 1. The one-page blind spot is worse than the second pass stated because safety helpers fetch `status=all` and then filter open orders locally

- Evidence:
  - the order fetch path still requests one page
  - the query is broad enough to include closed history
  - open-order filtering happens after the fetch
- Why this matters:
  - older still-open GTC orders can be missed even when the account has fewer than `500` open orders
- Practical implication:
  - if one bounded safety patch lands before a larger redesign, make the safety-action query `open`-only and paginate deliberately

### 2. Broker safety actions still rely on a point-in-time runner inactivity check

- Evidence:
  - the runner inactivity assertion occurs before the blocking broker safety path
  - the runner can be armed after that check but before or during the safety action
- Why this matters:
  - this is a real control-plane race, not just a theoretical wrapper concern
- Practical implication:
  - a robust fix wants a stronger broker-control reservation or equivalent lifecycle-aware guard
  - that still belongs in the larger deferred bucket

### 3. The second-pass partial-audit concern still matters, but it is no longer the top implementation priority

- Practical implication:
  - durable partial cancellation recording is still worthwhile
  - the safer immediate sequence is:
    - query correctness first
    - control-plane race hardening next
    - audit polish after that

## Implementation Priority

- Highest-ROI local hardening:
  - use an open-order scoped broker query for safety actions
- Architectural work that still belongs in the defer bucket:
  - lifecycle-aware broker control instead of point-in-time snapshot checks

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_execution -- --nocapture`

Add or extend mocked transport coverage for:

- older open GTC orders outside the first `status=all` page
- cancel-open-orders and flatten-position safety checks using `status=open`
- start-runner races against blocking broker safety actions
- mixed-outcome cancellation audit recording

If broker control or order identity changes:

- perform a paper-account Alpaca smoke run

## Regression And Feature-Preservation Constraints

- Preserve current HTTP timeout posture
- Prefer query correctness and explicit pagination over more broker polling in the per-bar runner
- Avoid deadlocking runner stop/status paths if a stronger broker-control reservation is added
- Treat live safety changes as broker-integrity work, not wrapper cleanup

## Third-Pass Conclusion

`Bucket unchanged, but the next fix is clearer`

The file still belongs in `Larger refactor but defer`, but the third pass makes the immediate sequence clearer. The most valuable bounded hardening is not UI or audit polish first; it is fixing the safety-action order query so older open orders cannot be hidden behind a one-page `status=all` fetch.
