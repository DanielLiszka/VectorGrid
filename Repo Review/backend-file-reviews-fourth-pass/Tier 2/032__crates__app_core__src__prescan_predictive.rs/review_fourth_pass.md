# Fourth-Pass Review: crates/app_core/src/prescan_predictive.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/prescan_predictive.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/prescan_predictive.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/prescan_predictive.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/prescan_predictive.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass bucket should **stand**. This file is still a bounded, crate-private, CPU-only rescue heuristic, not a redesign candidate. The earlier packet was right to treat it as semantics-sensitive and to avoid casual scoring retunes.

What justified a fourth pass is that the third-pass framing stayed slightly too generic. It leaned on “policy surface / tolerant behavior” language, but the live source has narrower and more concrete edges than that framing suggests. The most important fourth-pass outcome is not a bucket change. It is a sharper split between:

- **safe-now hardening** that improves determinism, documentation, and tests without changing the product role of the heuristic, and
- **deferred redesign** that would materially change prescan breadth, score comparability, or failure behavior.

In particular, the live code still contains three details the current packet underweights:

1. duplicate or unsorted period domains can survive the common path and reach scoring unchanged,
2. `MAX_TIME_STEPS` is only an approximate sampling target, not a hard cap, and
3. the scoring contract is more specific than the current packet says: `ma_source` shapes the MA signals, but the target remains next **close** log return, and `long_only` removes bearish bars from the denominator instead of averaging them as zero.

Those are the live-source details worth preserving or hardening. They are more actionable than a broad push toward stricter `Err` behavior or stronger typing.

## Fourth-Pass Research Log

This fourth pass re-read the live file first, then re-checked the earlier packets only to see where the current packet had become too broad or too soft.

The external research was targeted and conservative:

- Rust API Guidelines on argument validation and type-guided contracts, to check whether earlier “tighten validation” recommendations were actually justified for a crate-private rescue helper.
- Rust standard-library docs for `Vec::dedup`, `sort_unstable`, and integer `div_ceil`, to verify the edge behavior behind the file’s current normalization and time-stride math.
- time-series split guidance, to confirm that the current next-step chronology is directionally correct and should not be casually replaced with anything that leaks future information.

That research did **not** support a stronger bucket. It did support a narrower conclusion: the third-pass packet was right about preserving the heuristic role, but it still missed a few code-local semantics that should be documented or pinned by tests before anyone “cleans this up.”

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The current packet got the two biggest judgment calls right.

First, it kept the file in `Small surgical refactor`, and that remains correct. Nothing in the live source justifies a redesign bucket. The helper is small, bounded, crate-private, and intentionally approximate.

Second, it treated the score as policy, not just plumbing. That is the right framing. Changing sampling density, denominator rules, or admission thresholds here changes which MA pairs even reach later optimization stages.

### What it missed or underweighted

#### 1. The duplicate-period problem is more concrete than the current packet states

The earlier packet correctly noticed order-dependence before normalization, but it underweighted how often normalization is skipped entirely.

At lines `14-15`, `sample_periods_by_stride` returns the raw input unchanged whenever `period_stride <= 1`. At lines `34-35`, `cap_periods_even` returns the raw vector unchanged whenever `len <= cap`. With `MAX_SAMPLED_PERIODS = 48`, that means many ordinary period-domain inputs can pass through without any sort or dedup step at all.

That matters more than “less deterministic than it appears.” It means duplicate domains can survive into the nested scoring loops at lines `133-178`, and the final output path at lines `191-194` performs no pair dedup. In other words, duplicate `(fast, slow)` seeds are possible if duplicate periods reach this function. That is not just cosmetic ordering drift. It can waste MA row work, reduce unique rescue breadth under `top_k`, and make candidate expansion look wider than it really is.

This is the most important concrete issue the current packet still missed.

#### 2. `MAX_TIME_STEPS` is not a hard cap

The current packet describes time-step subsampling as if `MAX_TIME_STEPS` were the effective bound, but line `109` computes:

`((n.saturating_sub(1)) / MAX_TIME_STEPS).max(1)`

That is floor-division logic, so the helper can still scan far more than `20_000` points. For lengths between roughly `20k` and `40k`, stride remains `1`, so the loop can still walk nearly the whole series. Even on much larger inputs the effective sample count can land above `MAX_TIME_STEPS`, not at or below it.

This does **not** mean the code is wrong. It does mean the constant is currently an approximate target, not a strict ceiling. That distinction matters because tightening it to a real cap with `div_ceil`-style logic would change which timestamps get scored, which in turn changes pair ranking. The current packet treated this as a bounded hot path, which is true, but it did not identify that the bound is softer than the constant name implies.

