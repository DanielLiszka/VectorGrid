# Fourth-Pass Review: `crates/app_core/src/cscv_pbo.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/cscv_pbo.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file deserved a fourth pass because the current third-pass packet drifted too far toward “this is mostly deliberate policy, preserve it unless product policy changes” and stopped separating three different classes of concern that should stay distinct in a public validation helper:

1. **safe-now public-contract hardening** that can be improved without changing the statistical method,
2. **methodology conventions** that are real but not clearly wrong, and
3. **larger statistical redesign choices** that would change downstream validation, reporting, and ranking behavior.

The third-pass bucket itself still looks right. The packet does not. The live source remains a compact, shared CSCV/PBO helper with bounded runtime and no obvious need for a larger redesign, but it still has underdocumented public semantics, a shared-helper observability leak, and a few statistically meaningful edge behaviors that should be named explicitly instead of being waved away as generic “policy.” The file is also publicly exposed through `app_core` (`context__01__crates__app_core__src__lib.rs:17`, `164`), so small-looking choices here are compatibility-sensitive.

## Fourth-Pass Research Log

I re-read the live source, the prior three packets, and the small amount of local repo context provided with this bundle. I also rechecked the method against outside references because this file is methodology-sensitive and the prior packets disagreed too confidently in places.

The most important external cross-check was Bailey et al., *The Probability of Backtest Overfitting*. Algorithm 2.3 states the relative OOS rank as `r / (N + 1)` before taking the logit, but the earlier definition of PBO in the same paper describes overfitting in terms of ranking below the median OOS, written as `r_n < N / 2`. Those are not the same odd-`N` boundary unless a rank-indexing convention is made explicit. That means the first-pass packet’s “clear normalization bug” framing was stronger than the evidence supports.

I also checked the commonly used R `pbo` package implementation. Its source uses `which.max(r)` for IS winner selection, `rank(r_bar)[n_star]` for OOS ranking, and `omega_bar_c <- os_rank / length(r_bar)`, which is materially closer to the current Rust implementation than to a literal `r / (N + 1)` rewrite. That does not prove the Rust file is correct, but it does show that the current normalization and tie behavior are not obviously accidental.

Finally, I rechecked Rust documentation guidance for public APIs. The relevant point for this file is simple: public items should document assumptions and behavior rather than forcing consumers to reverse-engineer them from implementation details. That guidance matters here because the DTOs and entrypoint are public, re-exported, and serialized, while several important semantics are currently only discoverable from source.

## What Fourth Pass Sharpened

### Bucket assessment

The **third-pass bucket should stand**: this is still a `Small surgical refactor`, not a redesign packet. The safe-now work remains narrow and mostly contract hardening. What changed in this fourth pass is not the bucket, but the precision of what belongs inside it.

### What the current third-pass packet got right

The current third-pass packet was right about three things.

First, this file is not the right place for speculative performance work. The runtime is bounded by sanitized slice counts (`source__crates__app_core__src__cscv_pbo.rs:66-72`) and capped combination counts (`300-309`), and the main scratch buffers are reused inside the combination loop (`337-349`). There is no credible throughput-driven reason to broaden scope.

Second, broad redesign ideas still belong in the deferred bucket. The file is shared validation infrastructure, not strategy-local code, and changing score semantics, sampling methodology, or rank normalization would move downstream validation outcomes rather than merely “clean up” code.

Third, the current packet was right to resist converting tolerant edge handling into hard failures. The current entrypoint already has a stable pattern of returning structured skipped results for low-information paths and `Err(String)` only for malformed input or cancellation (`247-297`, `311-331`, `353-355`). Nothing in the live source justifies a panic-oriented rewrite.

### What the current third-pass packet missed or underweighted

The main problem with the current third-pass packet is that it flattened too many live behaviors into “deliberate policy” and therefore failed to distinguish what is safe to harden now from what truly requires coordinated methodology work.

#### 1. The normalization issue is real, but the prior “clear bug” framing was overstated

The live code computes `omega` as `os_rank / candidates` and stores the same convention in per-candidate rank fractions (`389-392`, `406-408`). The first-pass packet treated that as a likely correctness bug because Bailey et al. Algorithm 2.3 spells the relative rank as `r / (N + 1)`. That was a useful challenge, but not a sufficiently settled conclusion.

Why the earlier claim is too strong:

- the same paper earlier defines PBO in terms of ranks below the median OOS, which does not map cleanly to the Algorithm 2.3 logit threshold for odd `N` unless the rank convention is spelled out;
- the reference R `pbo` implementation uses `os_rank / N`, `which.max`, and default `rank()` behavior, which broadly matches this Rust helper;
- the live Rust code even documents the IS tie-breaking intent inline with `// which.max: first max index` (`378`).

So the sharper conclusion is:

- **do not treat `N` vs `N + 1` as a safe-now local bug fix**;
- **do treat it as a methodology convention that must be documented and pinned by tests**;
- **if the team wants literal Bailey-Algorithm-2.3 alignment, treat it as a coordinated validation/reporting change**, because it would alter `lambda_*`, `oos_rank_frac_p50`, hard-gate behavior downstream, and possibly user-facing report ordering.

