# Fourth-Pass Review: crates/strategies/src/double_ma/mod.rs

- Review date: 2026-04-15
- Source file: `crates/strategies/src/double_ma/mod.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Targeted hardening now; larger redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file got a fourth pass because the current third-pass packet was directionally right about the module being foundational and overgrown, but it was too willing to collapse the remaining work into “documentation now, real changes later.” Re-reading the live source against the current `ta_app_core` consumers shows a narrower conclusion.

The big structural conclusion still stands: this is not a good candidate for casual extraction or public-surface tightening. The module still mixes public DTOs, compatibility-sensitive config defaults, curve builders, batch helpers, and multiple evaluation engines in one 4.4k-line file. That is larger redesign territory.

But the live source also shows one bounded, low-risk hardening step that the third-pass packet missed. The f64 halftrend reduced-metric path instantiates `COMPUTE_DRAWDOWN_RISK = false` through the public wrapper surface (`eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter` and `_fast` at lines 2558-2602), yet the underlying implementation still computes drawdown depth and accumulates `ulcer_sum_sq` unconditionally at lines 3174-3190. The plain f64 path gates that work at lines 2821-2838, and the sibling f32 halftrend path also gates it at lines 3954-3974. That makes the current “defer everything except comments/docs” bucket too passive.

My conclusion is that the third-pass bucket should change. The right posture is not a broad refactor now, but it is also not “comments only.” There is safe-now hardening worth doing immediately, while the larger redesign still remains deferred.

## Fourth-Pass Research Log

I re-read the live source first, then the current third-pass packet, then the earlier packets. Within the source file, I focused on four areas: the already-documented `Metrics` contract (lines 39-107), the still-large `StrategyConfig` deserialize/default surface (lines 624-865 and 781-864), the public convenience helpers (`DoubleMaBatchBuilder`, `eval_double_ma_one`, and `eval_double_ma_one_with_types` at lines 2178-2335), and the f64/f32 metric-surface implementations, especially the halftrend variants (lines 2558-2602, 2612-2903, 2954-3257, and 3734-4039).

I then checked the provided downstream context in `crates/app_core/src/double_ma.rs` and `crates/app_core/src/double_ma_validate.rs` to see whether the shared `StrategyConfig` defaults and metric-surface distinctions are merely local conveniences or active downstream contracts. They are active downstream contracts. `ta_app_core` defaults `strategy: StrategyConfig` in the optimization request, drilldown request, local stability request, and validation candidate params (app-core lines 287-288, 464-465, 482-483; validation lines 1485-1486). `ta_app_core` also deliberately chooses reduced versus richer metric surfaces, with `score_mode_needs_rich_metrics` at lines 1295-1296 and `compute_sharpe` selection at lines 11292-11295.

I also checked the live public-deserialization and documentation standards that the current packet leaned on. Serde’s container-attribute docs confirm that `#[serde(default)]` fills missing fields from `Default`, and Serde’s field-attribute docs confirm that per-field defaults are part of the active deserialize contract. That supports the third-pass warning that parse tightening is not a local cleanup. The Rust API Guidelines documentation guidance also supports documenting public helper behavior and non-obvious caveats, which matters more here for the convenience execution helpers than for `Metrics`, because `Metrics` already has substantial field-level documentation in the live source.

Finally, I re-checked the local tests. The current file has seven `#[test]`s, concentrated at lines 4255-4422. They cover some cost, TP/SL, and grid sanity, but they do not provide “strong coverage” for the breadth of this module.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep the large module split deferred. `StrategyConfig` is still a compatibility-sensitive deserialize surface, and it is embedded by default in multiple app-core request types. Broad changes to unknown-field handling or default semantics would not stay local to `ta_strategies`; they would propagate into optimize, drilldown, local-stability, and validation entrypoints.

The packet was also right to be cautious about hot-path cleanup. The evaluator family remains duplicated across f64/f32 and filtered/unfiltered variants, and that duplication is ugly, but it is also where CPU semantics anchor downstream optimizer behavior. A mechanical extraction here still carries real parity and performance risk.

### What the current third-pass packet missed or underweighted

The biggest miss is the live inconsistency in the f64 halftrend reduced-metric path. The public wrappers at lines 2558-2602 request `COMPUTE_DRAWDOWN_RISK = false` for the non-rich surfaces, but the implementation at lines 3174-3190 still computes percentage drawdown and accumulates `ulcer_sum_sq`. The plain f64 implementation does not do that when reduced metrics are requested (lines 2821-2838), and the sibling f32 halftrend implementation is already consistent (lines 3954-3974). This is a narrow fix, not a redesign.

The packet also underweighted the fact that the live source has already moved some of the earlier “docs first” concerns. `Metrics` is no longer mostly undocumented. The field block at lines 39-107 now carries meaningful semantics and backward-compatible `#[serde(default)]` annotations for the widened metrics surface. The sharper documentation gap is not `Metrics`; it is the convenience execution surface.

That convenience surface is narrower and more implicit than the packet acknowledged. `DoubleMaBatchBuilder::run_cpu` always uses `"close"` and offers no `StrategyConfig` injection (lines 2221-2229). `eval_double_ma_one` hard-codes only three numeric MA ids, mapping `0/1/2` to `sma/ema/alma` (lines 2260-2270), while `eval_double_ma_one_with_types` is the general path keyed by caller-provided MA type lists (lines 2316-2322). Those public helpers are convenience APIs, not the same contract breadth that the rest of the repo uses. That should be documented explicitly before anyone “cleans them up.”

