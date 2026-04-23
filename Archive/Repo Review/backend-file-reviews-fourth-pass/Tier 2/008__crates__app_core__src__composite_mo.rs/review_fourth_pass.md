# Fourth-Pass Review: `crates/app_core/src/composite_mo.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/composite_mo.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file got a fourth pass because the live source still justifies the third-pass bucket, but the current third-pass packet is too generic to be the best guide for safe follow-up work.

The current third-pass packet gets one important thing right: this is still not a “rewrite the module” situation. `composite_mo.rs` remains a public-surface, telemetry-sensitive orchestration file whose safest next moves are narrow, behavior-preserving hardening changes rather than structural surgery.

What did not hold up as well under a fresh source reread is the packet’s emphasis. The third-pass writeup frames this file primarily as a hash/cache/runtime-scope problem. The live source does not support that emphasis strongly enough. The only hash-derived identity in the file is a private in-run cache key used by `dynamic_plan_cache_key` (`composite_mo.rs:317-335`), and the two caches that matter here are constructed inside `run_composite_mo_optimization_blocking_with_candles` (`composite_mo.rs:1823-1829`), not as process-lifetime singletons. The stronger remaining issues are still the public request/telemetry contract and a few concrete invalid-path edge cases.

So the bucket should stand, but the packet should be sharpened away from generic cache-scope language and back toward the narrower request/telemetry surfaces that can still surprise callers.

## Fourth-Pass Research Log

1. Re-read the live source with focus on:
   - public request/result/telemetry structs (`composite_mo.rs:70-191`, `385-466`)
   - GPU prescan request normalization and domain narrowing (`composite_mo.rs:1208-1558`)
   - MA-bank construction under prescan allowlists (`composite_mo.rs:1727-1811`)
   - candidate evaluation, invalid exits, and counter accounting (`composite_mo.rs:1893-2597`)
   - exact-grid / sampler routing and final `Trade22` rewrite (`composite_mo.rs:2777-3099`)
   - in-file tests, especially prescan, exact-grid, dynamic-strategy parity, and decode coverage (`composite_mo.rs:3210-4414`)

2. Re-read the current third-pass packet and compared its framing against the live source.
   - The bucket call still fits.
   - The supporting research focus does not fit as well, because the file does not use `OnceLock` or `thread_local!`, and its hash use is private and in-run.

3. Re-checked the earlier review history only where it changed risk posture.
   - The second-pass note about Serde tag spellings is still materially relevant because `CompositeMoSampler` is an internally tagged enum (`composite_mo.rs:77-83`).
   - The first-pass API/telemetry concerns remain more concrete than the third-pass cache-scope concerns.

4. Re-checked external primary sources only where they materially changed the review:
   - Rust `std::hash::DefaultHasher` docs: useful for confirming that hash outputs are not a stable public identity surface.
   - Rust `std::fmt::Debug` docs: useful because this file serializes `gpu_prescan.mode` using `format!("{:?}", cfg.mode)` (`composite_mo.rs:1543`), and `Debug` output is programmer-facing rather than a stable wire schema.
   - Serde enum representation docs: useful for confirming that `#[serde(tag = "sampler")]` makes the sampler variant spelling part of the request contract.
   - Rust API Guidelines checklist: useful mainly to reinforce argument-validation and public-contract clarity, not to justify a broader refactor.

## What Fourth Pass Sharpened

### 1. The current third-pass bucket should stand, but its supporting rationale should narrow

`Small surgical refactor` is still the right bucket.

The file is too sensitive for opportunistic cleanup:
- it owns public request/result/telemetry structs (`composite_mo.rs:70-191`, `385-466`)
- it rewrites the effective search domain when `gpu_prescan` is enabled (`composite_mo.rs:1459-1497`)
- it conditionally swaps into exact-grid execution (`composite_mo.rs:2777-3011`)
- it deliberately rewrites returned Pareto metrics onto the CPU `Trade22` surface (`composite_mo.rs:3030-3099`)

That is still a surgical-hardening profile, not a redesign profile.

What should change is the emphasis. The third-pass packet overweights cache/runtime-scope language relative to the live source. The hash-derived key at `composite_mo.rs:317-335` is private, and the caches are run-local `Arc<Mutex<...>>` values created inside the optimization function (`composite_mo.rs:1823-1829`). That is a real implementation detail, but it is not the strongest remaining public-risk surface.

### 2. The strongest still-missed issue is not “cache identity”; it is non-exhaustive rejection telemetry

The current search telemetry is not just under-documented. It is also non-exhaustive.

