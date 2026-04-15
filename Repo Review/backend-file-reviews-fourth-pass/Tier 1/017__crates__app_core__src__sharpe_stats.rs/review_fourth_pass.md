# Fourth-Pass Review: crates/app_core/src/sharpe_stats.rs

- Review date: 2026-04-15
- Source file: `crates/app_core/src/sharpe_stats.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: Reviewed

## Why This File Got A Fourth Pass

This file got a fourth pass because the third-pass packet changed the bucket from `Document only` to `Small surgical refactor`, and its justification was aggressive enough to deserve adversarial review. The core third-pass claim was that the `0.5` fallback in `probabilistic_sharpe_ratio` was itself the main contract flaw.

After reopening the live source, the nearby validation caller, and the underlying statistical references, the bucket can stand, but not for the reason the third-pass packet gave. The file does need a small, surgical change. The real issue is a correctness bug in the private inverse-normal helper that materially distorts `expected_max_sharpe`, and therefore `deflated_sharpe_ratio`, once the trial count is large enough to enter Acklam’s tail branches. The sentinel-behavior debate is still worth documenting, but it is not the strongest live issue in this file.

That distinction matters. A surgical correctness fix with targeted tests is justified now. A broader redesign of sentinel semantics is not yet justified from the evidence in this bundle.

## Fourth-Pass Research Log

The fourth pass re-read the live `sharpe_stats.rs` implementation first, then challenged the third-pass packet against the actual downstream caller in `validation_pipeline.rs`. That caller context matters because this file is not the full validation policy surface by itself.

The statistical formulas were then cross-checked against Bailey / López de Prado references for the Probabilistic Sharpe Ratio, Deflated Sharpe Ratio, and Minimum Track Record Length, plus an Acklam reference for the inverse normal CDF approximation. That comparison was important for separating “document this contract better” from “the code is numerically wrong right now.”

This pass also checked the public exposure surface in `lib.rs`. That confirmed the Sharpe helpers are not buried implementation details. Even though `normal_inv_cdf` is private, its behavior propagates into public Sharpe-family helpers through `expected_max_sharpe` and `deflated_sharpe_ratio`.

The most important result from that research pass was narrow and concrete: the live `normal_inv_cdf` tail-branch signs do not match the Acklam algorithm. The lower-tail branch currently applies a leading minus where Acklam does not, and the upper-tail branch omits the leading minus where Acklam does include it. In other words, the tail signs are swapped.

## What Fourth Pass Sharpened

### 1. The current third-pass bucket should stand, but its justification should change

The third-pass packet got two things right.

First, it correctly treated this file as semantics-sensitive support code rather than as a performance target. That is still the right posture.

Second, it correctly treated formula swaps and sentinel changes as validation-sensitive behavior, not harmless cleanup.

But its central claim is overstated. The packet framed the `0.5` fallback in `probabilistic_sharpe_ratio` as the main reason this file needed a refactor. The live source does not support that as the strongest issue.

The actual caller shown in `validation_pipeline.rs` clamps `sample_obs` to at least `2` before calling the PSR / DSR wrapper (`obs = ...unwrap_or(2).max(2)` at lines 288-290). That means the specific `samples < 2` path that the third pass focused on is not even the normal path in the provided validation flow. The third pass also compared the PSR helper’s scalar probability output to `minimum_track_record_length`’s `Option<usize>` output as if they were parallel invalidity models, but they are not used the same way downstream. `minimum_track_record_length` absence flows into approval logic through `unwrap_or(false)` at lines 454-456, which is a very different contract from a scalar probability helper.

So the bucket can remain `Small surgical refactor`, but the justification should pivot away from the sentinel argument and toward the real numerical defect.

### 2. The third-pass packet missed a more important correctness bug: `normal_inv_cdf` has its tail signs reversed

This is the most important fourth-pass finding.

In the live source, the lower-tail branch at lines 62-65 returns a negated rational expression, and the upper-tail branch at lines 66-69 returns the same expression without negation. Acklam’s algorithm does the opposite: lower tail without the leading minus, upper tail with the leading minus.

That is not cosmetic. It means the helper returns positive quantiles for tiny probabilities and negative quantiles for probabilities near one. The central branch is fine; the tail branches are sign-inverted.

Because `expected_max_sharpe` calls `normal_inv_cdf(1 - 1/N)` and `normal_inv_cdf(1 - 1/(Ne))` at lines 127-130, this sign bug directly corrupts the expected-maximum threshold used by `deflated_sharpe_ratio` at lines 142-143. Bailey’s own DSR appendix and sample code show that this expected-maximum threshold should increase as the number of trials grows. The live implementation can instead push it below `sr_mean`, and eventually below zero, once those inputs cross into the upper tail.

A quick local check on the live formula makes the impact concrete:

| `num_trials` | live `expected_max_sharpe(0.4, 0.2)` | reference-consistent value |
| --- | ---: | ---: |
| 10 | 0.7149 | 0.7149 |
| 17 | 0.2990 | 0.7656 |
| 42 | -0.0417 | 0.8417 |
| 100 | -0.1061 | 0.9061 |

The break starts earlier than it first appears. Around `N = 17`, one of the two quantiles has already crossed into the faulty upper-tail branch. By `N = 42`, both quantiles are in the wrong-sign region. That means the DSR null threshold is systematically too low for moderate and larger trial counts, which inflates DSR.

This also exposes a test-gap the prior packets missed. The existing test at lines 198-202 already exercises `expected_max_sharpe(42, ...)`, which is firmly inside the broken regime, but it only checks determinism. So the faulty branch is currently covered without being validated.

This is the strongest evidence-backed reason the file needs a surgical change now.

### 3. The caller context weakens the third-pass case for changing PSR sentinel semantics right now

The third-pass packet underweighted caller context in two ways.

The first is the `samples < 2` point already noted above. In the provided validation flow, that path is not the ordinary path because the pipeline clamps observations to at least `2` before delegating to PSR / DSR computation.

The second is the boundary between `sharpe_stats.rs` and `validation_stats.rs`. The validation pipeline stores PSR and DSR as `Option<f64>` and applies hard gates with `unwrap_or(0.0)` at lines 375-386. That strongly suggests the wrapper layer, not necessarily the low-level math helper, is where absence semantics are meant to be normalized for validation policy. The third-pass packet recommended changing the low-level helper contract without first proving that the wrapper contract was wrong.

That recommendation is too strong for the evidence available here. Changing `probabilistic_sharpe_ratio` to return `Option<f64>` or `NaN` would be a public-surface contract change, and it would be doing so without the missing `validation_stats.rs` evidence that should be inspected first.

So the fourth pass does not support the third pass’s “make invalid PSR semantically distinct now” recommendation as a safe-first change.

### 4. The most valuable non-bug hardening is still documentation and focused edge-case tests

The earlier packets were directionally right that this file is underdocumented. The fourth pass sharpens where that documentation matters most.

The parameter named `kurtosis` is a real semantic trap. The implemented formula expects raw Pearson kurtosis, where Normal returns correspond to `3`, not excess kurtosis, where Normal returns correspond to `0`. The code itself shows that expectation: it defaults non-finite kurtosis to `3.0` and floors finite values at `1.0` (lines 90-94 and 164-168). Passing excess kurtosis here would materially bias PSR and MinTRL.

That is a better documentation target than the third-pass focus on the neutral `0.5` alone. This file should also document that `sr_hat` and `benchmark_sr` need to be in the same periodicity / scale, because the statistical references treat these quantities as frequency-consistent Sharpe values.

The tests are also too light exactly where the live bug was able to hide. There is no sign test for the inverse CDF tails, no tail round-trip test, no monotonicity test for `expected_max_sharpe` as `num_trials` grows, and no explicit regression test for the invalid / neutral sentinel outputs that earlier packets kept discussing.

## Implementation Priority

### Safe-now hardening

The first change should be a narrow correctness fix in `normal_inv_cdf`: swap the tail signs to match Acklam, and do nothing else to the approximation family, coefficients, or clamping behavior in the same patch.

Immediately after that, add targeted tests that would have caught this defect the first time:

1. a lower-tail sign test such as `normal_inv_cdf(1e-6) < 0`;
2. an upper-tail sign test such as `normal_inv_cdf(1.0 - 1e-6) > 0`;
3. a round-trip test for representative tail and central probabilities;
4. an `expected_max_sharpe` monotonicity test over increasing `num_trials` for positive `sr_std`;
5. an explicit edge-case test for the current PSR fallback behavior, so the neutral-sentinel policy is frozen if it remains intentional.

After the correctness fix and tests, add rustdoc to the public helpers clarifying three things that are easy to misuse today: the formulas are Sharpe-family statistical estimators rather than generic math utilities, `kurtosis` means raw kurtosis rather than excess kurtosis, and invalid or insufficient inputs are normalized into sentinel outputs rather than surfaced as errors.

### Larger redesign still deferred

Do not change the PSR / DSR return-shape semantics in this pass. The current bundle does not include `validation_stats.rs`, while `validation_pipeline.rs` clearly expects an `Option<f64>` policy layer above this file. That is exactly the kind of boundary where a seemingly “cleaner” low-level API change can cause avoidable regressions.

Do not swap the current approximation family for a different numerical dependency in this pass either. The concrete defect is not that the repo uses Acklam / Abramowitz-Stegun style approximations. The defect is that one Acklam transcription is wrong in the tail branches. Fix the transcription bug first.

Do not widen `SharpeStats`, rename public functions, or introduce typed statistical wrappers in the same patch. Those are larger API-shape questions and can wait until the live validation wrapper surface is reviewed end to end.

## Verification If Changed

At minimum, run the file-local Sharpe tests:

`cargo test -p ta_app_core sharpe_stats::tests:: -- --nocapture`

Then run the adjacent validation tests that consume these outputs:

`cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`

and, if present in the workspace,

`cargo test -p ta_app_core validation_stats::tests:: -- --nocapture`

Because the inverse-CDF fix will lower the DSR null threshold correction error, representative validation-path coverage should also be rerun for cases with more than a trivial number of candidates. This is the main behavior that can move after the fix.

One subtle but important expectation for verification: `minimum_track_record_length` should not materially change from the tail-sign fix, because it squares the `z` quantile at lines 170-172. If MinTRL changes in the same patch, something broader than the intended fix was altered.

## Regression And Feature-Preservation Constraints

Keep the bug fix surgical. The expected output drift here is in DSR-related values for moderate and larger trial counts. That drift is desirable because the current outputs are mathematically wrong, but it is still a validation-semantic change and can alter candidate gating, ordering, and reported confidence.

Do not combine the inverse-CDF fix with a PSR sentinel redesign. Those are separate risk classes. The former is a concrete correctness repair. The latter is a public contract question that still needs wrapper-level review.

Do not combine the inverse-CDF fix with a `T` versus `T - 1` formula rewrite, a different normal approximation, or a change to kurtosis/skewness sanitization. The earlier packets were right that those changes would create extra numerical drift without a clear local bug.

There is no GPU ABI, IPC, or persistence-format risk in the narrow bug fix itself. The risk is validation behavior, not binary interface shape. That remains true only if the patch leaves signatures, serialization shapes, and public struct layout unchanged.

## Fourth-Pass Conclusion

The current third-pass bucket can remain `Small surgical refactor`, but the third-pass packet’s main argument should not survive unchanged.

What it got right was the general posture: this file is semantics-sensitive, publicly exposed, and not a place for casual numerical edits.

What it missed was more important than what it found: `normal_inv_cdf` has Acklam tail signs reversed, which materially breaks `expected_max_sharpe` and inflates `deflated_sharpe_ratio` once trial counts are large enough to hit the tail branches. That is a real correctness bug, and it is the strongest evidence-backed reason for a surgical change.

What it overstated was the need to redesign the `0.5` PSR fallback immediately. The provided caller context weakens that case, and changing the low-level helper contract now would be riskier than the packet acknowledged.

So the conservative fourth-pass call is:

keep the `Small surgical refactor` bucket, fix the inverse-CDF tail sign bug first, add tail-focused regression tests, and add the missing documentation around kurtosis semantics and sentinel behavior. Defer the broader invalidity-contract redesign until the `validation_stats` wrapper layer is reviewed directly.
