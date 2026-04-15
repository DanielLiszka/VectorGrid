# Second-Pass Review: `crates/app_core/src/options_signal_manifest.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_signal_manifest.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/options_signal_manifest.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a small but persisted-contract-sensitive manifest builder where index clamping and session semantics matter downstream.

## Stale Check

The live source still:

- clamps event indices
- builds stable event IDs
- derives session IDs from UTC timestamp formatting

The first-pass packet remains current.

## Second-Pass Research Log

1. `chrono::DateTime`
   - Source: <https://docs.rs/chrono/latest/chrono/struct.DateTime.html>
   - Why consulted in second pass:
     - verify failure behavior for timestamp conversion and the UTC construction contract
   - Additional constraint informed:
     - malformed timestamps can still drop `session_id` while leaving the event structurally present

## Delta vs First Pass

### 1. Session labeling is stricter than the first pass emphasized: malformed timestamps can silently yield events without `session_id`

- Why this matters:
  - downstream `SessionChronological` behavior is still keyed to UTC-day buckets, but malformed timestamps degrade quietly
- Practical implication:
  - preserve the current UTC-session contract unless downstream split policy is deliberately reworked

### 2. Index clamping remains stronger than "safe saturation" because both entry and exit can collapse onto the last candle and still look valid

- Practical implication:
  - this remains a good bounded hardening target if manifest correctness is revisited

## Verification If Changed

- `cargo test -p ta_app_core options_signal_manifest::tests:: -- --nocapture`
- `cargo test -p ta_app_core options_expression::tests:: -- --nocapture`
- Tauri-side options-expression replay tests if session or timestamp semantics change

## Regression And Feature-Preservation Constraints

- Preserve `trade_on_next_bar` decision-vs-entry projection semantics
- Preserve current event ID stability unless replay/cache keys migrate together
- Do not change the UTC-session contract casually

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that malformed timestamps can still quietly strip `session_id`, and that index clamping remains stronger than a harmless saturation convenience.