The file defines four search counters at `composite_mo.rs:1893-1896`:
- `feasible_points`
- `rejected_decode_or_order`
- `rejected_prescan_pair`
- `rejected_other_constraints`

But several invalid exits bypass all of them:
- structural parameter invalidation returns early at `composite_mo.rs:2271-2289`
- missing prescan-built MA banks or rows return early at `composite_mo.rs:2021-2066` and `2068-2105`
- indicator compute failures return early at `composite_mo.rs:2301-2374`
- dynamic-plan compile/eval failures return early at `composite_mo.rs:2480-2528`

The only place `rejected_other_constraints` is incremented is after full metrics evaluation at `composite_mo.rs:2587-2589`.

That means the current counters do not cover all invalid candidate exits, and `feasible_points + rejected_*` is not an exhaustive partition of `actual_evaluations`. That is a more concrete remaining issue than the current third-pass packet’s cache/hash framing.

This also sharpens one earlier recommendation: adding dedicated telemetry for internal compute failures would be useful, but the narrower safe-now hardening is first to make the existing rejection accounting internally consistent.

### 3. One earlier “safe-now” fix is directionally right but under-specified

The first-pass packet was directionally right to flag that known-invalid candidates still flow into the expensive metrics path. The live source still does that for the `start_t >= n` and cancellation path:
- it only pushes a violation marker at `composite_mo.rs:2404-2408`
- then still proceeds into metrics evaluation at `composite_mo.rs:2431-2548`
- and only later increments `rejected_other_constraints` at `composite_mo.rs:2587-2589`

However, the small fix is not just “return early.” A naive early return would silently change search telemetry because it would skip the only current `rejected_other_constraints` increment site.

So the safe-now version of that fix is:
- short-circuit the expensive metrics work for known-invalid `start_t` / cancelled candidates
- but preserve the current rejection accounting semantics explicitly, either by incrementing `rejected_other_constraints` before return or by moving invalid-path accounting into a shared helper

That is a real sharpening over the current packet. The optimization is still justified, but the implementation must preserve telemetry behavior.

### 4. `gpu_prescan.ma_pairs` has a stronger uncovered edge case than the current packet calls out

The current packet correctly notes that `gpu_prescan` is only operative for `IntRange` MA-length domains.

What it still misses is the fixed-type allowlist trap.

When `gpu_prescan.ma_pairs` is provided, the code uses that list as-is after normalization and dedup (`composite_mo.rs:1261-1279`). It does **not** automatically add the request’s base `(fast_ma_type, slow_ma_type)` pair when `ma_pairs` is explicitly provided.

Later, the file builds MA banks only for type pairs that survived prescan (`composite_mo.rs:1727-1811`), and candidate evaluation rejects or starves type pairs that were never scanned:
- rejected prescan-pair path at `composite_mo.rs:1994-2017`
- missing-bank path at `composite_mo.rs:2021-2066`

That creates a concrete bad request shape the current tests do not cover:
- `fast_ma_type` / `slow_ma_type` are fixed in the request
- `gpu_prescan.ma_pairs` excludes that fixed pair
- prescan still succeeds for some other pair
- evaluation then runs against a search space whose only executable MA-type pair was never scanned

This is a stronger public-contract edge case than the third-pass packet’s cache-scope focus.

The existing multi-pair prescan test (`composite_mo.rs:4203-4287`) only covers the additive case where the base pair is still present. It does not cover exclusion.

The narrowest safe-now hardening is not a sweeping semantic change. It is:
- for **fixed-type requests** (no `fast_ma_type`/`slow_ma_type` param in the search space), reject `gpu_prescan.ma_pairs` when it excludes the fixed base pair

A broader validation rule for optimized MA-type requests is more contract-sensitive and should remain deferred unless the intended allowlist semantics are documented first.

### 5. The current third-pass packet underweights the still-live request/wire-contract concerns from earlier passes

Two earlier points remain more actionable than the third-pass cache focus:

First, the ignored top-level request `mode` field is still a real contract hazard.
- The request exposes `mode: Option<OptimizationMode>` at `composite_mo.rs:191`
- the entrypoint immediately discards it at `composite_mo.rs:1145-1146`

The current packet does not revisit that, but it should remain near the top of the queue.

Second, the second-pass note about sampler wire tags is still important.
- `CompositeMoSampler` is an internally tagged Serde enum at `composite_mo.rs:77-83`
- there is no rename attribute on the variants
- so the current Rust variant spellings are part of the request wire shape

