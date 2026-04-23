# Fourth-Pass Review: `crates/app_core/src/options_expression.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/options_expression.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file was worth a fourth pass because the third-pass packet was directionally right about caution, but not fully right about emphasis. The live source still justifies a conservative bucket: this is a public, blocking, benchmark-sensitive engine that owns split assignment, dense candidate/summary caches, ranking, exactification, feasibility reporting, and inspection replay. A careless change here can move ranking, stress metrics, GPU route behavior, and persisted review surfaces all at once.

What the third pass got right is the general posture. Cache operability matters. Inspection is currently tied to live reranking. Lexicographic eviction is still secondary. Those points hold.

What needed sharpening is which issues are actually the best next targets. The live source now shows three things more clearly than the current packet states.

First, some concrete inspection correctness bugs are already present today and are safer to fix than an inspect-by-stable-hash redesign. Second, the cache story is less about poisoning than about ownership and hit-path copying. Third, there is a larger stress-semantics question in the live code that is more consequential than the packet suggests, but too risky to fold into a “small surgical” patch.

## Fourth-Pass Research Log

I reread the live `options_expression.rs` first, then checked the third-pass packet against the actual code paths it discusses. I used `options_types.rs` only where the public DTO contract mattered, especially around `tuple_index` and the invalid-reason surface.

The code areas that changed my view most were the aggregate fill helpers and their callers (`options_expression.rs:659-790`), the cache get-or-build wrappers (`options_expression.rs:4209-4288`), the inspection row/path builders (`options_expression.rs:4482-4659`), the feasibility timing/report assembly (`options_expression.rs:2535-2678`, `5261-5614`), and the exit-summary representation (`options_expression.rs:2076-2078`, `3894-4199`).

I also checked official Rust guidance on three narrow points: `Mutex` poison recovery and `clear_poison`, `Arc` shared-ownership cloning as the obvious alternative to full buffer cloning, and Rust API documentation guidance around `Errors` / `Panics` sections. Those sources support the implementation judgment here, but the main conclusions in this pass are code-local rather than standards-driven.

## What Fourth Pass Sharpened

The third-pass bucket should stand, but only if it stays narrow. I would keep `Small surgical refactor` for inspection correctness, docs, and tightly bounded hardening. I would not treat the newly surfaced stress-lane semantics issue as part of that same bucket.

The most important overstatement in the current packet is the claim that cache poisoning is now the highest-ROI hardening target. The locks do still use `expect("... poisoned")` in the cache wrappers, so the panic path is real. But the expensive work is not done while the mutex is held. `get_or_build_structural_candidate_cache` and `get_or_build_exit_summary_cache` take the lock for lookup, drop it before compilation, and then reacquire it for insertion (`options_expression.rs:4227-4246`, `4265-4283`). That means “one panic during cache population poisons the process-wide cache” is too strong as written. A panic during the long compile path is not the common poison scenario, because the compile path runs outside the lock.

The more practical cache issue is different and was missed entirely by the current packet. Warm-cache hits still clone the entire cached value with `cache.get(&key).cloned()`, and misses also clone the full built value back into the map with `cache.insert(key, built.clone())` (`options_expression.rs:4231-4246`, `4269-4283`). These caches hold large dense vectors, so the current design is a compile-avoidance cache, not a shared-buffer cache. That matters twice: it keeps warm hits more expensive than the packet implies, and it means per-step feasibility timings report `0` compile milliseconds on hits even though hit-path cloning still happened (`options_expression.rs:2557-2561`, `2577-2581`, `2605-2608`, `2629-2632`, `2657-2660`, `2677-2680`). So the packet is pointing at the right subsystem, but the wrong symptom.

The current packet also underweights safer, more concrete inspection bugs that already exist.

One is `entry_lag_minutes`. The inspection row computes `requested_entry_timestamp`, but `entry_lag_minutes` is measured from `event.earliest_tradable_timestamp` rather than from that requested timestamp (`options_expression.rs:4439-4444`, `4518-4529`). With any non-zero tuple delay, the field reports intentional strategy delay as if it were data/execution lag. That is a current inspection semantics bug, and it has a very small blast radius.

