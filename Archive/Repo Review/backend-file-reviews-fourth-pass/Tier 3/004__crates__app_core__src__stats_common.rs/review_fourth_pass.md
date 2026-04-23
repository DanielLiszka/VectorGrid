# Fourth-Pass Review: crates/app_core/src/stats_common.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/stats_common.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/stats_common.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/stats_common.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/stats_common.rs.review.md`
- Third-pass action bucket: `Document only`
- Fourth-pass action bucket: `Safe-now hardening`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file is still tiny, but it is also still a public module surface: `lib.rs` exports it as `pub mod stats_common` at line 64. That matters because small numerical helpers are easy to read as “standard statistics” even when they actually encode local conventions. The third-pass packet had the right instinct on one major point — do not redesign this module casually — but it also looked partially templated. Its references to “tolerant parsing” and “heuristic selection rules” do not match the live file at all.

That mismatch made a fourth pass worthwhile. The live source is small enough that a final review can be more exacting: it can separate genuinely harmless simplicity from small but real contract holes. After reopening the file, the main conclusion is that the third-pass `Document only` bucket is slightly too passive. The file still does not justify algorithm churn, but it does justify a very small hardening patch that adds explicit contract docs and a few targeted tests around current behavior.

## Fourth-Pass Research Log

I re-read the live source first, then compared it against the third-pass packet and earlier packet history. The key source anchors are `quantile` at lines 1-10, `median` at lines 13-15, `worst_min` at lines 17-27, `worst_max` at lines 29-39, `median_u32` at lines 41-48, `worst_min_u32` at lines 50-52, and the file-local tests at lines 54-77.

I then checked a small set of primary Rust references to avoid guessing about edge semantics:

