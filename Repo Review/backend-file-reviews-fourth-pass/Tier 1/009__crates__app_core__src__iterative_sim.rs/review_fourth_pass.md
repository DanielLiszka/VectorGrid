# Fourth-Pass Review: `crates/app_core/src/iterative_sim.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/iterative_sim.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/iterative_sim.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/iterative_sim.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/iterative_sim.rs.review.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

`iterative_sim.rs` stayed worth a fourth pass because the third-pass packet was directionally right about public-contract weakness, but it still needed pressure-testing on one important question: is the next best move really a deferred larger redesign, or is there a bounded hardening step here that materially improves the file without destabilizing validation semantics?

After re-reading the live file, the prior packets, the crate export surface, and the available `double_ma.rs` context, my conclusion is that the third-pass bucket should **change**.

The third pass correctly identified two real issues:

1. invalid input still collapses into `Metrics::default()` instead of an explicit failure path;
2. the iterative path still returns a `Metrics` value with several fields left as structural placeholders.

But it over-ascribed the result-shape problem to this file alone. The broader repo context shows the main `double_ma` path already tolerates a partially populated `Metrics` contract in some areas, including structural zeros for trade-distribution fields such as `closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`, and `gross_loss`. That means the crate-wide result-shape redesign is still real debt, but it is **not** a reason by itself to keep this file in a larger-refactor bucket.

The sharper file-local reading is:

- there is a **safe, bounded hardening path now**;
- the larger redesign is still worth deferring;
- therefore the action bucket for this file should move back to **`Small surgical refactor`**.

## Fourth-Pass Research Log

This pass used both repo-local and standards context.

Repo-local review focused on:

- the live `iterative_sim.rs` implementation, especially the public DTOs, invalid-input guard, overlay state machine, slippage path, and metric finalization;
- `crates/app_core/src/lib.rs` to confirm that `iterative_sim` is a public module and that `ValidationEngine`, `IntrabarRule`, `IterativeSimConfig`, and `IterativeMetricsDelta` are re-exported from the crate root;
- the earlier packets to distinguish live-source issues from inherited review assumptions;
- available `double_ma.rs` context to compare what the broader `Metrics` contract already does in the main path.

Standards context re-checked:

- Rust API documentation expectations for public items;
- Rust’s `Result`-oriented error surface guidance for recoverable input failures;
- the `#[must_use]` diagnostic model for values where ignoring the return is typically a mistake;
- Serde default-field behavior, because `IterativeSimConfig` is a public serde-backed config surface.

The most important repo-local comparison from this pass was that `double_ma.rs` already reuses shared risk finalization for several fields that `iterative_sim.rs` leaves at zero, while still leaving some trade-distribution fields as structural zero placeholders. That materially narrows the truly file-local gap.

## What Fourth Pass Sharpened

### 1. The third-pass bucket should change, not stand

The current third-pass bucket is too aggressive for the **next** action.

Why it should change:

- `IterativeMetricsDelta` only compares `pnl`, `sharpe`, `max_dd`, `trades`, `exposure`, and `net_exposure`, which already tells you the module’s practical comparison surface is narrower than the full `Metrics` struct.
- The live repo context for `double_ma.rs` shows that structural-zero `Metrics` fields are already broader crate debt, not an iterative-only defect.
- The file has several bounded hardening moves that are small and high leverage:
  - additive fallible entrypoint;
  - documentation and contract tests;
  - selective reuse of shared risk finalization for the fields this loop already tracks;
  - pinning or clarifying current overlay/slippage semantics.

That is a classic `Small surgical refactor` posture, not a file-local “larger refactor but defer” posture.

### 2. What the third-pass packet got right

The third pass was correct to keep these constraints front and center:

- do **not** casually rewrite the bar loop just to make it shorter;
- preserve intrabar, cooldown, persistence, and trade-cost semantics unless a change is deliberate and verified;
- prefer an additive `try_...`/`Result` path over silently repurposing the legacy function;
- treat public docs and public tests as part of the fix, not nice-to-have cleanup.

It was also correct that invalid-input fallback is still a real contract weakness, and that returning a broad `Metrics` type without explaining which fields are authoritative today is misleading.

### 3. What the third-pass packet overstated

The biggest overstatement is treating the entire partial-`Metrics` problem as though `iterative_sim.rs` alone had drifted from a fully authoritative crate-wide `Metrics` contract.

That is too strong.

Available `double_ma.rs` context shows:

- the main path already computes a richer subset through `finalize_risk_metrics(...)`;
- but even there, some `Metrics` fields remain structural zeros.