A second is the fallback companion-entry value in `build_event_detail_row`. When detailed replay fails and the row falls back to raw entry pricing, the companion leg always uses `option_exit_price_for_row(...)` (`options_expression.rs:4494-4504`). That is correct for the short leg credit in a debit vertical, but wrong for a long pair, where the companion leg is also an entry-side buy. The executed-path tests do not catch this because they exercise successful detail replay, not the `OutcomeMissing` fallback path.

A third is event-path inspection under quote semantics. `option_entry_price_for_row`, `option_exit_price_for_row`, and `option_path_range_for_row` already distinguish `TopOfBookQuote` from aggregate-style semantics (`options_expression.rs:701-724`, `801-824`). But `build_event_path_points` ignores those helpers and always emits raw OHLC-derived points from the stored rows (`options_expression.rs:4582-4659`). That means the path chart can disagree with the execution semantics used everywhere else for quote-aware runs. This is a much more immediate inspection defect than redesigning tuple identity.

The biggest finding the current packet missed is the aggregate stress-lane asymmetry. `aggregate_entry_price` clearly defines distinct baseline, conservative, harsher, and extreme buy-side fill behavior (`options_expression.rs:659-665`). But `option_entry_price_for_row` hardcodes `AggregateStressMode::Baseline` for aggregate-style semantics (`options_expression.rs:701-709`). As a result, buy-side legs do not get stress-adjusted in those lanes. That affects long-single entry, long-pair entry, and the debit-vertical companion buy-to-close path (`options_expression.rs:728-759`, `762-789`). The sell-side paths do use stressed exit pricing. So the current stress model is not symmetric across buy and sell legs.

I would not “fix” that inside a surgical hardening pass, because doing so would rewrite holdout stress metrics, harsher/extreme metrics, shortlist ordering, finalist promotion, and CPU/GPU parity expectations. But it is too material to leave unmentioned. This looks more like either unfinished semantics or a deliberately narrower stress model that the file does not document.

The current packet also missed a larger diagnostics limitation. `ExitSummaryCache` stores outcomes as `Vec<Option<TradeOutcome>>` (`options_expression.rs:2076-2078`), and missing exit-path replay only surfaces later as `OutcomeMissing` in inspection rows (`options_expression.rs:4513-4516`). Meanwhile, the public invalid-reason enum advertises `SpreadTooWide`, `InsufficientFuturePath`, and `StaleOrMissingPath` (`options_types.rs:221-235`), but this engine never records those as actual outcome-failure reasons. The feasibility report can count invalid slots and missing outcomes, but it cannot attribute outcome-missing events with the same specificity it gives pre-entry candidate rejection. That is a real modeling gap, not just a docs issue.

Finally, the third-pass recommendation to make inspection “inspect-by-spec or stable-hash” as the next bounded improvement is too strong. `tuple_index` is already part of the public inspection DTOs in `options_types.rs:701-731`. That makes stable identity a cross-contract change touching app-core DTOs, the Tauri boundary, and likely UI/application logic. It may still be worth doing later, but it is not the next low-risk hardening step when there are still basic inspection-field correctness issues in the current API.

## Implementation Priority

### Safe-now hardening

The best immediate work is inspection correctness and documentation, not ranking redesign.

Fix `entry_lag_minutes` so it is measured relative to `requested_entry_timestamp`, not `earliest_tradable_timestamp`. Add a regression test with non-zero `delay_minutes`; the current inspection tests only cover zero-delay slices.

Make `fallback_companion_entry_price` structure-aware in `build_event_detail_row`. Debit vertical can keep its credit-side fallback behavior, but long pair should use entry-side pricing for the companion leg. Add an `OutcomeMissing` test for long-pair inspection so this path is actually pinned.

Make `build_event_path_points` semantics-aware for `TopOfBookQuote`, or document explicitly that inspection paths remain raw aggregate bars even when execution used quote-side semantics. I would prefer the semantics-aware fix, but either way this should be intentional and tested. Right now it is neither.

Add rustdoc to the four public blocking entry points. The third-pass packet was right that these APIs still hide too much behavior. The docs should say that inspection reruns prepare plus ranking, that `tuple_index` is current-rank-relative rather than stable identity, that inspection is baseline-lane replay, and that split fallback semantics exist.

### Larger redesign still deferred