This is the clearest place where the current third-pass packet was too weak and the earlier first-pass packet was too strong.

#### 2. The skipped-path tuple contract is still a real public-surface footgun

The normal path returns one `CscvCandidatePboStats` per candidate (`432-454`). The skipped paths for too few candidates, too-small `slice_bars`, or zero combinations return `Vec::new()` (`247-289`, `311-331`). Because the function signature is positional and public (`236-243`), that shape change is easy for callers to misread.

The current third-pass packet did not revisit this, but it still matters. Since the module is re-exported publicly, this is not just internal convenience.

The conservative fourth-pass view is narrower than the first pass here: **documenting and testing the empty-vector contract is safe now; changing it to placeholder-filled candidate rows is not automatically safe-now**, because that would change serialized/IPC shape and any consumer that currently interprets “skipped” as “no candidate payload.”

#### 3. The quantile helper is a bigger contract issue than the current packet recognized

`quantile` is currently not a midpoint median or interpolated quantile. It chooses a single order statistic by rounding an index into the sorted slice (`169-178`). That means:

- `p50` for a two-element series returns the upper element, not the midpoint,
- `p50` for an even-length series returns one observed element rather than the average of the two middle values,
- the same convention drives `lambda_p25`, `lambda_p50`, `lambda_p75`, and every candidate-level `oos_rank_frac_p50` / `oos_metric_p50` (`424-428`, `445-452`).

This is not necessarily wrong, but it is definitely underdocumented and more surprising than the current third-pass packet implied. Because those fields are explicitly labeled as quantiles and medians, this behavior should either be documented and test-pinned or changed in a coordinated semantics pass. It should not stay implicit.

This was the most important missed edge in the current packet.

#### 4. Tie behavior is explicit, meaningful, and should be treated as contract, not noise

The live helper uses:

- **first-max** winner selection IS (`378-387`), and
- **average ascending ranks** OOS (`145-167`, `389-390`).

That combination is defensible and it matches the broad shape of the R `pbo` reference implementation (`which.max` plus base `rank()` defaults). But it also means candidate-level outputs are order-sensitive when multiple candidates tie IS. The current smoke test does not pin this. The current third-pass packet did not mention it.

This should not be rewritten casually. It should be documented and covered by a small fixture so the behavior is intentional rather than merely inherited.

#### 5. The shared helper still leaks a composite-specific progress phase label

The helper emits progress with the literal phase name `"composite_validate_cscv"` at start and during the loop (`351`, `412-417`). That is a real shared-helper coupling, not just generic “policy.” The helper already accepts a generic `ProgressSink`, so hardcoding one validation-owner phase name inside a shared CSCV helper remains weak module hygiene.

This is a good safe-now hardening target, but it is still **observability/IPC-sensitive**. If any task center, timeline, or report fixture depends on the phase name, a rename is not zero-risk. The right safe-now move is to thread the phase label from the caller or centralize CSCV progress emission there, not to rename strings casually in place without checking consumers.

#### 6. The score helper silently collapses zero-variance or numerically invalid cases to `0.0`

`sharpe_from_sum_sumsq` returns `0.0` whenever `n < 2`, variance is non-finite, or variance is non-positive (`132-142`). That is robust, but it is not neutral in a statistical sense. A constant positive-return series, a flat series, and a constant negative-return series all collapse to the same score when variance is `<= 0`.

That behavior was underweighted in every earlier packet. I would not call it a proven production bug from the evidence in this bundle, but it is a real methodology edge. Because it directly affects ranking, this should be treated as **document-and-test now, redesign later**. Replacing it with signed infinity, a capped sentinel, or a different performance-statistic abstraction is not a small safe-now tweak.

#### 7. Sampling is deterministic, but still only best-effort on sample count

`sample_k_masks` uses seeded shuffling plus a `HashSet`, with a hard attempt cap of `take * 50` or `10_000`, whichever is larger (`110-129`). The sampled path therefore guarantees determinism for a given seed, but it does **not** guarantee that the returned set size equals `take` if the attempt cap is exhausted. The outer function then accepts the shorter result and reports `combinations = masks.len()` (`308-310`).

That is not catastrophic, and I would not expand scope to rewrite it now, but the third-pass packet under-described it. This is another place where the honest contract is “best-effort deterministic sampling,” not “exactly `max_combinations` sampled paths.”

## Implementation Priority

### Safe-now hardening

The safe-now work should stay narrow and evidence-backed.

The first priority is **public contract documentation**. Add rustdoc to `CscvPboConfig`, `CscvCandidatePboStats`, `CscvPboResult`, and `compute_cscv_pbo_from_slices` explaining at least these points: slices are sanitized to an even `4..=32` range; all candidates must already be provided at the sanitized slice count; the score is currently Sharpe-like and derived from per-slice `sum`/`sumsq`; skipped results are successful returns with `skipped_reason`; skipped paths currently return an empty per-candidate vector; sampled mode is deterministic but best-effort; `max_combinations = 0` is coerced to at least one sampled combination; winner selection uses first-max IS ties; OOS ties use average ascending ranks; exported quantiles are computed by rounded order statistic, not interpolated median.