So the right file-local conclusion is not “this file must wait for a larger result-type redesign.” The right conclusion is “this file should be brought **closer to the existing shared finalization contract**, while the broader `Metrics` redesign remains deferred.”

Related overstatement: the third-pass packet leaned heavily on downstream-consumer risk that was cited in earlier packets, but that downstream consumption was not re-proved inside the current fourth-pass bundle. The risk is plausible and likely real, but the fourth-pass wording should stay slightly more conservative there.

### 4. What the third-pass packet missed or underweighted

#### a. Cooldown semantics have an off-by-one risk

`overlay_on_trade_close()` sets `cooldown_left = cooldown_bars`, but `overlay_on_bar_start()` decrements that counter **before** `overlay_apply_entry_cooldown()` checks it.

That means a configured `cooldown_bars = 1` blocks zero future entry bars, and more generally a configured cooldown of `N` blocks only `N - 1` future bars unless the broader product contract intentionally defines cooldown that way.

This is not a speculative style issue. It is a semantics risk in the state machine itself, and it deserved to be called out explicitly.

I would not “fix” it blindly, because changing it changes observed behavior. But it absolutely deserves a focused test and a contract comment.

#### b. Exposure and net-exposure semantics are under-specified on intrabar-exit bars

The loop adds return for a bar, may flatten intrabar via stop/take, and then accumulates `sum_abs_pos` / `sum_pos` using the **post-exit** `pos` value.

So a bar that contributed positive or negative PnL while a position existed intrabar can still count as zero exposure if the stop/take flattened the position before the exposure counters are updated.

That may be intentional, but it is not obvious, and it means exposure is not “bars with any capital at risk.” It is closer to an end-of-bar state snapshot.

This matters because `IterativeMetricsDelta` explicitly compares `exposure` and `net_exposure`. Even if the current behavior remains, it should be documented and tested, not left implicit.

#### c. `slippage_vol_mult` currently uses same-bar realized move as its multiplier input

`trade_log_cost()` takes `lr_abs`, and the caller feeds it `lr[t].abs()` for the current bar.

That means entry/exit cost at the start of a bar is scaled by the realized magnitude of that same bar. As a backtest heuristic this may be acceptable, but it is an ex-post slippage model, not a neutral transaction-cost abstraction.

The third pass talked about config normalization, but it underweighted that this specific config field carries a semantics claim that should be documented. Again, this does **not** justify a redesign; it just means the public contract is thinner than it should be.

#### d. The safe-now hardening path is stronger than the third pass gave it credit for

The third pass talked about redesign direction, but it underweighted the bounded path that exists today:

- keep the current public function as a compatibility wrapper if needed;
- add a fallible sibling for invalid input;
- reuse the shared risk finalizer for the fields this loop already has enough information to compute;
- add tight behavior tests around cooldown, intrabar ties, exposure counting, and slippage semantics.

That is enough value to justify changing the bucket.

### 5. Safe-now hardening vs larger redesign still deferred

#### Safe-now hardening

These are the changes I would consider the best next pass for this file:

1. **Add a fallible sibling**
   - Add `try_simulate_iterative_from_signals(...) -> Result<Metrics, IterativeSimError>`.
   - Keep the current function as a compatibility wrapper for now if external callers may depend on `Metrics::default()` fallback.
   - Add `debug_assert!` shape checks inside the legacy wrapper so internal misuse shows up earlier in debug/test builds.

2. **Document the real contract**
   - Add rustdoc on `IntrabarRule`, `IterativeSimConfig`, `OhlcSlice`, and `simulate_iterative_from_signals`.
   - Explicitly say:
     - input lengths must match;
     - non-finite stop/take values are normalized away;
     - `slippage_vol_mult` uses same-bar absolute log return as a cost multiplier input;
     - exposure is derived from this loop’s state accounting, not from a fractional intrabar time model.

3. **Compute the missing risk metrics that are already derivable here**
   - The shared `double_ma` finalizer already computes a richer subset than this file currently returns.
   - Bring `iterative_sim.rs` up to that shared subset for fields such as:
     - `downside_frequency`
     - `downside_severity`
     - `kappa_tau`
     - `long_mean_return`
     - `short_mean_return`
     - `underwater_fraction`
     - `max_underwater_duration_bars`
   - Leave trade-distribution fields alone for now if they are still broader crate debt.

4. **Add focused semantics tests**
   - invalid shape and invalid `start_t` behavior on both legacy and `try_` paths;
   - every `IntrabarRule` tie case;
   - cooldown semantics for `cooldown_bars = 1` and `2`;
   - exposure/net-exposure behavior on intrabar stop/take exits;
   - slippage heuristic behavior for `slippage_vol_mult > 0`.