#### 3. The score semantics are sharper than “long_only zeroes short exposure”

The current packet said `long_only` zeroes short exposure, which is directionally true but incomplete.

At lines `155-164`, when `long_only` is set and the MA spread is bearish, `sign` becomes `0.0`, and that observation is skipped entirely. `obs` is incremented only when the bar contributes a non-zero sign and a finite next-step return. Combined with lines `171-174`, the long-only score is therefore a **conditional mean over bullish-spread observations**, not a flat-inclusive average that treats bearish bars as zero-return holds.

That is a meaningful contract difference. It makes long-only and long-short scores non-comparable in a stronger way than the current packet says, and it increases the chance that some candidates never reach `MIN_OBS` in long-only mode.

This should be documented and pinned by tests. It should **not** be “cleaned up” unless product policy wants a different heuristic.

#### 4. `ma_source` affects feature construction, not the target return series

At lines `98-104`, the helper builds MA signals from `params.ma_source`, but always computes `lr` from `close`. That means the heuristic is presently “predict next close-to-close return using MAs built on the requested source,” not “predict next return of the same source.”

That asymmetry may be intentional and may be exactly what the product wants. It is still a semantics-bearing choice that the current packet only hinted at via “alternate `ma_source` behavior.” It deserves a clearer statement because it is the kind of thing a well-meaning refactor could silently change.

#### 5. The file-local tests do not pin the real ranking contract

The current packet was right that test coverage is narrow, but it did not call out the most important weakness. The main happy-path test at lines `242-268` asserts that returned pairs are lexicographically nondecreasing. That is not the actual ranking contract. The actual sort at lines `184-189` is score descending, then fast period ascending, then slow period ascending.

So the current test is only indirectly validating behavior on one synthetic trend. It does not protect against duplicate pair emission, it does not pin the long-only denominator semantics, and it does not tell a future maintainer which ordering is contractual versus incidental.

### What the current packet overstates or frames too strongly

#### 1. Tightening empty results into hard errors is weakly supported and regression-prone

The first-pass packet pushed fairly hard on the `Ok(Vec::new())` versus `Err` split. That concern is understandable, but by the fourth pass it looks overstated as an implementation priority.

This helper is an additive rescue lane for composite prescan, not the primary optimization engine. Returning “no rescue seeds” is a low-friction fallback. Turning more branches into hard errors would make prescan more brittle and could abort optimization setup in cases where the current behavior simply declines to contribute extra candidates.

If diagnostics are needed, comments, debug assertions, or optional tracing are safer than promoting benign no-op cases into hard failures.

#### 2. Stronger typing is lower-value than pinning semantics

The earlier packet also emphasized weak typing around `&str` fields and tuple output. That is true in the abstract, but it is not the highest-value next step for this file.

This module is `pub(crate)`, the helper is used on an internal contract surface, and the live risks are not “someone cannot tell what a tuple means.” The real risks are duplicate-seed behavior, approximate subsampling, and underdocumented score semantics. Newtypes or wrapper structs would add churn before they solve the sharper issues.

#### 3. “Normalize before stride sampling” is directionally right but broader than necessary

The current packet recommended normalizing period ordering before stride sampling. That would work, but it is a somewhat broader behavioral choice than the current evidence requires.

The narrower requirement is simpler: by the time scoring starts, sampled fast and slow domains should either be known caller-normalized invariants or be made unique locally. If the repo already guarantees sorted, unique period sets upstream, then debug assertions and tests may be enough. If it does not, then a local sort/dedup just before matrix construction is the more defensible hardening point.

That is a smaller, safer conclusion than “always normalize before stride selection.”

## Implementation Priority

### Safe-now hardening

1. **Document the real score contract where the function is defined.**
   Add a short doc comment or file note that says, explicitly:
   - this is a rescue-seed heuristic, not validation evidence,
   - MAs are built from `ma_source`,
   - the target remains next close log return,
   - `long_only` is a conditional bullish-only average because bearish/flat bars do not enter the denominator,
   - time downsampling is approximate rather than a strict `MAX_TIME_STEPS` cap.

2. **Pin the period-domain uniqueness story with tests before changing logic.**
   Add a narrow regression test for duplicate or unsorted `fast_periods` / `slow_periods` on the `period_stride <= 1`, `len <= MAX_SAMPLED_PERIODS` path. The goal is to make one of two contracts explicit:
   - either duplicate inputs are invalid and should be caught by a debug assertion or caller contract, or
   - duplicate inputs are tolerated but must not yield duplicate output pairs.

