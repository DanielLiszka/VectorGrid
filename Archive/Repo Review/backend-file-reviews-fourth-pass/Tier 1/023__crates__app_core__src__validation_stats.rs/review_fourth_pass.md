# Fourth-Pass Review: `crates/app_core/src/validation_stats.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/validation_stats.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Document only`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file got a fourth pass because the third-pass packet changed the action bucket from `Document only` to `Small surgical refactor` on a very small helper module whose behavior is reused deep in validation and confidence-shortlist flows. That bucket change needed an adversarial re-check against the live source, the actual validation caller, and the nearby confidence-ranking consumers before treating the contract diagnosis as established.

After re-reading the live file, the adjacent validation pipeline, and the provided downstream snippets, the third-pass bucket increase does not hold up as written. The file is still semantics-sensitive, but the strongest evidence continues to support documentation hardening and narrower test pinning rather than a behavior change in this module.

## Fourth-Pass Research Log

The fourth pass reviewed the live helper surface in `validation_stats.rs`, especially `sanitize_probability_gate`, `compute_psr_dsr`, and `passes_probability_gates`, along with the file-local tests. It then checked the actual validation path in `validation_pipeline.rs`, including `sanitize_hard_gates`, `compute_probability_metrics`, and `apply_candidate_hard_gates`, because those functions determine whether this helper is truly the authority for live admission behavior.

The fourth pass also re-checked the provided `double_ma.rs` and `gpu_backtest_kernel.rs` snippets, because `compute_psr_dsr` is used outside validation to rerank confidence-based shortlists. That matters: a supposedly small refactor in this file would not stay local to validation verdicts.

For external grounding, the pass re-checked the Rust `Option` documentation, the Rust API Guidelines documentation and dependability guidance, and the Bailey / López de Prado PSR / DSR references. Those sources support two narrow conclusions only: `Option` should distinguish presence from absence, public semantics should be documented clearly, and PSR / DSR are probability-style statistics whose methodology and assumptions should be made explicit. They do not, by themselves, prove that this file is manufacturing bogus finite probabilities.

## What Fourth Pass Sharpened

### Bucket assessment

The third-pass bucket should **change back** to `Document only`.

The central third-pass claim is that this file "wraps sentinel-style probabilities into `Some` values" and therefore has a contract bug. The live source does not actually establish that. `compute_psr_dsr` computes PSR and DSR through `sharpe_stats`, returns `None` only when the underlying result is non-finite, and otherwise clamps the finite output into `[0.0, 1.0]` (`validation_stats.rs:17-33`). That is a thin adapter over a probability-shaped metric, not evidence that finite outputs are fabricated. The literature being cited for PSR / DSR explicitly describes them as probability-based statistics, and DSR is expressed through the standard Normal cumulative function, so a finite bounded output is the expected shape, not inherently suspicious.

If the real concern is that `sharpe_stats` itself may emit finite sentinel values for underspecified inputs, that concern belongs first in `sharpe_stats.rs`, not here. The provided bundle does not include that file, so the third-pass packet overreaches by treating the missing internal proof as already settled.

### What the third-pass packet got right

The third-pass packet was right about three things.

First, the file is contract-sensitive despite being small. The assumptions baked into `compute_psr_dsr` are not just arithmetic plumbing; they are policy choices.

Second, the current rustdoc surface is too thin. The file still leaves its assumptions, caller expectations, and threshold semantics implicit.

Third, a casual behavioral change here would be high-risk. This helper sits near approval logic, and the surrounding validation layer is not the kind of place where undocumented semantic drift is acceptable.

### What the third-pass packet missed or underweighted

The most important miss is that `passes_probability_gates` is **not** the canonical live hard-gate implementation in the provided validation flow. `validation_pipeline::apply_candidate_hard_gates` performs the real approval checks inline with `cand.psr().unwrap_or(0.0)` and `cand.dsr().unwrap_or(0.0)` rather than routing through `passes_probability_gates` (`validation_pipeline.rs:359-386`). That means a refactor of `passes_probability_gates` alone would not harden the live validation path. The third-pass packet treated the helper as if it were the operative gate authority, but the local context shows the real authority is elsewhere.

The third-pass packet also underweighted that `compute_psr_dsr` is used for more than validation approval. The provided `double_ma.rs` and `gpu_backtest_kernel.rs` snippets show the same helper feeding confidence-based shortlist reranking. A more aggressive `None` policy would therefore change optimization shortlist ordering and GPU/host parity expectations, not just validation verdicts. That is a broader regression surface than the third-pass packet acknowledged.

It also missed the breadth of `sanitize_probability_gate`. In the live validation pipeline, that helper is reused not only for `min_psr` and `min_dsr`, but also for `min_cscv_oos_rank_frac` and `max_pbo` (`validation_pipeline.rs:244-265`). So even a “small” semantic change here would spill into CSCV/PBO threshold handling.

Finally, the earlier packets, not only the third pass, were slightly too strong in describing missing-stat behavior. The live code does **not** literally say “missing stats fail whenever a threshold is present.” It says missing stats are compared as `0.0` (`validation_stats.rs:36-44`). Those are different statements. With a positive threshold, the effect is failure. With a `0.0` threshold, or a negative threshold that was clamped to `0.0`, the missing stat passes. That edge is currently untested in this file and should be documented more precisely.

### What the third-pass packet overstated

The sentence that this file “mostly manufactures bounded values rather than surfacing real absence” is too strong for the evidence in the bundle.