5. **Consider `#[must_use]` on public return-bearing helpers**
   - This is minor, but for an execution helper returning `Metrics`, ignoring the value is usually a mistake.

#### Larger redesign still deferred

These still belong in the deferred bucket:

1. **A crate-wide `Metrics` contract cleanup**
   - If the project wants `Metrics` to be universally authoritative, this is broader than `iterative_sim.rs`.
   - It would need agreement across `double_ma`, validation, scoring, reports, and any consumer that already tolerates placeholder fields.

2. **A distinct iterative-specific result type**
   - This is still a valid long-term design direction.
   - But it is not the best next move for this file because there is still meaningful bounded value available without it.

3. **Shared helper extraction with `asr.rs`**
   - The duplication risk is real.
   - But helper extraction is semantics-sensitive and should happen only after parity tests pin both call sites.

## Implementation Priority

### Highest-value next pass

1. additive fallible API;
2. docs plus contract tests;
3. selective reuse of shared risk finalization for metrics this loop already has enough state to compute.

### Changes I would not merge casually

These recommendations are real, but they are also regression-prone if applied too quickly:

- **changing cooldown semantics** without first confirming product intent;
- **changing exposure accounting** without agreeing on whether partial intrabar exposure should count as full-bar, end-of-bar, or fractional-bar exposure;
- **changing the slippage heuristic** without treating it as an observable semantics change;
- **switching the legacy function to error-returning behavior in place** instead of adding a new path.

### Practical bucket assessment

So the action split is:

- **Do now:** small surgical hardening in this file.
- **Defer:** broader result-type redesign and helper extraction.

That is why the fourth-pass bucket changes to `Small surgical refactor`.

## Verification If Changed

At minimum, keep the existing verification lane and extend it with the missing behavior checks.

Recommended local verification:

- `cargo test -p ta_app_core iterative_ -- --nocapture`
- `cargo test -p ta_app_core double_ma_validation_iterative_engine_emits_delta_fields -- --nocapture`
- `cargo test -p ta_app_core double_ma_validation_iterative_engine_matches_optimizer_top_candidates -- --nocapture`
- `cargo test -p ta_app_core composite_validation_iterative_engine_emits_delta_fields -- --nocapture`

Add focused tests for:

- invalid-input fallback on the legacy path;
- explicit failure on the new `try_` path;
- `IntrabarRule::{Pessimistic, Optimistic, StopFirst, TakeFirst}` tie behavior;
- cooldown behavior at `1`, `2`, and a larger value;
- exposure/net-exposure on intrabar stop/take exit bars;
- shared-finalizer parity for the metrics newly populated from the bounded hardening step;
- parity against the duplicated `asr.rs` helpers if duplication remains.

If the shared finalizer is reused, also run the higher-level validation consumers that compare iterative and non-iterative outputs, because field-population changes are observable even when the bar loop itself is untouched.

## Regression And Feature-Preservation Constraints

- Preserve the current bar-ordering semantics unless a specific behavior change is intended and tested.
- Do not flatten the state machine for readability if that makes fill, flip, cooldown, or persistence ordering harder to audit.
- Treat `cooldown_bars` as semantics-bearing. The current ordering may be wrong, but the fix still needs product confirmation.
- Treat `slippage_vol_mult` as semantics-bearing. Reword and document it before changing it.
- If the legacy infallible function remains, preserve its current behavior deliberately rather than by accident.
- If additional `Metrics` fields start being computed, verify any downstream score, report, or artifact surface that may have implicitly relied on structural zero placeholders.
- If helper extraction with `asr.rs` is attempted later, land parity tests first.

## Fourth-Pass Conclusion

The current third-pass packet was useful, but its bucket should **change**.

`iterative_sim.rs` does **not** need to sit in `Larger refactor but defer` as its next action. The fourth pass shows that the file has a meaningful, bounded hardening path:

- add a fallible sibling;
- document the real public contract;
- align iterative metric finalization with the existing shared risk-finalizer where possible;
- pin today’s semantics around cooldown, intrabar ties, exposure, and slippage.

The broader redesign remains valid, but it is broader crate debt, not the best next file-local move.

So the right fourth-pass call is:

- **Third-pass bucket:** wrong for the next action.
- **Fourth-pass bucket:** `Small surgical refactor`.
- **Reason:** the prior packet was right about contract weakness, but it underweighted both the shared nature of the broader `Metrics` debt and the availability of a safer bounded hardening step in this file right now.