3. **Prefer the narrowest fix for duplicate-seed emission.**
   If the code is changed now, the safest local hardening is not a scoring redesign. It is to ensure sampled domains are unique before entering the nested loops, or at minimum to dedup the final `(fast, slow)` output. That addresses the concrete duplicate-seed problem without changing the heuristic’s scoring formula.

4. **Strengthen tests around the actual ranking contract.**
   Replace or supplement the current “sorted pairs” assertion with tests that pin what really matters: uniqueness, `fast < slow`, bounded `top_k`, and the intended tie-break behavior if a deterministic fixture can be built.

### Larger redesign still deferred

1. **Do not switch approximate subsampling to a strict cap without evidence.**
   Changing line `109` to `div_ceil`-style logic would produce a truer cap, but it would also change which timestamps are observed on long series and therefore change rankings. That is a behavior change, not a cleanup.

2. **Do not change `Ok(Vec::new())` into broader `Err` behavior as a local refactor.**
   That would change composite rescue fault tolerance, and the current file alone does not justify it.

3. **Do not retune the denominator or target-return semantics casually.**
   Using `ma_source` returns instead of close returns, or averaging flat periods as zero in long-only mode, would create a different heuristic. Those are product-level decisions and should remain deferred.

4. **Do not spend this pass on type-wrapper churn.**
   A pair struct or stronger source/MA typing might improve readability later, but it is not the highest-value intervention on the current risk surface.

## Verification If Changed

If this file changes, the minimum verification set should still include:

- `cargo test -p ta_app_core prescan_predictive::tests:: -- --nocapture`
- `cargo test -p ta_app_core composite_cpu_prescan_predictive_rescue_can_expand_candidates -- --nocapture`
- `cargo test -p ta_app_core composite_mo_gpu_prescan_supports_multiple_ma_pairs -- --nocapture`

If the change touches period normalization, duplicate handling, denominator logic, or time-step sampling, add focused file-local tests for those exact branches rather than relying on the current trending happy-path fixture.

If line `109` changes or any scoring formula changes, treat that as both a correctness and throughput-sensitive edit. Re-run the relevant composite optimization workflow or CPU perf lane before accepting it, because this helper indirectly changes candidate admission into later optimization stages.

## Regression And Feature-Preservation Constraints

Preserve the current role of this helper as an **additive rescue heuristic**. It should not become a heavyweight optimizer or a hard-failure gate for composite workflows.

Preserve chronology-respecting next-step evaluation. There is no immediate leakage problem in the current structure, and any more ambitious predictive rewrite belongs in a separate evidence-backed change.

Be explicit about the indirect downstream risk surface:

- there is **no direct GPU ABI risk** in this file,
- there **is** indirect GPU-route risk because Composite MO uses the emitted seed pairs to shape later GPU prescan work,
- there is **validation-surface risk** because changed seed admission affects which candidates ever reach later validation and ranking stages.

If period normalization is hardened locally, decide and document whether the repo intends caller-normalized period domains to be an invariant. Silent local normalization is reasonable if unsorted inputs are considered “tolerated but non-semantic.” It is less reasonable if the upstream order is intentionally meaningful. The current file does not prove that upstream order is meaningful, so any change here should state the chosen contract plainly.

Finally, do not let a cleanup accidentally erase the current semantics that matter most:

- `ma_source` shapes the signal, not the target return,
- `long_only` changes the denominator by skipping bearish bars,
- the current time-step cap is approximate,
- duplicate seeds are currently possible unless uniqueness is enforced somewhere on the scoring boundary.

## Fourth-Pass Conclusion

`Bucket unchanged.` The third-pass `Small surgical refactor` call remains correct.

What the current packet got right is the big picture: this file is a bounded, semantics-sensitive rescue heuristic that should not be redesigned casually.

What it still missed is more important than another round of generic cleanup advice. The live source has a concrete duplicate-seed edge on common `period_stride <= 1` paths, a softer-than-advertised `MAX_TIME_STEPS` bound, and sharper scoring semantics than the current packet spells out. Those are the parts to harden.

So the fourth-pass recommendation is narrower than the first-pass packet and sharper than the third-pass packet:

- **safe now:** document the score contract, pin duplicate/unsorted-domain behavior with tests, and enforce uniqueness at the narrowest sensible point,
- **defer:** stricter error behavior, strict time-cap math, denominator redesign, target-return redesign, and type-wrapper churn.

That keeps the existing bucket, preserves current behavior where it is clearly intentional, and focuses the next change on the highest-value local hardening rather than the most abstract cleanup story.