The live file only does two things after delegating to `sharpe_stats`: it maps non-finite results to `None`, and it bounds finite results into `[0.0, 1.0]`. Without direct evidence from `sharpe_stats.rs` that finite sentinel values are being emitted, the stronger diagnosis is not established. The third-pass recommendation therefore risks sending an implementer to the wrong layer and encouraging a behavior change with wider downstream fallout than the packet admits.

### A narrower issue the current packet still missed

There is one nearby issue that is more concrete than the third-pass Option diagnosis, but it is outside this file.

`validation_pipeline::compute_probability_metrics` builds `sharpe_peers` from all candidate sharpe values, then counts only finite values for `trials` (`validation_pipeline.rs:272-282`). The provided shortlist snippets in `double_ma.rs` and `gpu_backtest_kernel.rs` instead filter non-finite sharpes before calling the shared `mean_std` helper. If `sharpe_stats::mean_std` does not already self-sanitize, that is a real caller-consistency risk. If it does self-sanitize, then the third-pass claim about this file manufacturing questionable `Some` values weakens even further. Either way, this is not a reason to refactor `validation_stats.rs` first.

## Implementation Priority

### Safe-now hardening

The safe-now work is still documentation-first, with one optional test tightening.

1. Add rustdoc to `sanitize_probability_gate` that explicitly states it accepts only finite values, clamps them into `[0.0, 1.0]`, and is reused for several bounded validation thresholds, not just PSR / DSR.

2. Add rustdoc to `compute_psr_dsr` that documents the current statistical assumptions being passed through (`reference_sharpe = 0.0`, `skew = 0.0`, `kurtosis = 3.0`), and that the helper expects caller-side shaping for observation count and peer summary inputs.

3. Add rustdoc to `passes_probability_gates` clarifying the exact fallback semantics: missing PSR / DSR is treated as `0.0` for comparison purposes, which usually fails positive thresholds but does not force failure when the threshold is `0.0`.

4. If the file is touched anyway, add a narrow regression test for the `0.0` threshold edge so future packets do not overstate the policy again. That is the one test hole the current file still leaves open.

These are small, low-risk hardening steps. They preserve behavior, make the public contract clearer, and avoid implying that this file should now become the place where deeper validation-policy redesign happens.

### Larger redesign still deferred

A real redesign, if the team wants one, belongs outside this file and should stay deferred.

The first deferred path is a richer result contract for PSR / DSR availability. If the goal is to distinguish “computed probability,” “insufficient statistical support,” and “invalid numeric inputs,” that should be designed around `sharpe_stats.rs` and the live validation / shortlist consumers together. A local `Option` refactor here would be incomplete.

The second deferred path is unifying peer-stat preparation across callers. The current bundle already suggests that validation and shortlist code prepare sharpe peer sets differently before calling shared summary helpers. That is a cross-layer consistency question, not a `validation_stats.rs` refactor.

The third deferred path is stronger input validation or type-level enforcement for bounded probability thresholds and observation counts. Rust’s dependability guidance supports that direction in principle, but it is broader API work and not justified as a quick fix in this helper module.

## Verification If Changed

If only rustdoc and a narrow regression test are added, the verification surface stays small:

- `cargo test -p ta_app_core validation_stats::tests:: -- --nocapture`
- `cargo test -p ta_app_core validation_pipeline::tests:: -- --nocapture`

If anyone ignores this packet and changes `compute_psr_dsr` semantics anyway, verification has to widen immediately to the confidence-shortlist paths as well, because those flows also consume this helper. At that point, re-run the representative `double_ma.rs` and `gpu_backtest_kernel.rs` confidence / reranking tests before accepting the change.

No GPU ABI, IPC, or persistence-specific verification is needed for the safe-now documentation path. Those risk categories only become relevant if behavior changes propagate into saved validation artifacts, shortlist composition, or result ordering.

## Regression And Feature-Preservation Constraints

Preserve the current finite-to-bounded mapping in `compute_psr_dsr` unless `sharpe_stats.rs` is reviewed at the same time and proves that a different absence contract is required.

Preserve the current “compare missing as `0.0`” behavior unless all live gate call sites are updated together. In the provided bundle, the authoritative hard-gate logic is duplicated in `validation_pipeline::apply_candidate_hard_gates`, so changing only `passes_probability_gates` would create semantic drift instead of fixing it.

Do not casually change `sanitize_probability_gate` because its current normalization behavior is reused for multiple bounded validation thresholds, including CSCV rank fraction and PBO ceilings.

Do not assume this file is validation-only. `compute_psr_dsr` also feeds confidence-based shortlist reranking, so availability or clamping changes can alter optimization outputs and CPU / GPU comparison expectations.

If future work targets caller consistency, inspect `sharpe_stats::mean_std` first. The current bundle does not prove whether the raw-vs-filtered peer preparation difference is benign or user-visible.

## Fourth-Pass Conclusion

The third-pass packet should **not** stand as written. Its bucket should change back from `Small surgical refactor` to `Document only`.

What the third pass got right is that this is a small but semantics-sensitive helper whose assumptions need to be made explicit. What it got wrong is the core escalation rationale: the provided evidence does not show that `validation_stats.rs` is manufacturing bogus `Some` values, and the packet overstates how much hardening would actually happen by changing this file alone.

The safest and most accurate fourth-pass call is therefore: keep behavior, document the contract precisely, add one narrow edge-case test if the file is touched, and defer any real availability-model redesign to a coordinated review of `sharpe_stats.rs`, `validation_pipeline.rs`, and the confidence-shortlist consumers.
