# Fourth-Pass Review: `crates/app_core/src/options_signal_manifest.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/options_signal_manifest.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file stayed in the review queue because it is tiny, public, and easy to underestimate while still emitting persisted contract data that downstream code hashes, replays, and uses for options-candidate selection. The third-pass packet was directionally right to treat it as semantics-sensitive, but a fourth pass was still justified because the safest next changes are narrower than the packet implied in some places, and one more concrete contract weakness is more important than the broad calendar-provenance framing.

The bucket should **stand**. This is still a `Small surgical refactor` file, not a redesign file. The safe work remains mostly validation tightening, error-text cleanup, documentation, and fixture coverage. The broader policy questions are real, but they are not file-local fixes.

## Fourth-Pass Research Log

1. Re-read the live source file line by line, especially the index handling, timestamp extraction, event-field population, wrapper delegation, and the single file-local test.
2. Re-read the first-, second-, and third-pass packets specifically to challenge their actionability rather than restate them.
3. Re-read the manifest type contract in `options_types.rs` to confirm which fields are durable emitted schema versus file-local convenience, especially `base_exit_timestamp`, `base_exit_bar_index`, `session_id`, `calendar_id`, and `runtime_dataset_id`.
4. Re-read the downstream consumer in `options_expression.rs` to confirm which emitted fields are actually operational:
   - cache hashing of manifest/event fields (`options_expression.rs:333-369`)
   - `SessionChronological` fallback behavior when `session_id` is absent (`482-549`)
   - moneyness targeting and strike selection using `event.raw_underlier_price` (`992-1005`)
   - `BaseExitOrTargetStop` fallback behavior when `event.base_exit_timestamp` is absent (`1210-1219`, `1482-1491`, `3995-3998`)
5. Re-checked current Double-MA trade-emission snippets to confirm that open trades are intentionally represented with `open = true` and an end-of-series `exit_t`, which matters when deciding how aggressive builder-side exit-index validation should be.
6. Re-checked current official Rust documentation guidance:
   - Rust API Guidelines documentation guidance for documenting error conditions
   - `chrono::DateTime::from_timestamp_millis` behavior for out-of-range timestamps
   - the Rust Book’s `Result` guidance for recoverable failures

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The current packet correctly keeps this file in a semantics-sensitive bucket. It is also right that changing event timestamps, IDs, session metadata, or calendar metadata is not a harmless cleanup: those fields are part of the downstream manifest hash in `options_expression.rs:333-369`, so changes here can invalidate replay/cache identity even if the schema stays the same.

It was also right to treat the current no-signal error string as misleading for three of the four public wrappers. The shared helper still returns a `double_ma`-specific failure string even when called through `build_options_signal_manifest_from_composite_trend`, `build_options_signal_manifest_from_regime_hybrid`, and `build_options_signal_manifest_from_breakout_momentum` (`options_signal_manifest.rs:117-118`, `171-260`). That is a real clarity defect and still a safe fix.

Finally, the packet was right that documentation and test coverage are thin for a public, re-exported helper surface. The public builders are re-exported from `lib.rs`, but the file still has only one narrow test and no rustdoc explaining the emitted semantics.

### What it missed or underweighted

#### 1. Closed trades do **not** currently require an exit timestamp, and that silently changes downstream exit semantics

This is the most important missed issue.

For closed trades, the builder populates `base_exit_timestamp` with `candles.timestamp.get(exit_idx).copied()` and does **not** fail if that lookup returns `None` (`options_signal_manifest.rs:70-75`). The type allows `None` because open trades legitimately have no exit yet, but this file also permits `None` for **closed** trades.

That matters because downstream `BaseExitOrTargetStop` evaluation does not use `base_exit_bar_index`; it uses `base_exit_timestamp`, and if that timestamp is absent it silently falls back to fixed-hold behavior (`options_expression.rs:1210-1219`, `1482-1491`, `3995-3998`). In other words, a closed parent trade can degrade from “use the parent exit” into “pretend there was no parent exit and use a hold fallback” without any builder error.

That is a more direct contract weakness than the broader calendar discussion. It is also a better safe-now hardening target.

#### 2. There is no validation that a closed trade exits at or after entry

The file clamps `entry_t` and `exit_t`, but it never checks `exit_t >= entry_t` for `trade.open == false` (`options_signal_manifest.rs:56-62`). If a corrupt or drifted parent trade ever arrives with a closed exit before entry, the emitted event still looks structurally valid.

Downstream, that does not obviously explode; it instead tends to collapse toward an entry-bar cutoff because the evaluation code searches for a cutoff index starting at the entry index. That is exactly the kind of silent semantic degradation a small bridge file should catch early.