Those are concrete request-surface issues. They matter more here than generalized cache-scope language.

### 6. Some earlier recommendations are too aggressive for this contract surface

A few fixes that sound attractive remain too strong for a conservative fourth-pass recommendation.

#### `mode`: documentation-and-test is safer than hard rejection right now

The current request comment explicitly says the field is present to keep the request shape similar (`composite_mo.rs:189-191`). That strongly suggests shared request-builder compatibility.

So while rejecting non-`None` values would be cleaner from a pure API perspective, it is not the safest immediate recommendation. The safer immediate move is:
- document that `mode` is currently ignored
- add a test that pins that behavior
- reserve hard rejection for a versioned or caller-audited cleanup

#### categorical `gpu_prescan`: documentation is safer than turning the current no-op into an immediate hard error

Today, `gpu_prescan` is only hard-rejected for non-`IntRange` MA-length domains when the period sets are already too large (`composite_mo.rs:1554-1558`). Otherwise it is accepted but non-operative.

That behavior is ambiguous, but it is already deployed behavior. So the conservative immediate recommendation is still documentation or explicit telemetry of the skip, not a blanket hard rejection.

### 7. The “make result types deserializable for campaign reuse” idea should stay deferred more firmly than earlier packets imply

That cross-file debt item is real, but it is not a safe-now change.

The reason is visible in this file’s own public telemetry shape:
- `CompositeMoPrescanTelemetry.mode` is serialized from `format!("{:?}", cfg.mode)` (`composite_mo.rs:1543`)
- `backend_used`, `backend_route`, and `backend_fallback_reason` are free-form summary strings (`composite_mo.rs:1538-1542`)
- `CompositeMoSearchTelemetry.sampler` is an effective-route label string such as `ExactGrid`, `Nsga2`, `Random`, `Tpe`, or `Qmc` (`composite_mo.rs:2941-3010`, `3045`)

That is fine for one-way reporting. It is a much weaker foundation for a round-trippable persisted schema.

This is where the Rust `Debug` docs matter. `Debug` output is programmer-facing, and derived `Debug` formats are not stable. That makes `format!("{:?}", cfg.mode)` especially awkward to bless as persisted round-trip data.

So the larger redesign should be:
- stabilize or split reporting-only text fields from persistence-grade schema fields first
- only then consider deriving `Deserialize` for full-result artifact reuse

That keeps the debt item alive, but it moves it firmly into the deferred bucket.

## Implementation Priority

### Safe-now hardening

1. **Make rejection accounting exhaustive before adding new rejection counters.**
   - Keep the current public telemetry shape if possible.
   - Audit every early invalid return in `eval_one` and ensure it increments either its dedicated bucket or `rejected_other_constraints` before returning.
   - This is the smallest change that materially improves search telemetry truthfulness.

2. **Short-circuit the `start_t >= n` / cancellation path only if accounting is preserved.**
   - The performance win is real.
   - The narrow safe implementation is “short-circuit plus preserved counter semantics,” not “short-circuit blindly.”

3. **Validate `gpu_prescan.ma_pairs` coverage for fixed-type requests.**
   - If the request does not optimize `fast_ma_type` / `slow_ma_type`, require that explicit `ma_pairs` includes the fixed base pair.
   - This closes a real request-shape trap without redefining the broader semantics for optimized MA-type domains.

4. **Document and test ignored request `mode` behavior instead of rejecting it immediately.**
   - That preserves compatibility with shared request builders.
   - It also makes later tightening easier because the current behavior becomes explicit rather than accidental.

5. **Carry forward the second-pass wire-contract tests before adding cache-behavior tests.**
   - Serde decode coverage for sampler tag spellings is still a better use of the next test slot than generic cache-reset coverage.

### Larger redesign still deferred

1. **Do not turn the full result/telemetry surface into a round-trippable persistence schema yet.**
   - The string-heavy and partly debug-derived telemetry fields are not ready for that.

2. **Do not widen the fixed-type `ma_pairs` validation into a broad MA-type-domain policy change yet.**
   - For optimized `fast_ma_type` / `slow_ma_type`, the current behavior is effectively an allowlist.
   - That may or may not be desirable, but changing it is a contract question, not a one-line hardening patch.

3. **Do not rename telemetry fields or redefine evaluation counters casually.**
   - Current tests already pin several semantics around `requested_evaluations`, exact-grid selection, and `total_evaluations`.

4. **Do not spend a pass on hash/cache refactoring here.**
   - The live file does not justify making cache mechanics the lead item.
   - Those details are private and in-run.