1. [`f64::clamp`](https://doc.rust-lang.org/std/primitive.f64.html#method.clamp) documents that `NaN` input remains `NaN` rather than being coerced into the interval. That matters directly for `quantile`, which does `q.clamp(0.0, 1.0)` before computing the index.
2. The Rust Reference section on [numeric casts](https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast) documents that float-to-integer casts round toward zero and that `NaN` casts to `0`. Combined with the live source, this means `quantile(sorted, f64::NAN)` currently returns index `0` for any non-empty slice.
3. [`f64::round`](https://doc.rust-lang.org/std/primitive.f64.html#method.round) documents that halfway cases round away from `0.0`. Combined with `q * (len - 1)`, that explains why `median(&[1.0, 2.0, 3.0, 4.0])` resolves to the upper middle element rather than an average or lower middle.
4. [`slice::select_nth_unstable`](https://doc.rust-lang.org/std/primitive.slice.html#method.select_nth_unstable) documents the standard-library nth-element alternative: in-place, no allocation, `O(n)`. That confirms the earlier packets were directionally right to mention it as a possible future performance tool, but not as a justified change today.
5. [`f64::total_cmp`](https://doc.rust-lang.org/std/primitive.f64.html#method.total_cmp) documents the total-order escape hatch for floating-point sorting. This is important because any future redesign that tries to sort `quantile` inputs internally is not a trivial cleanup; it has to choose a NaN and signed-zero ordering policy.

I also ran a very small local `rustc 1.81.0` sanity check on an extracted snippet to confirm the composed `clamp` plus cast behavior. That check verified the expected live-language result: `f64::NAN.clamp(0.0, 1.0)` stays `NaN`, and the subsequent cast-to-`usize` resolves to `0`. This was only an isolated semantics check, not a repo-wide verification run.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should change.

I would not escalate this file into redesign work, performance work, or API-shape work. But I also would not leave it at `Document only`. The live code has a few behavior edges that are cheap to lock down and too easy to misread if they remain undocumented and untested. The right fourth-pass bucket is `Safe-now hardening`: documentation plus a handful of targeted tests, with no algorithm change.

### What the third-pass packet got right

The third-pass packet was right not to recommend `select_nth_unstable`, interpolated quantiles, `Option`/`Result` returns, or any other semantic rewrite. Nothing in the live source shows a correctness bug serious enough to justify changing the file’s behavior under existing callers. It was also right that tolerant behavior here should not be tightened casually, because even tiny numeric helper changes can leak into validation summaries and downstream reporting.

It was also correct that this file is not a hot-path or CUDA/runtime-critical target. The repo guidance provided in this bundle places `stats_common.rs` among shared validation/statistics primitives rather than GPU ABI or persistence surfaces, so the real risk is semantic drift, not throughput loss or device/runtime breakage.

### What the third-pass packet missed or underweighted

The most important miss is that `quantile` has two distinct preconditions or policies, not one. The earlier packets focused on the rounded-index convention, which is real, but they underweighted the simpler and more failure-prone contract: the input slice must already be sorted in the intended ascending order. The parameter name `sorted` hints at that, but the public helper still has no rustdoc saying so. For a public stats helper, that is a genuine contract hole.

The second miss is that the file has stronger even-length median behavior than the prior reviews stated. `median` delegates to `quantile(sorted, 0.5)` and, because `round()` rounds halfway cases away from zero, even-length `f64` slices resolve to the upper middle element. `median_u32` also returns the upper middle element via `ys[len / 2]`. That cross-type consistency is good, but it is not currently documented and is not pinned by tests.

The third miss is more significant than the earlier packets made it sound: `worst_min` and `worst_max` do not merely “fall back on empty or non-finite-only inputs.” Their actual behavior is narrower and stranger than that. `worst_min` returns `0.0` whenever the selected minimum is non-finite, which means any `NEG_INFINITY` in the slice collapses the result to `0.0` even if finite values are present. `worst_max` behaves symmetrically for `INFINITY`. By contrast, the opposite-direction infinity is ignored by the fold because it never wins the comparison. The current tests only lock `NaN` behavior, not this infinity behavior.

The fourth miss is the `q = NaN` path. Because `clamp` preserves `NaN` and Rust float-to-int casts map `NaN` to `0`, `quantile(sorted, f64::NAN)` currently returns the first element of a non-empty slice. That is a real live behavior, but it does not read like an intentionally chosen contract. The earlier packets did not mention it at all.

The fifth miss is test coverage around clamping. The live implementation explicitly clamps `q` into `[0.0, 1.0]`, but the current tests only cover exactly `0.0`, `0.5`, and `1.0`. They do not lock the actual out-of-range behavior for values such as `-1.0` or `2.0`.

### What the third-pass packet overstated or weakened

The third-pass packet weakens itself by describing this module as if it were a compatibility/parsing surface. It is not. There is no parsing here. There are no heuristics beyond simple numeric selection rules. That framing reads like packet carryover from a different file, and it obscures the actual scope of the work. The right fixes here are file-local and precise: documentation and tests.

It also overstates the need for broad “representative heuristic/compatibility fixtures.” For this module, file-local unit tests are the right default evidence. If the code stays behavior-preserving, there is no reason to drag in larger workflow fixtures just to bless a tiny stats-helper clarification patch.

## Implementation Priority

### Safe-now hardening

The safe-now work is small and behavior-preserving.

First, add rustdoc that states the contracts the code already relies on. `quantile` should say that the input must already be sorted, that `q` is clamped into `[0.0, 1.0]`, that the helper returns a rounded-index selection rather than an interpolated estimator, and that empty input returns `0.0`. `median` and `median_u32` should say that even-length inputs resolve to the upper middle element under the current policy. `worst_min` and `worst_max` should say what the implementation actually does: they return `0.0` when the selected extreme is not finite, rather than promising to ignore every non-finite input.

Second, add a very small test expansion that pins the current contract without changing it. The highest-value additions are:

- out-of-range `q` clamping for `quantile`, for example `q < 0.0` and `q > 1.0`
- even-length median tests for both `median` and `median_u32`
- infinity-behavior tests for `worst_min` and `worst_max`, including both the poisoning direction and the ignored opposite direction

Third, decide how explicit to be about `q = NaN`, but do not silently ossify it by accident. The smallest conservative move is documentation that `q` is expected to be finite, while leaving runtime behavior unchanged. A debug assertion could be reasonable if the maintainers want tighter development-time checks, but that is already a policy decision, not mere documentation, so it should be deliberate.

This is the core reason the bucket changes. The file does not need algorithm work, but it does need enough docs and tests that future edits cannot casually move these edge conventions.

### Larger redesign still deferred

The fourth pass does **not** justify changing the quantile convention, changing median semantics, or replacing forgiving helpers with `Option`/`Result` returns. Any of those would be a downstream semantic change, not a cleanup.

It also does **not** justify rewriting `median_u32` around `select_nth_unstable` today. The standard library does provide that tool, and it is algorithmically better on paper, but the current function takes `&[u32]`. That means any local rewrite still clones the input or forces an API change to a mutable slice. Without evidence that this helper is hot, the current implementation remains the right simplicity/performance tradeoff.

Finally, do not “improve” `quantile` by making it sort internally unless there is a deliberate redesign. Sorting floating-point inputs requires an explicit total-order policy such as `total_cmp`, and choosing that policy would be a semantic expansion of the helper, not a free correctness win.

## Verification If Changed

If the work is limited to docs plus file-local tests, the primary verification target should stay narrow:

- `cargo test -p ta_app_core stats_common::tests:: -- --nocapture`

If any actual behavior changes are proposed — quantile convention, median convention, non-finite fallback semantics, or NaN handling — then verification should expand to representative validation/statistics consumers that summarize percentile-like or worst-case metrics, because that is where semantic drift would show up.

This fourth pass itself was bundle-based and evidence-driven. I did not run a full workspace test or app-level gate from the provided packet.

## Regression And Feature-Preservation Constraints

There is no direct IPC, persistence, or GPU ABI risk in this file. The meaningful risk is validation and reporting drift: this module sits on a shared stats-helper surface, so even tiny semantic changes can alter summary values that other code treats as stable.

Preserve the following unless maintainers intentionally want a broader behavior migration:

- the current upper-middle median policy for even-length inputs
- the current empty-input fallback values (`0.0` and `0`)
- the current rounded-index quantile convention
- the current “selected extreme must be finite” behavior in `worst_min` and `worst_max`

Be especially careful with any proposal that rewrites non-finite handling. Changing `worst_min([1.0, NEG_INFINITY])` from `0.0` to `1.0`, or `worst_max([1.0, INFINITY])` from `0.0` to `1.0`, may look more mathematically elegant, but it is a real contract change and could affect downstream robustness summaries or sentinel-sensitive code.

Also avoid accidentally freezing `q = NaN` behavior through tests unless the maintainers truly want “first element” to become the public contract. Right now that edge looks more accidental than intentional.

## Fourth-Pass Conclusion

The third-pass review had the right anti-redesign instinct, but its `Document only` bucket should not stand.

The better fourth-pass call is `Safe-now hardening`. This remains a small patch: add explicit rustdoc, add a few precise tests, and leave the algorithms alone. The file does not warrant performance work, API redesign, or stricter return types. What it does warrant is closing the gap between the live semantics and the written contract, especially around sorted-input expectations, even-length medians, infinity handling, and the currently undocumented `q = NaN` edge.

That is the conservative, evidence-backed way to make this file more public-review-ready without risking avoidable regressions.