This is another better-targeted hardening change than the broader “reject all clamped indexes” recommendation.

#### 3. `raw_underlier_price` is an operational contract, not just metadata

The third-pass packet focused on timestamps and session provenance, but it underweighted that `raw_underlier_price` is directly used to choose moneyness and strike direction downstream (`options_expression.rs:992-1005`).

This builder always sources that field from the entry-bar `open`, with a fallback to `close` (`options_signal_manifest.rs:63-68`). That means the file is already making a strong price-basis choice for the options workflow.

The practical implication is not “change it now.” The safer implication is:

- document the current price-basis contract explicitly
- add a test for `trade_on_next_bar = false`
- avoid “cleaning up” this field without checking candidate-selection parity

Right now, the only local test exercises `trade_on_next_bar = true` and does not pin the same-bar case.

#### 4. The stronger deferred design question is not the error text; it is whether “no signals” should be an error at all

The third-pass packet treated the no-signal path mainly as an error-message cleanup issue. The deeper issue is that this builder returns `Err(...)` when `events` is empty (`options_signal_manifest.rs:117-118`), even though the manifest schema allows an empty `events` vector and downstream split assignment already handles zero events without failing (`options_expression.rs:501-509`).

That does **not** mean the file should change now. It does mean the larger design question is broader than the packet suggested:

- Is “no candidate signal events” a genuine materialization failure?
- Or is it an expected business outcome that should become an empty artifact or a typed no-op result?

That policy choice reaches past this file into IPC, artifact persistence, and UX. It belongs in deferred redesign, not in the next surgical patch.

#### 5. The file is missing a cheap wrapper-parity guard

The earlier packets noted that the public wrappers are intentionally identical today, but they stopped at documentation. A small additive test that proves all four public wrappers currently delegate to the same emitted manifest semantics would be useful and cheap. That would guard against accidental future drift without forcing a premature abstraction change.

### What the current packet overstated or should narrow

#### 1. The calendar/session recommendation is broader than the evidence in this file supports

The packet is right that the file emits hardcoded metadata: both the event and manifest `calendar_id` are set to `"market_hours_utc"` (`options_signal_manifest.rs:113`, `133`), and `session_id` is derived from a UTC date string (`27-30`, `110`).

But the stronger recommendation to “preserve source-dataset provenance” is not yet well-supported by the provided bundle.

Two reasons:

1. The manifest types themselves already default `calendar_id` to `market_hours_utc` in `options_types.rs:428-447`, so the current emitted value is not a file-local aberration.
2. `SessionChronological` split assignment does not depend on a rich calendar object. If `session_id` is absent, it falls back to a deterministic `utc_day:<...>` key derived from `earliest_tradable_timestamp` (`options_expression.rs:534-549`).

So the immediate problem here is not “missing dataset provenance plumbing.” The immediate problem is that the current UTC-day-based behavior is implicit and under-documented. A real provenance/threaded-calendar change is broader subsystem work.

There is also a precision issue in the old wording: this file does **not** implement true “market hours” sessionization. It stamps a calendar label and a UTC-day string. That is weaker than a real market-hours resolver and should be described that way.

#### 2. Blanket rejection of out-of-range `entry_t` and `exit_t` is too coarse as a next step

The current packet pushed toward rejecting out-of-range trade indices. The direction is reasonable, but the next change should be narrower.

The builder only operationally uses `exit_t` for closed trades. For open trades, it explicitly emits `base_exit_timestamp = None` and `base_exit_bar_index = None` regardless of the stored exit metadata (`options_signal_manifest.rs:70-75`). That means a hard failure on an unused open-trade `exit_t` would be a behavior change with limited demonstrated benefit.

A safer next step is:

- always validate `entry_t`
- validate `exit_t`, exit ordering, and exit timestamp presence only when `trade.open == false`

That catches the meaningful corruption paths without expanding failure cases on unused metadata.

#### 3. Missing `session_id` is more of a persisted-metadata weakness than a split-correctness bug

The second/third-pass framing made missing `session_id` sound closer to a split-policy failure. In current code it is weaker than that. Because `SessionChronological` falls back to `utc_day:<...>` keys when `session_id` is absent (`options_expression.rs:534-549`), a missing `session_id` mostly weakens human-readable emitted metadata. It does not automatically destroy session grouping correctness.

That is still worth documenting and testing, but it is not the strongest near-term correctness problem in this file.

## Implementation Priority

### Safe-now hardening