The second priority is **focused tests that pin today’s semantics**. At minimum, add fixtures for:

- the skipped-path vector shape,
- odd-`N` rank normalization and `lambda <= 0` classification under the **current chosen convention**,
- even-length quantile behavior for `p50`,
- first-max IS tie selection and average-rank OOS ties,
- zero-variance score fallback,
- sampled-path determinism and the invariant that sampled `combinations <= max_combinations`.

The third priority is **progress-label decoupling**, but only in the conservative form. If this helper is touched, pass the phase label down from the caller or move CSCV progress emission outward. Do not silently rename emitted phases without checking task/progress consumers.

### Larger redesign still deferred

Several ideas remain deferred because they would move methodology or public contract semantics.

The biggest deferred item is any **rank-normalization rewrite** from `r / N` to `r / (N + 1)` or any other boundary reinterpretation. The paper and reference implementation do not point to a single unambiguous safe-now answer, and the change would affect validation gates, report fields, and downstream comparisons.

Also deferred is any **quantile-method rewrite**. Switching from rounded order-statistic quantiles to midpoint or interpolated quantiles would alter `lambda_*` and candidate medians. That may be desirable later, but not as a “cleanup.”

Also defer any **zero-variance Sharpe redesign**. A signed-infinity or capped-sentinel approach may be statistically cleaner in some edge cases, but it would alter ranking behavior in a way that needs deliberate policy review.

Finally, defer any **sampling-algorithm rewrite** whose main goal is guaranteeing exact sample count or stronger combinatorial coverage. The current sampled path is deterministic and bounded; a replacement algorithm should only happen if the team explicitly wants a stronger statistical contract.

## Verification If Changed

If only docs and local tests are added, the minimum verification remains the file-local test lane:

- `cargo test -p ta_app_core cscv_pbo -- --nocapture`

If progress emission is touched, also run the validation workflow surfaces that own CSCV/PBO progress reporting and any task/progress fixture coverage that keys off phase names or timeline text.

If any semantics move beyond documentation and tests — especially rank normalization, quantile behavior, skip-path vector shape, or zero-variance score handling — treat that as a cross-surface regression exercise, not a local helper edit. Re-run the owning CSCV/PBO validation coverage for both Double-MA and composite flows, and verify any serialized artifacts or reports that consume:

- `oos_rank_frac_p50`,
- `oos_metric_p50`,
- `lambda_p25`, `lambda_p50`, `lambda_p75`,
- `pbo`,
- `prob_oos_below_threshold`,
- progress/timeline phase labels.

## Regression And Feature-Preservation Constraints

This file has **no meaningful GPU ABI risk** in the current bundle. The real blast radius is elsewhere: validation semantics, serialized DTO shape, and observability labels.

The main constraints are:

- preserve deterministic seeded sampling unless methodology is intentionally changed;
- do not silently change the rank convention, tie convention, quantile convention, or zero-variance score fallback without cross-surface review;
- do not change skipped-path candidate-vector shape unless consumers are audited for IPC/report/schema expectations;
- treat progress-label changes as observability/IPC-sensitive even though the helper is CPU-only;
- preserve current behavior for both Double-MA-family and composite-family validation consumers unless a method change is explicitly approved.

The conservative mindset here is important: the safest fourth-pass work improves **clarity and test coverage around the current semantics**. It does not smuggle in a new statistical contract under the label of refactoring.

## Fourth-Pass Conclusion

`Bucket unchanged`

`crates/app_core/src/cscv_pbo.rs` should remain in `Small surgical refactor`, but the current third-pass packet is too dismissive. It got the scope right — no broad rewrite, no panic-driven tightening, no speculative optimization work — yet it underweighted live public-contract and methodology-edge details that still deserve attention.

The most important fourth-pass correction is this: **the current rank normalization should not be called a clear local bug from the evidence in hand**. Bailey et al. give one formula, the same paper also gives a median-rank definition that is not perfectly aligned for odd `N`, and the CRAN/GitHub `pbo` implementation matches the Rust file much more closely than the first-pass packet suggested. That finding weakens the case for a safe-now normalization rewrite.

At the same time, the third-pass packet missed several real hardening targets that are still worth doing now: document the public semantics, pin the tie/rank/quantile/skip-path behavior with small tests, and clean up the shared-helper progress coupling carefully.

So the fourth-pass recommendation is:

- **safe now:** contract docs, edge-case tests, careful progress-label decoupling;
- **deferred:** normalization rewrites, quantile-method changes, zero-variance score redesign, and stronger sampling guarantees.

That is the smallest change set that improves public-review quality without creating avoidable regressions in a validation-contract-sensitive module.