5. **Do not touch the final `Trade22` rewrite flow in this packet.**
   - It remains deliberate semantics, not accidental duplicate work.

## Verification If Changed

At minimum, keep the current focused suite:

- `cargo test -p ta_app_core composite_mo -- --nocapture`
- `cargo test -p ta_app_core composite_mo_fast_objectives_emit_trade22_metric_surface -- --nocapture`
- `cargo test -p ta_app_core composite_mo_gpu_prescan_narrows_large_len_ranges -- --nocapture`
- `cargo test -p ta_app_core composite_mo_gpu_prescan_uses_exact_grid_for_small_candidate_sets -- --nocapture`
- `cargo test -p ta_app_core composite_mo_gpu_prescan_supports_multiple_ma_pairs -- --nocapture`
- `cargo test -p ta_app_core composite_mo_dynamic_strategy_env_path_matches_legacy_results -- --nocapture`
- `cargo test -p ta_app_core composite_mo_dynamic_strategy_shadow_mode_matches_legacy_results -- --nocapture`
- `cargo test -p ta_app_core composite_mo_candidates_validate_consistently -- --nocapture`

If the safe-now hardening above is implemented, add these targeted tests:

1. **Fixed-type prescan exclusion test**
   - fixed request MA types
   - explicit `gpu_prescan.ma_pairs` that excludes the fixed pair
   - assert a request error rather than an all-rejected search

2. **Ignored `mode` contract test**
   - send `mode: Some(OptimizationMode::Grid)` (or another non-`None` value)
   - assert that current behavior stays accepted and ignored, unless the contract is intentionally versioned and changed

3. **Rejection-accounting test**
   - hit one early invalid path that currently bypasses counters (for example a structural invalid combination or a forced `start_t >= n` case)
   - assert that `actual_evaluations` and the rejection counters remain semantically aligned after the change

4. **Sampler serde wire-shape test**
   - deserialize representative JSON payloads for `Nsga2`, `Random`, `Tpe`, and `Qmc`
   - pin the current accepted tag spellings before any public API cleanup work

## Regression And Feature-Preservation Constraints

- **The third-pass bucket should remain unchanged.** Nothing in the live source justifies escalating this into a redesign packet.

- **Do not convert documentation-safe fixes into behavior-breaking request validation without caller audit.**
  - This applies especially to ignored `mode` and the currently accepted-but-non-operative categorical `gpu_prescan` case.

- **If you short-circuit invalid candidates earlier, preserve telemetry semantics.**
  - A small performance cleanup that silently changes `rejected_other_constraints` accounting is not actually low risk.

- **If you validate `gpu_prescan.ma_pairs`, do the fixed-type case first.**
  - Broader validation against all optimized MA-type combinations is materially more contract-sensitive.

- **Do not derive `Deserialize` on the full result surface until reporting-only strings are separated from persistence-grade schema.**
  - `mode`, `sampler`, `backend_route`, and `backend_fallback_reason` are the immediate blockers visible in this file.

- **Preserve current sampler tag spellings unless a compatibility plan exists.**
  - `CompositeMoSampler` is part of the incoming wire contract.

- **Preserve current evaluation-count semantics unless downstream consumers are updated together.**
  - `requested_evaluations`, `actual_evaluations`, exact-grid behavior, and `total_evaluations` already have test coverage and likely UI/report coupling.

- **Do not use the current fourth pass to relitigate the final `Trade22` rewrite.**
  - That is still a deliberate semantics choice.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `crates/app_core/src/composite_mo.rs` is still a `Small surgical refactor`, not a redesign.

What should change is the packet emphasis.

The current third-pass packet was right to avoid a broad rewrite, and it was directionally right that the private `DefaultHasher` key should not be treated as a stable identity surface. But it overstates cache/runtime-scope risk relative to the live source and underweights stronger, more concrete issues that still sit on the public contract surface.

The strongest remaining work is:
- exhaust invalid-path rejection accounting
- short-circuit known-invalid candidates only with preserved telemetry semantics
- validate the fixed-type `gpu_prescan.ma_pairs` exclusion trap
- keep `mode` and prescan no-op cleanup documentation-first rather than behavior-breaking

The larger redesign still deferred is equally clear:
- do not freeze the current result/telemetry surface into a round-trippable artifact schema yet
- do not widen MA-type allowlist semantics or rename public telemetry fields without explicit compatibility work

So the bucket stays the same, but the fourth-pass guidance is sharper: the next improvements should land on request/telemetry truthfulness and prescan edge-case validation, not on generic cache-scope cleanup.