1. **Keep the bucket unchanged.** This file is still a small surgical hardening target.
2. **Require exit timestamp integrity for closed trades.** If `trade.open == false`, fail when the exit timestamp cannot be read, instead of emitting `base_exit_bar_index = Some(...)` with `base_exit_timestamp = None`.
3. **Validate closed-trade exit ordering.** If `trade.open == false`, reject `exit_t < entry_t` rather than letting downstream evaluation silently collapse toward entry-bar behavior.
4. **Narrow index validation by trade state.** Validate `entry_t` unconditionally. Validate `exit_t` only when the exit is semantically used (`trade.open == false`).
5. **Genericize the no-signal error string.** Make it strategy-neutral or pass a caller label into the shared helper.
6. **Add docs and tests for the actual emitted price basis.** Document that `raw_underlier_price` is currently entry-bar `open` with `close` fallback, and add local coverage for `trade_on_next_bar = false` so the same-bar contract is pinned.
7. **Add low-cost fixture coverage.** Specifically:
   - closed trade with missing exit timestamp should fail
   - closed trade with `exit_t < entry_t` should fail
   - open trade should preserve `base_exit_timestamp = None` / `base_exit_bar_index = None`
   - wrapper parity / strategy-neutral error-path coverage
   - existing `trade_on_next_bar = true` behavior stays pinned

### Larger redesign still deferred

1. **No-signal modeling policy.** Decide deliberately whether “no candidate signal events” should remain an error or become an empty manifest / typed no-op outcome. This is broader than this file because downstream code already tolerates zero events structurally.
2. **Calendar/session provenance threading.** Do not try to solve this only inside this helper. If the product really needs exchange-local or dataset-derived session semantics, that should be introduced as an explicit contract across the manifest type, the materialization boundary, and downstream evaluation.
3. **Explicit entry-price semantics.** If non-raw adjustment modes or same-bar execution require a different underlier price basis, thread that as an explicit contract. Do not silently change `raw_underlier_price` heuristics in place.
4. **Typed error surface.** An internal error enum is defensible if this subsystem grows, but it is not the highest-value next move for this small bridge file.

## Verification If Changed

- `cargo test -p ta_app_core options_signal_manifest::tests:: -- --nocapture`
- `cargo test -p ta_app_core options_expression::tests:: -- --nocapture`
- If manifest materialization or no-signal policy changes at the IPC/artifact boundary:
  - `cargo test -p ta_desktop_demo_app commands_options_expression::tests:: -- --nocapture`

Additional fixture coverage I would want before accepting behavior changes here:

- a closed trade whose exit timestamp lookup fails
- a closed trade whose `exit_t` is before `entry_t`
- an open trade proving exit metadata remains intentionally absent
- a `trade_on_next_bar = false` case pinning the current `raw_underlier_price` contract
- parity coverage across the four public wrapper functions
- a regression test proving strategy-neutral no-signal messaging if that cleanup lands

## Regression And Feature-Preservation Constraints

- Preserve `event_id` and `manifest_id` formats unless cache/replay invalidation is intentional and coordinated.
- Do not silently change `decision_timestamp`, `earliest_tradable_timestamp`, `decision_bar_index`, or `earliest_tradable_bar_index`; those fields participate in downstream manifest hashing and inspection identity.
- Do not silently change `raw_underlier_price`; downstream moneyness targeting depends on it.
- Do not turn the current no-signal `Err(...)` into `Ok(empty_manifest)` as part of a small patch. That is a product and IPC behavior decision.
- Treat `calendar_id` changes as persistence-affecting, not cosmetic. Even where runtime behavior does not currently consume it directly, it is hashed downstream.
- No GPU ABI risk is visible here. The real blast radius is artifact persistence, replay cache identity, and validation semantics.

## Fourth-Pass Conclusion

`Bucket should stand: Small surgical refactor.`

The current third-pass packet got the broad classification right: this file is small, public, and contract-sensitive. It also correctly identified thin docs/tests and the misleading `double_ma`-specific no-signal error.

What fourth pass changed is the priority order.

The most actionable missed issue is **not** the broad calendar-provenance framing. It is that closed trades can currently lose `base_exit_timestamp` without builder failure, which silently changes downstream `BaseExitOrTargetStop` behavior. The second most actionable missed issue is the absence of closed-trade exit-order validation. Those are better safe-now hardening targets than a broader metadata-provenance refactor.

The packet also slightly overreached on what this file is currently doing with calendar/session logic. It hardcodes a calendar label and UTC-day-derived session ID, but it does not implement a true market-hours resolver. That means the right near-term move is to document and test the existing UTC-day policy, not to pretend this file alone can recover richer provenance.

So the next pass should stay conservative:

- tighten closed-trade validation
- genericize the error message
- document and test the actual emitted price/session semantics
- defer no-signal modeling and calendar-provenance redesign until the wider options-expression boundary is being changed on purpose