Do not fold the aggregate stress-lane question into a small hardening PR. The asymmetry between stressed sell paths and baseline buy paths is the most consequential new finding in this pass, but it is not a safe tweak. It needs an explicit semantics decision and then a dedicated validation pass because it will change rankings and stress metrics.

Do not treat cache poison recovery as the main redesign target without first deciding whether the cache should remain by-value. If warm-hit cost matters, the more meaningful change is moving cache storage to shared immutable ownership, likely via `Arc`, and only then deciding whether to add singleflight behavior for concurrent callers. That is a worthwhile internal refactor, but it is benchmark-sensitive and not required for the immediate public-surface hardening.

Do not try to retrofit stable tuple identity until the current inspection fields are trustworthy. If identity redesign still matters after the inspection fixes land, handle it as an explicit contract change across DTOs and callers.

Do not attempt to solve outcome-missing diagnostics with a tiny patch. If the goal is to expose `InsufficientFuturePath` / `StaleOrMissingPath` / spread-style failures as first-class reportable reasons, `ExitSummaryCache` and the feasibility/inspection DTO surfaces need a richer modeled outcome state than `Option<TradeOutcome>`.

## Verification If Changed

If the safe-now changes above are made, the baseline verification should still include the existing focused options-expression test lane:

`cargo test -p ta_app_core options_expression::tests:: -- --nocapture`

If inspection semantics change, add targeted tests before touching broader benchmarking:

- a tuple-inspection test with non-zero `delay_minutes` that proves `entry_lag_minutes` excludes intentional delay
- an `OutcomeMissing` long-pair inspection test that proves the companion fallback uses entry-side pricing
- a quote-semantics event-path inspection test that proves path points align with quote-side execution semantics, or a test that explicitly locks in the documented aggregate-bar fallback if that behavior is retained

If anything touches the aggregate stress semantics, rerun the broader options-expression verification from the earlier packets, including the app-core options tests, the Tauri-side options command tests, and the options-expression bench compile lanes. That change is not inspection-only.

If cache ownership changes, compare cold and warm feasibility behavior separately. The current hit counters are not enough; warm-hit latency needs to be observed directly because the present cache design still copies large buffers on hit.

## Regression And Feature-Preservation Constraints

Preserve current ranking and finalist behavior in any small hardening patch. The inspection fixes above should not alter tuple ordering, stress scores, or backend selection.

Treat the aggregate stress-lane asymmetry as semantics-sensitive. If it is corrected, that is a behavior change, not a refactor. It will affect reported stress metrics, shortlist boundaries, and probably CPU/GPU comparison baselines.

Treat `tuple_index` as an active wire contract, not a private implementation detail. A stable-identity redesign is possible, but it is an IPC/API change because the DTOs already expose rank-relative indexes.

Treat cache values as accelerators only. If ownership moves to `Arc` or another shared form, preserve immutability and avoid introducing hidden mutable state that could leak across runs.

Treat outcome-failure reason modeling as a persistence and IPC concern. Extending the current `Option<TradeOutcome>` model will affect public DTO shape, artifact interpretation, and report logic, so it should be planned as a real contract change.

Finally, if quote-aware path points are fixed, remember that this is still a user-visible behavior change. Charts for quote-semantics slices may flatten from OHLC-like bars into single-price points, which is probably more correct, but still changes what downstream consumers see.

## Fourth-Pass Conclusion

`Bucket unchanged, but the scope needs correction.`

I would keep the fourth-pass action bucket at `Small surgical refactor`, but I would narrow it sharply. The current third-pass packet correctly says this file should be changed conservatively. It is wrong, though, to present cache-poison recovery and inspect-by-stable-hash as the obvious next two targets.

The highest-value immediate work is simpler: fix the current inspection-field inaccuracies, document the public blocking semantics, and add the missing tests that pin those behaviors. Those are real, present-tense defects with low blast radius.

The more important newly surfaced issue is the aggregate stress-lane asymmetry around buy-side fills. That is not a small hardening task. It is the sort of semantics decision that deserves its own pass because it can legitimately move rankings and reported robustness. Likewise, richer outcome-missing diagnostics and stable tuple identity are valid future improvements, but they are not the safest next step.

So the third-pass bucket can stand, but only if the follow-up work stays truly surgical. The live file now points to a different surgical target than the packet does.