### What the current packet overstated

The strongest overstatement is the inherited “strong test coverage” framing from the earlier packets. The file has only seven local tests (lines 4255-4422). None of them cover halftrend reduced-metric behavior, the convenience-helper contract differences, or fast/rich surface invariants. For a 4.4k-line foundational module, that is targeted coverage, not strong coverage.

The other overstatement is priority. The third-pass packet made unknown-field tolerance sound like the sharpest near-term concern. It is a real compatibility topic, but not the best immediate change candidate. Because app-core defaults `StrategyConfig` into multiple request types, adding `#[serde(deny_unknown_fields)]` here would be a user-visible parse-policy change across several flows, not a local hardening tweak. Serde tightening belongs in a coordinated migration or request-boundary validation pass, not as an isolated edit to this shared struct.

## Implementation Priority

### Safe-now hardening

The first priority is to fix the f64 halftrend reduced-metric inconsistency. Specifically, gate the `dd` / `ulcer_sum_sq` accumulation in `eval_double_ma_from_log_returns_and_mas_f64_with_halftrend_filter_impl` the same way the plain f64 and f32 halftrend implementations already do. This should be behavior-preserving for outputs because `finalize_risk_metrics` already receives `COMPUTE_DRAWDOWN_RISK`, but it removes unnecessary work from the reduced surface that app-core intentionally selects.

The second priority is bounded public-contract clarification, not signature churn. Add rustdoc to `DoubleMaBatchBuilder`, `run_cpu`, `eval_double_ma_one`, and `eval_double_ma_one_with_types` stating that the builder is a close-price/default-strategy convenience path, and that `eval_double_ma_one` uses the legacy fixed MA-id mapping while `eval_double_ma_one_with_types` is the caller-supplied generic path. The safe change here is clarification; do not change signatures, visibility, or return types in this pass.

The third priority is to strengthen the tests around the area the current packet missed. I would add focused coverage for halftrend reduced-surface selection and for the convenience-helper contract. Even if a unit test will not directly catch wasted work, it can at least pin the public wrapper choices and the expected surface fields so a future refactor does not widen the reduced path accidentally.

### Larger redesign still deferred

The full module split remains deferred. The evaluator family is still duplicated across curve generation and metrics generation, across filtered and unfiltered paths, and across f64/f32 variants. There is obvious cleanup value here, but this is still the wrong file for a casual decomposition.

The deserialize-policy question also remains deferred. If the repo wants strict unknown-field rejection for `StrategyConfig`, that should happen as a coordinated request-boundary and fixture migration across optimize, analysis, validation, and stored artifact re-load paths. It should not be done by dropping `deny_unknown_fields` onto the shared struct and hoping the fallout is small.

Finally, the public helper surface deserves a design pass later. The current mix of numeric MA ids, caller-supplied type vectors, fixed `"close"` sourcing, and default-strategy convenience methods is serviceable, but not especially elegant. That is an API-shaping task, not a fourth-pass hardening edit.

## Verification If Changed

For the safe-now patch, I would treat verification as both correctness and contract-preservation work.

Start with crate-local coverage:

- `cargo test -p ta_strategies -- --nocapture`
- `cargo test -p ta_optimizer -- --nocapture`
- `cargo test -p ta_app_core double_ma -- --nocapture`
- `cargo test -p ta_app_core double_ma_validate -- --nocapture`

Because the live miss is performance/consistency oriented rather than output-visible, I would also run a targeted workflow or perf smoke after the f64 halftrend gate fix. At minimum, use the repo quality gate for broad safety. If the patch is kept purely local but touches the evaluator hot path, a candidate CPU perf gate is justified by the repository guidance.

If the helper rustdocs are added, no extra compatibility testing is needed beyond the crate and downstream tests above. If anyone proposes more than docs there — for example a `Result` return, visibility reduction, or deprecation path — that should be reclassified as coordinated API work and verified separately.

## Regression And Feature-Preservation Constraints

Preserve current `StrategyConfig` default and omission behavior in this pass. The shared struct is default-injected downstream, so parse-policy tightening is broader than this file.

Do not change the serialized field names or backward-compatible `Metrics` shape. The widened metrics surface is already consumed downstream, and the live source now documents those fields reasonably well.

Do not change the public signatures of `DoubleMaBatchBuilder`, `eval_double_ma_one`, or `eval_double_ma_one_with_types` in this pass. Their current behavior is narrow and somewhat implicit, but signature changes would be public-contract work, not hardening.

Do not use this patch as a reason to extract common evaluator helpers aggressively. The duplicated state machines are unattractive, but they sit in hot and semantics-sensitive code. Extraction should come only with dedicated parity and performance validation.

If the f64 halftrend gating fix is made, keep it strictly behavior-preserving. The change should remove unnecessary work only; it should not alter reported metrics, trade counts, or default field population for the reduced or rich surfaces.

## Fourth-Pass Conclusion

The current third-pass bucket should change.

What the third-pass packet got right is still important: the module is foundational, the big split is still deferred, and `StrategyConfig` deserialize tightening is not a safe local cleanup. But the packet was too passive because it missed a live, bounded inconsistency in the f64 halftrend reduced-metric path and it overstated the current local test confidence.

My fourth-pass judgment is therefore: take a small hardening patch now, then stop. Fix the missing drawdown gate in the f64 halftrend reduced surface, clarify the public convenience-helper contracts, and add focused tests around those areas. Keep the larger redesign — module extraction, API reshaping, and deserialize-policy tightening — explicitly deferred.
